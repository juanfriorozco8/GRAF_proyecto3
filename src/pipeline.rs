use crate::geometry::{Basis, Mesh, Vec3};
use crate::shaders::{self, Fragment, Material, Uniforms};

const NEAR: f32 = 0.16;
pub const FAR: f32 = 520.0;
const SUBPIXEL: i64 = 64;

#[derive(Clone, Copy)]
pub struct Camera {
    pub eye: Vec3,
    pub basis: Basis,
}

impl Camera {
    pub fn look(eye: Vec3, target: Vec3) -> Self {
        Self {
            eye,
            basis: Basis::looking(target - eye, Vec3::new(0.0, 1.0, 0.0)),
        }
    }

    fn eye_space(self, world: Vec3) -> Vec3 {
        let view = self.basis.local(world - self.eye);
        Vec3::new(view.x, view.y, -view.z)
    }
}

#[derive(Clone, Copy)]
pub struct Model {
    pub position: Vec3,
    pub basis: Basis,
    pub radius: f32,
}

#[derive(Clone, Copy)]
pub enum Surface {
    Planet(Material),
    Ship,
}

#[derive(Default)]
pub struct RenderStats {
    pub triangles: usize,
    pub culled: usize,
}

pub struct Frame {
    pub width: usize,
    pub height: usize,
    rgba: Vec<u8>,
    depth: Vec<f32>,
    vertices: Vec<Vertex>,
    calls: Vec<DrawCall>,
    triangles: Vec<Triangle>,
    workers: usize,
    focal: f32,
    pub stats: RenderStats,
}

#[derive(Clone, Copy)]
struct Vertex {
    eye: Vec3,
    local: Vec3,
}

#[derive(Clone, Copy)]
struct Screen {
    x: i64,
    y: i64,
    inverse_z: f32,
    local: Vec3,
}

struct DrawCall {
    surface: Surface,
    eye_local: Vec3,
    light: Vec3,
    uniforms: Uniforms,
}

struct Triangle {
    vertices: [Screen; 3],
    call: usize,
    normal: Vec3,
    glass: bool,
    x: std::ops::Range<usize>,
    y: std::ops::Range<usize>,
    inverse_area: f32,
}

struct Band<'a> {
    rgba: &'a mut [u8],
    depth: &'a mut [f32],
    width: usize,
    first_row: usize,
    last_row: usize,
}

fn edge(a: Screen, b: Screen, x: i64, y: i64) -> i64 {
    (b.x - a.x) * (y - a.y) - (b.y - a.y) * (x - a.x)
}

impl Frame {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            rgba: vec![0; width * height * 4],
            depth: vec![0.0; width * height],
            vertices: Vec::new(),
            calls: Vec::new(),
            triangles: Vec::new(),
            workers: std::thread::available_parallelism()
                .map_or(4, |n| n.get())
                .clamp(1, 8),
            focal: height as f32 * 0.5 / 29.0_f32.to_radians().tan(),
            stats: RenderStats::default(),
        }
    }

    pub fn clear(&mut self) {
        self.stats = RenderStats::default();
        self.calls.clear();
        self.triangles.clear();
        self.depth.fill(0.0);
        for pixel in self.rgba.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[2, 3, 8, 255]);
        }
    }

    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    pub fn blend(&mut self, x: i32, y: i32, color: [u8; 4]) {
        if x < 0 || x >= self.width as i32 || y < 0 || y >= self.height as i32 {
            return;
        }
        let offset = (y as usize * self.width + x as usize) * 4;
        let alpha = u16::from(color[3]);
        for (dst, src) in self.rgba[offset..offset + 3].iter_mut().zip(color) {
            *dst = ((u16::from(*dst) * (255 - alpha) + u16::from(src) * alpha + 127) / 255) as u8;
        }
    }

    pub fn line(&mut self, a: (i32, i32), b: (i32, i32), color: [u8; 4]) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let steps = dx.abs().max(dy.abs()).max(1);
        for step in 0..=steps {
            self.blend(a.0 + dx * step / steps, a.1 + dy * step / steps, color);
        }
    }

    fn project_eye(&self, eye: Vec3) -> (f32, f32) {
        let scale = self.focal / eye.z;
        (
            self.width as f32 * 0.5 + eye.x * scale,
            self.height as f32 * 0.5 - eye.y * scale,
        )
    }

    fn visible_point(&self, eye: Vec3) -> Option<(i32, i32)> {
        if eye.z <= NEAR {
            return None;
        }
        let (x, y) = self.project_eye(eye);
        (x >= 0.0 && x < self.width as f32 && y >= 0.0 && y < self.height as f32)
            .then_some((x as i32, y as i32))
    }

    pub fn project_world(&self, camera: Camera, world: Vec3) -> Option<(i32, i32)> {
        self.visible_point(camera.eye_space(world))
    }

    pub fn project_direction(&self, camera: Camera, direction: Vec3) -> Option<(i32, i32)> {
        let eye = camera.basis.local(direction);
        self.visible_point(Vec3::new(eye.x, eye.y, -eye.z))
    }

    pub fn visible_sphere(&self, camera: Camera, model: Model) -> bool {
        let eye = camera.eye_space(model.position);
        let side = self.width as f32 / (2.0 * self.focal);
        let top = self.height as f32 / (2.0 * self.focal);
        eye.z + model.radius >= NEAR
            && eye.z - model.radius <= FAR
            && eye.x.abs() - eye.z * side <= model.radius * (1.0 + side * side).sqrt()
            && eye.y.abs() - eye.z * top <= model.radius * (1.0 + top * top).sqrt()
    }

    pub fn projected_radius(&self, camera: Camera, model: Model) -> f32 {
        self.focal * model.radius / camera.eye_space(model.position).z.max(NEAR)
    }

    pub fn draw(
        &mut self,
        camera: Camera,
        model: Model,
        mesh: &Mesh,
        surface: Surface,
        time: f32,
        world_light: Vec3,
    ) {
        let eye_local = model.basis.local(camera.eye - model.position) * model.radius.recip();
        let light = model.basis.local(world_light);
        let uniforms = Uniforms::new(time, light, 0.43, true);
        let call = self.calls.len();
        self.calls.push(DrawCall {
            surface,
            eye_local,
            light,
            uniforms,
        });
        self.vertices.clear();
        self.vertices
            .extend(mesh.vertices.iter().map(|&local| Vertex {
                eye: camera.eye_space(model.position + model.basis.world(local * model.radius)),
                local,
            }));
        self.stats.triangles += mesh.faces.len();

        for face in &mesh.faces {
            let triangle = face.indices.map(|index| self.vertices[index]);
            let normal = match surface {
                Surface::Planet(_) => Vec3::zero(),
                Surface::Ship => (triangle[1].local - triangle[0].local)
                    .cross(triangle[2].local - triangle[0].local)
                    .normalized(),
            };
            let (polygon, length) = clip_near(triangle);
            for index in 1..length.saturating_sub(1) {
                let screen = [polygon[0], polygon[index], polygon[index + 1]].map(|v| {
                    let (x, y) = self.project_eye(v.eye);
                    Screen {
                        x: (x * SUBPIXEL as f32).round() as i64,
                        y: (y * SUBPIXEL as f32).round() as i64,
                        inverse_z: v.eye.z.recip(),
                        local: v.local,
                    }
                });
                self.submit(screen, call, normal, face.glass);
            }
        }
    }

    fn submit(
        &mut self,
        vertices @ [a, b, c]: [Screen; 3],
        call: usize,
        normal: Vec3,
        glass: bool,
    ) {
        let area = edge(a, b, c.x, c.y);
        if area >= 0 {
            return;
        }
        let floor = |value: i64| (value.max(0) / SUBPIXEL) as usize;
        let ceil = |value: i64| ((value.max(0) + SUBPIXEL - 1) / SUBPIXEL) as usize;
        let x0 = floor(a.x.min(b.x).min(c.x));
        let y0 = floor(a.y.min(b.y).min(c.y));
        let x1 = ceil(a.x.max(b.x).max(c.x)).min(self.width);
        let y1 = ceil(a.y.max(b.y).max(c.y)).min(self.height);
        if x0 < x1 && y0 < y1 {
            self.triangles.push(Triangle {
                vertices,
                call,
                normal,
                glass,
                x: x0..x1,
                y: y0..y1,
                inverse_area: (area as f32).recip(),
            });
        }
    }

    pub fn rasterize(&mut self) {
        let width = self.width;
        let rows = self.height.div_ceil(self.workers);
        let triangles = &self.triangles;
        let calls = &self.calls;
        std::thread::scope(|scope| {
            let colors = self.rgba.chunks_mut(rows * width * 4);
            let depths = self.depth.chunks_mut(rows * width);
            for (index, (rgba, depth)) in colors.zip(depths).enumerate() {
                let first_row = index * rows;
                let mut band = Band {
                    rgba,
                    last_row: first_row + depth.len() / width,
                    depth,
                    width,
                    first_row,
                };
                scope.spawn(move || {
                    for triangle in triangles {
                        let y0 = triangle.y.start.max(band.first_row);
                        let y1 = triangle.y.end.min(band.last_row);
                        if y0 < y1 {
                            band.draw(triangle, y0..y1, &calls[triangle.call]);
                        }
                    }
                });
            }
        });
    }
}

impl Band<'_> {
    fn draw(&mut self, triangle: &Triangle, rows: std::ops::Range<usize>, call: &DrawCall) {
        let [a, b, c] = triangle.vertices;
        let inverse_area = triangle.inverse_area;
        let [du, dv, dw] = [b.y - c.y, c.y - a.y, a.y - b.y].map(|slope| slope * SUBPIXEL);
        let px = triangle.x.start as i64 * SUBPIXEL + SUBPIXEL / 2;
        for y in rows {
            let py = y as i64 * SUBPIXEL + SUBPIXEL / 2;
            let mut edge_u = edge(b, c, px, py);
            let mut edge_v = edge(c, a, px, py);
            let mut edge_w = edge(a, b, px, py);
            let row = (y - self.first_row) * self.width;
            for x in triangle.x.clone() {
                let (u, v, w) = (edge_u, edge_v, edge_w);
                edge_u += du;
                edge_v += dv;
                edge_w += dw;
                if u > 0 || v > 0 || w > 0 {
                    continue;
                }
                let u = u as f32 * inverse_area * a.inverse_z;
                let v = v as f32 * inverse_area * b.inverse_z;
                let w = w as f32 * inverse_area * c.inverse_z;
                let inverse_z = u + v + w;
                let pixel = row + x;
                if inverse_z <= self.depth[pixel] {
                    continue;
                }
                let local = (a.local * u + b.local * v + c.local * w) * inverse_z.recip();
                let view = (call.eye_local - local).normalized();
                let color = match call.surface {
                    Surface::Planet(material) => material.shade(
                        Fragment {
                            normal: local.normalized(),
                            view,
                        },
                        &call.uniforms,
                    ),
                    Surface::Ship => shaders::ship(
                        Fragment {
                            normal: triangle.normal,
                            view,
                        },
                        call.light,
                        triangle.glass,
                    ),
                };
                self.depth[pixel] = inverse_z;
                self.rgba[pixel * 4..pixel * 4 + 4].copy_from_slice(&color);
            }
        }
    }
}

fn clip_near(triangle: [Vertex; 3]) -> ([Vertex; 4], usize) {
    if triangle.iter().all(|vertex| vertex.eye.z >= NEAR) {
        return ([triangle[0], triangle[1], triangle[2], triangle[0]], 3);
    }
    let mut polygon = [triangle[0]; 4];
    let mut count = 0;
    let mut previous = triangle[2];
    for current in triangle {
        let inside = current.eye.z >= NEAR;
        let previous_inside = previous.eye.z >= NEAR;
        if inside != previous_inside {
            let t = (NEAR - previous.eye.z) / (current.eye.z - previous.eye.z);
            let mut eye = previous.eye + (current.eye - previous.eye) * t;
            eye.z = NEAR;
            polygon[count] = Vertex {
                eye,
                local: previous.local + (current.local - previous.local) * t,
            };
            count += 1;
        }
        if inside {
            polygon[count] = current;
            count += 1;
        }
        previous = current;
    }
    (polygon, count)
}
