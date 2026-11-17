use std::collections::HashMap;

pub use raylib::prelude::Vector3 as Vec3;

#[derive(Clone, Copy)]
pub struct Basis {
    pub right: Vec3,
    pub up: Vec3,
    pub back: Vec3,
}

impl Basis {
    pub fn looking(forward: Vec3, reference_up: Vec3) -> Self {
        let back = -forward.normalized();
        let right = reference_up.cross(back).normalized();
        Self {
            right,
            up: back.cross(right),
            back,
        }
    }

    pub fn banked(mut self, roll: f32) -> Self {
        let (sin, cos) = roll.sin_cos();
        let right = self.right;
        self.right = right * cos + self.up * sin;
        self.up = self.up * cos - right * sin;
        self
    }

    pub fn yaw(angle: f32) -> Self {
        let (sin, cos) = angle.sin_cos();
        Self {
            right: Vec3::new(cos, 0.0, -sin),
            up: Vec3::new(0.0, 1.0, 0.0),
            back: Vec3::new(sin, 0.0, cos),
        }
    }

    pub fn world(self, vector: Vec3) -> Vec3 {
        self.right * vector.x + self.up * vector.y + self.back * vector.z
    }

    pub fn local(self, vector: Vec3) -> Vec3 {
        Vec3::new(
            self.right.dot(vector),
            self.up.dot(vector),
            self.back.dot(vector),
        )
    }
}

#[derive(Clone, Copy)]
pub struct Face {
    pub indices: [usize; 3],
    pub glass: bool,
}

pub struct Mesh {
    pub vertices: Vec<Vec3>,
    pub faces: Vec<Face>,
}

impl Mesh {
    pub fn obj(source: &str) -> Result<Self, String> {
        let mut vertices = Vec::new();
        let mut faces = Vec::new();
        let mut glass = false;
        for (line_index, line) in source.lines().enumerate() {
            let invalid = || format!("OBJ inválido en línea {}", line_index + 1);
            let mut fields = line.split_whitespace();
            match fields.next() {
                Some("v") => {
                    let mut xyz = [0.0_f32; 3];
                    for coordinate in &mut xyz {
                        *coordinate = fields
                            .next()
                            .and_then(|n| n.parse().ok())
                            .filter(|n: &f32| n.is_finite())
                            .ok_or_else(invalid)?;
                    }
                    vertices.push(Vec3::new(xyz[0], xyz[1], xyz[2]));
                }
                Some("usemtl") => glass = fields.next() == Some("Parabrisas"),
                Some("f") => {
                    let mut indices = [0; 3];
                    for index in &mut indices {
                        *index = fields
                            .next()
                            .and_then(|n| n.split('/').next())
                            .and_then(|n| n.parse::<usize>().ok())
                            .and_then(|n| n.checked_sub(1))
                            .filter(|&n| n < vertices.len())
                            .ok_or_else(invalid)?;
                    }
                    if fields.next().is_some() {
                        return Err(invalid());
                    }
                    faces.push(Face { indices, glass });
                }
                _ => {}
            }
        }
        if faces.is_empty() {
            return Err("OBJ sin caras trianguladas".into());
        }
        Ok(Self { vertices, faces })
    }

    pub fn normalize_sphere(&mut self) {
        for vertex in &mut self.vertices {
            *vertex = vertex.normalized();
        }
    }

    pub fn subdivide_sphere(&self) -> Self {
        let mut vertices = self.vertices.clone();
        let mut edges = HashMap::new();
        let mut midpoint = |a: usize, b: usize| {
            *edges.entry((a.min(b), a.max(b))).or_insert_with(|| {
                let index = vertices.len();
                vertices.push((vertices[a] + vertices[b]).normalized());
                index
            })
        };
        let mut faces = Vec::with_capacity(self.faces.len() * 4);
        for face in &self.faces {
            let [a, b, c] = face.indices;
            let ab = midpoint(a, b);
            let bc = midpoint(b, c);
            let ca = midpoint(c, a);
            faces.extend(
                [[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]].map(|indices| Face {
                    indices,
                    glass: false,
                }),
            );
        }
        Self { vertices, faces }
    }

    pub fn fit_ship(&mut self) {
        let anchor = Vec3::new(0.0, 1.45, 0.1);
        let size = self
            .vertices
            .iter()
            .map(|&p| (p - anchor).length())
            .fold(0.0_f32, f32::max);
        for vertex in &mut self.vertices {
            // El parabrisas del OBJ mira a +Z y nuestro eje delantero es -Z.
            let local = (*vertex - anchor) * size.recip();
            *vertex = Vec3::new(-local.x, local.y, -local.z);
        }
    }
}
