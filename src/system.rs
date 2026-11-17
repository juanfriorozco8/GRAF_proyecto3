use std::f32::consts::TAU;

use crate::geometry::{Basis, Mesh, Vec3};
use crate::pipeline::{Camera, Frame, Model, Surface, FAR};
use crate::shaders::Material;

pub const STAR_COUNT: usize = 2;
pub const PLANET_COUNT: usize = 8;
pub const BODY_COUNT: usize = STAR_COUNT + PLANET_COUNT;
pub const SCARIF: usize = 4;
pub const OUTER_ORBIT: f32 = 155.0;
const START: Vec3 = Vec3 {
    x: -44.0,
    y: 88.0,
    z: 196.0,
};
pub const TOUR_LENGTH: f32 = 72.0;
const SHIP_RADIUS: f32 = 0.34;
const CAMERA_RADIUS: f32 = 0.30;
const LOCAL_RANGE: f32 = 12.0;
const UP: Vec3 = Vec3 {
    x: 0.0,
    y: 1.0,
    z: 0.0,
};

pub struct Body {
    pub name: &'static str,
    pub color: [u8; 4],
    pub radius: f32,
    pub orbit: f32,
    pub phase: f32,
    pub speed: f32,
    pub spin: f32,
    material: Material,
}

pub const BODIES: [Body; BODY_COUNT] = [
    Body {
        name: "TATOO I",
        color: [252, 188, 91, 255],
        radius: 12.1,
        orbit: 6.6,
        phase: 0.38,
        speed: 0.085,
        spin: 0.1,
        material: Material::TatooI,
    },
    Body {
        name: "TATOO II",
        color: [248, 116, 75, 255],
        radius: 8.6,
        orbit: 16.0,
        phase: 3.5215927,
        speed: 0.085,
        spin: -0.15,
        material: Material::TatooII,
    },
    Body {
        name: "TATOOINE",
        color: [224, 184, 119, 255],
        radius: 3.1,
        orbit: 29.5,
        phase: 2.76,
        speed: 0.104,
        spin: 0.16,
        material: Material::Tatooine,
    },
    Body {
        name: "MUSTAFAR",
        color: [234, 101, 66, 255],
        radius: 2.8,
        orbit: 38.0,
        phase: 1.58,
        speed: 0.087,
        spin: 0.22,
        material: Material::Mustafar,
    },
    Body {
        name: "SCARIF",
        color: [82, 197, 222, 255],
        radius: 4.8,
        orbit: 52.0,
        phase: 0.12,
        speed: 0.07,
        spin: 0.15,
        material: Material::Scarif,
    },
    Body {
        name: "NABOO",
        color: [111, 180, 147, 255],
        radius: 4.2,
        orbit: 69.0,
        phase: 3.57,
        speed: 0.058,
        spin: -0.12,
        material: Material::Naboo,
    },
    Body {
        name: "KASHYYYK",
        color: [133, 179, 96, 255],
        radius: 4.6,
        orbit: 86.0,
        phase: 2.21,
        speed: 0.048,
        spin: 0.17,
        material: Material::Kashyyyk,
    },
    Body {
        name: "KAMINO",
        color: [144, 175, 216, 255],
        radius: 4.3,
        orbit: 103.0,
        phase: 5.48,
        speed: 0.042,
        spin: 0.12,
        material: Material::Kamino,
    },
    Body {
        name: "BESPIN",
        color: [231, 159, 96, 255],
        radius: 7.8,
        orbit: 130.0,
        phase: 0.4,
        speed: 0.034,
        spin: 0.1,
        material: Material::Bespin,
    },
    Body {
        name: "HOTH",
        color: [183, 220, 241, 255],
        radius: 3.4,
        orbit: OUTER_ORBIT,
        phase: 2.83,
        speed: 0.027,
        spin: 0.11,
        material: Material::Hoth,
    },
];

impl Body {
    fn angle(&self, time: f32) -> f32 {
        self.phase + self.speed * time
    }

    pub fn center(&self, time: f32) -> Vec3 {
        let (sin, cos) = self.angle(time).sin_cos();
        Vec3::new(self.orbit * cos, 0.0, self.orbit * sin)
    }

    fn velocity(&self, time: f32) -> Vec3 {
        let (sin, cos) = self.angle(time).sin_cos();
        Vec3::new(-sin, 0.0, cos) * (self.orbit * self.speed)
    }
}

struct Star {
    direction: Vec3,
    color: [u8; 4],
}

pub struct System {
    far: Mesh,
    middle: Mesh,
    near: Mesh,
    close: Mesh,
    ship: Mesh,
    stars: Vec<Star>,
}

impl System {
    pub fn new() -> Result<Self, String> {
        let mut far = Mesh::obj(include_str!("../assets/sphere.obj"))?;
        far.normalize_sphere();
        let middle = far.subdivide_sphere();
        let near = middle.subdivide_sphere();
        let close = near.subdivide_sphere();
        let mut ship = Mesh::obj(include_str!("../assets/nave_kylo2.obj"))?;
        ship.fit_ship();
        let mut state = 0x4b13_c920_u32;
        let mut random = || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            (state >> 8) as f32 / 16_777_216.0
        };
        let stars = (0..3150)
            .map(|_| {
                let y = 2.0 * random() - 1.0;
                let (sin, cos) = (random() * TAU).sin_cos();
                let horizontal = (1.0 - y * y).sqrt();
                let magnitude = 35.0 + 200.0 * random().powi(3);
                let warmth = random();
                Star {
                    direction: Vec3::new(horizontal * cos, y, horizontal * sin),
                    color: [
                        (magnitude * (0.82 + 0.18 * warmth)) as u8,
                        (magnitude * 0.93) as u8,
                        (magnitude * (1.0 - 0.19 * warmth)) as u8,
                        255,
                    ],
                }
            })
            .collect();
        Ok(Self {
            far,
            middle,
            near,
            close,
            ship,
            stars,
        })
    }

    pub fn render(&self, frame: &mut Frame, view: View, time: f32, orbits: bool) {
        frame.clear();
        for star in &self.stars {
            if let Some((x, y)) = frame.project_direction(view.camera, star.direction) {
                frame.blend(x, y, star.color);
                if star.color[0] > 197 {
                    let halo = [star.color[0], star.color[1], star.color[2], 46];
                    frame.blend(x - 1, y, halo);
                    frame.blend(x + 1, y, halo);
                    frame.blend(x, y - 1, halo);
                    frame.blend(x, y + 1, halo);
                }
            }
        }
        if orbits {
            for body in &BODIES[STAR_COUNT..] {
                for sample in 0..320 {
                    let (sin, cos) = (sample as f32 * TAU / 320.0).sin_cos();
                    let point = Vec3::new(cos * body.orbit, 0.0, sin * body.orbit);
                    if let Some((x, y)) = frame.project_world(view.camera, point) {
                        frame.blend(x, y, [148, 174, 193, 92]);
                    }
                }
            }
        }
        let sources = [BODIES[0].center(time), BODIES[1].center(time)];
        let light_at = |position: Vec3| {
            let primary = sources[0] - position;
            let secondary = sources[1] - position;
            let intensity = |offset: Vec3, power: f32| power / offset.dot(offset).max(1.0);
            (primary.normalized() * intensity(primary, 1.0)
                + secondary.normalized() * intensity(secondary, 0.48))
            .normalized()
        };
        for body in &BODIES {
            let model = Model {
                position: body.center(time),
                basis: Basis::yaw(body.spin * time),
                radius: body.radius,
            };
            if !frame.visible_sphere(view.camera, model) {
                frame.stats.culled += 1;
                continue;
            }
            let radius = frame.projected_radius(view.camera, model);
            let mesh = if radius > 245.0 {
                &self.close
            } else if radius > 115.0 {
                &self.near
            } else if radius > 46.0 {
                &self.middle
            } else {
                &self.far
            };
            frame.draw(
                view.camera,
                model,
                mesh,
                Surface::Planet(body.material),
                time,
                light_at(model.position),
            );
        }
        let model = Model {
            position: view.ship,
            basis: Basis::looking(view.heading, UP).banked(view.bank),
            radius: SHIP_RADIUS,
        };
        frame.draw(
            view.camera,
            model,
            &self.ship,
            Surface::Ship,
            time,
            light_at(model.position),
        );
        frame.rasterize();
    }
}

#[derive(Clone, Copy)]
pub struct View {
    pub camera: Camera,
    pub ship: Vec3,
    pub heading: Vec3,
    pub bank: f32,
}

#[derive(Clone, Copy, Default)]
pub struct Input {
    pub thrust: f32,
    pub strafe: f32,
    pub altitude: f32,
    pub mouse: [f32; 2],
    pub boost: bool,
    pub brake: bool,
    pub aim: Option<usize>,
}

pub struct Flight {
    pub position: Vec3,
    pub velocity: Vec3,
    yaw: f32,
    pitch: f32,
    steering: [f32; 2],
    bank: f32,
    camera_forward: Vec3,
    pub local: Option<usize>,
    pub blocked: bool,
}

impl Flight {
    pub fn new() -> Self {
        let position = START;
        let heading = (-position).normalized();
        let [yaw, pitch] = heading_angles(heading);
        Self {
            position,
            velocity: Vec3::zero(),
            yaw,
            pitch,
            steering: [yaw, pitch],
            bank: 0.0,
            camera_forward: heading,
            local: None,
            blocked: false,
        }
    }

    pub fn heading(&self) -> Vec3 {
        let (sin, cos) = self.yaw.sin_cos();
        let (up, horizontal) = self.pitch.sin_cos();
        Vec3::new(sin * horizontal, up, -cos * horizontal)
    }

    pub fn toggle_local(&mut self, time: f32) {
        self.local = if self.local.is_some() {
            None
        } else {
            self.nearest_planet(time).filter(|&index| {
                let body = &BODIES[index];
                (body.center(time) - self.position).length() - body.radius < LOCAL_RANGE
            })
        };
    }

    fn nearest_planet(&self, time: f32) -> Option<usize> {
        (STAR_COUNT..BODY_COUNT).min_by(|&a, &b| {
            let distance = |index: usize| {
                (BODIES[index].center(time) - self.position).length() - BODIES[index].radius
            };
            distance(a).total_cmp(&distance(b))
        })
    }

    pub fn update(&mut self, input: Input, time: f32, delta: f32) {
        // El delta del mouse pertenece al cuadro entero, no a cada subpaso de física.
        self.steering[0] = (self.steering[0] + input.mouse[0]).rem_euclid(TAU);
        self.steering[1] = (self.steering[1] + input.mouse[1]).clamp(-1.28, 1.28);
        let steps = (delta * 120.0).ceil().max(1.0) as u32;
        let dt = delta / steps as f32;
        self.blocked = false;
        for step in 0..steps {
            self.step(input, time + dt * step as f32, dt);
        }
    }

    fn step(&mut self, input: Input, time: f32, dt: f32) {
        let old_yaw = self.yaw;
        if let Some(index) = input.aim {
            let target = BODIES[self.local.unwrap_or(index)].center(time) - self.position;
            self.steering = heading_angles(target.normalized());
        }
        let yaw_delta = angle_delta(self.steering[0], self.yaw);
        let response = 1.0 - (-10.0 * dt).exp();
        self.yaw = (self.yaw + yaw_delta * response).rem_euclid(TAU);
        self.pitch += (self.steering[1] - self.pitch) * response;

        if let Some(index) = self.local {
            let body = &BODIES[index];
            let radial = (self.position - body.center(time)).normalized();
            let tangent = UP.cross(radial).normalized();
            let translation = if input.brake {
                Vec3::zero()
            } else {
                tangent * (input.strafe * 4.8) + UP * (input.altitude * 2.5)
                    - radial * (input.thrust * 3.3)
            };
            let desired = body.velocity(time) + translation;
            self.velocity += (desired - self.velocity) * (1.0 - (-4.5 * dt).exp());
        } else {
            let heading = self.heading();
            let right = heading.cross(UP).normalized();
            let direction = heading * input.thrust + right * input.strafe;
            let acceleration = if input.boost { 48.0 } else { 24.0 };
            let thrust = if input.brake {
                Vec3::zero()
            } else {
                direction * (acceleration / direction.length().max(1.0))
                    + UP * (input.altitude * 8.0)
            };
            let damping = if input.brake { 4.8 } else { 0.82 };
            self.velocity = self.velocity * (-damping * dt).exp() + thrust * dt;
        }

        let turn = angle_delta(self.yaw, old_yaw) / dt.max(f32::EPSILON);
        let desired_bank = (-0.18 * turn - input.strafe * 0.13).clamp(-0.44, 0.44);
        self.bank += (desired_bank - self.bank) * (1.0 - (-6.0 * dt).exp());
        let desired = self.position + self.velocity * dt;
        if let Some((body_index, fraction)) =
            first_collision(self.position, desired, time, time + dt, SHIP_RADIUS + 0.16)
        {
            let body = &BODIES[body_index];
            let end_center = body.center(time + dt);
            let impact = self.position + (desired - self.position) * fraction;
            let center = body.center(time) + (end_center - body.center(time)) * fraction;
            let normal = (impact - center).normalized();
            self.position = end_center + normal * (body.radius + SHIP_RADIUS + 0.19);
            let body_velocity = body.velocity(time + dt);
            let inward = (self.velocity - body_velocity).dot(normal).min(0.0);
            self.velocity -= normal * inward;
            self.blocked = true;
        } else {
            self.position = desired;
        }
    }

    pub fn recenter_view(&mut self) {
        self.camera_forward = self.heading();
    }

    pub fn view(&mut self, time: f32, delta: f32) -> View {
        let heading = self.heading();
        let response = 1.0 - (-11.0 * delta).exp();
        self.camera_forward =
            (self.camera_forward * (1.0 - response) + heading * response).normalized();
        let facing = self.camera_forward.dot(heading);
        let trailing = (self.camera_forward + heading * (0.60 - facing).max(0.0)).normalized();
        chase(self.position, heading, trailing, self.bank, time)
    }

    pub fn resume_from(&mut self, view: View) {
        self.position = view.ship;
        self.velocity = Vec3::zero();
        self.steering = heading_angles(view.heading);
        [self.yaw, self.pitch] = self.steering;
        self.bank = view.bank;
        self.camera_forward = -view.camera.basis.back;
        self.local = None;
        self.blocked = false;
    }
}

fn heading_angles(heading: Vec3) -> [f32; 2] {
    [
        heading.x.atan2(-heading.z),
        heading.y.asin().clamp(-1.28, 1.28),
    ]
}

fn angle_delta(target: f32, current: f32) -> f32 {
    (target - current + TAU * 0.5).rem_euclid(TAU) - TAU * 0.5
}

pub fn aimed_planet(camera: Camera, time: f32) -> Option<usize> {
    let end = camera.eye - camera.basis.back * FAR;
    first_collision(camera.eye, end, time, time, 0.0)
        .map(|(index, _)| index)
        .filter(|&index| index >= STAR_COUNT)
}

fn first_collision(
    from: Vec3,
    to: Vec3,
    start: f32,
    end: f32,
    padding: f32,
) -> Option<(usize, f32)> {
    BODIES
        .iter()
        .enumerate()
        .filter_map(|(index, body)| {
            let initial = from - body.center(start);
            let movement = to - body.center(end) - initial;
            let radius = body.radius + padding;
            let c = initial.dot(initial) - radius * radius;
            if c < 0.0 {
                return Some((index, 0.0));
            }
            let a = movement.dot(movement);
            let b = initial.dot(movement);
            let discriminant = b * b - a * c;
            if a <= f32::EPSILON || b >= 0.0 || discriminant < 0.0 {
                return None;
            }
            let fraction = (-b - discriminant.sqrt()) / a;
            (0.0..=1.0).contains(&fraction).then_some((index, fraction))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

fn chase(ship: Vec3, heading: Vec3, forward: Vec3, bank: f32, time: f32) -> View {
    let forward = Vec3::new(forward.x, forward.y.clamp(-0.95, 0.95), forward.z).normalized();
    let basis = Basis::looking(forward, UP);
    let side_view = 1.0 - forward.dot(heading).clamp(0.0, 1.0);
    let offset = -forward * 2.85 - heading * (0.46 * side_view) + basis.up * 0.38;
    let desired = ship + offset - heading * (offset.dot(heading) + 0.24).max(0.0);
    let (mut eye, obstacle) = camera_path(ship, desired, time);
    if let Some(index) = obstacle.filter(|_| (eye - ship).length() < 0.90) {
        let outward = (ship - BODIES[index].center(time)).normalized();
        let transverse = outward - heading * outward.dot(heading);
        let detour = if transverse.length() > 0.15 {
            transverse.normalized()
        } else {
            Basis::looking(heading, UP).up
        };
        let alternative = ship + detour * 2.65 - heading * 0.32;
        let (clear_eye, _) = camera_path(ship, alternative, time);
        if (clear_eye - ship).length() > (eye - ship).length() {
            eye = clear_eye;
        }
    }
    // El punto de mira queda un poco por encima de la nave en cualquier rumbo.
    let sight = ship - eye;
    let screen_up = Basis::looking(sight, UP).up;
    let focus = ship + screen_up * (0.13 * sight.length());
    View {
        camera: Camera::look(eye, focus),
        ship,
        heading,
        bank,
    }
}

fn camera_path(ship: Vec3, desired: Vec3, time: f32) -> (Vec3, Option<usize>) {
    match first_collision(ship, desired, time, time, CAMERA_RADIUS) {
        Some((index, fraction)) => (
            ship + (desired - ship) * (fraction - 0.025).max(0.0),
            Some(index),
        ),
        None => (desired, None),
    }
}

fn ease(value: f32) -> f32 {
    let t = value.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn frame_target(heading: Vec3, toward: Vec3, amount: f32) -> Vec3 {
    let projection = heading.dot(toward);
    let sideways = toward - heading * projection;
    (heading * (1.0 - amount + amount * projection.max(0.14)) + sideways * amount).normalized()
}

fn approach_position(time: f32) -> Vec3 {
    let body = &BODIES[SCARIF];
    let center = body.center(time);
    let sunlight = (BODIES[0].center(time) - center).normalized();
    let lap = ease((time - 26.0) / 26.0);
    let orbit_angle = -0.92 + lap * TAU;
    let (sin, cos) = orbit_angle.sin_cos();
    let radial = (sunlight * cos + UP.cross(sunlight) * sin + UP * 0.32).normalized();
    let arrival = ease(time / 26.0);
    let initial_offset = START - center;
    let align = ease(time / 20.0);
    let direction = (initial_offset.normalized() * (1.0 - align) + radial * align).normalized();
    let distance = initial_offset.length() * (1.0 - arrival) + (body.radius + 3.9) * arrival;
    center + direction * distance
}

fn tour_position(time: f32) -> Vec3 {
    if time <= 52.0 {
        return approach_position(time);
    }
    let t = ((time - 52.0) / 20.0).clamp(0.0, 1.0);
    let from = approach_position(52.0);
    let entry_velocity = (from - approach_position(51.95)) * 400.0;
    let to = Vec3::new(4.5, 11.0, 23.0);
    let exit_velocity = (Vec3::new(0.0, 1.0, 0.0) - to).normalized() * 41.0;
    // La curva conserva la tangente al salir de la órbita y al terminar el vuelo.
    let t2 = t * t;
    let t3 = t2 * t;
    from * (2.0 * t3 - 3.0 * t2 + 1.0)
        + entry_velocity * (t3 - 2.0 * t2 + t)
        + to * (-2.0 * t3 + 3.0 * t2)
        + exit_velocity * (t3 - t2)
}

fn tour_heading(time: f32) -> Vec3 {
    (tour_position((time + 0.32).min(TOUR_LENGTH)) - tour_position((time - 0.32).max(0.0)))
        .normalized()
}

pub fn tour(time: f32) -> (View, &'static str) {
    let ship = tour_position(time);
    let center = BODIES[SCARIF].center(time);
    let departure = ease((time - 52.0) / 10.0);
    let target = center * ease((time - 1.6) / 14.0) * (1.0 - departure);
    let toward = (target - ship).normalized();
    let moving = tour_heading(time);
    let intro = ease((time - 2.8) / 8.0);
    let tangent = UP.cross(ship - center).normalized();
    let orbit = ease((time - 24.0) / 4.0) * (1.0 - ease((time - 50.8) / 4.6));
    let course = (moving * (1.0 - orbit) + tangent * orbit).normalized();
    let heading = (toward * (1.0 - intro) + course * intro).normalized();
    let framing = (0.77 + 0.225 * ease((time - 18.0) / 8.0)) * (1.0 - 0.13 * departure);
    let forward = frame_target(heading, toward, framing);
    let bank = (UP.dot(tour_heading(time - 0.24).cross(tour_heading(time + 0.24))) * 0.85)
        .clamp(-0.50, 0.50)
        * intro;
    let stage = if time < 10.0 {
        "01  VUELO ESPACIAL"
    } else if time < 26.0 {
        "02  ACERCAMIENTO"
    } else if time < 52.0 {
        "03  VUELTA AL PLANETA"
    } else {
        "04  ALEJAMIENTO"
    };
    (chase(ship, heading, forward, bank, time), stage)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ship_windshield_points_forward_after_import() {
        let mut ship = Mesh::obj(include_str!("../assets/nave_kylo2.obj")).unwrap();
        ship.fit_ship();
        let windshield: Vec<_> = ship.faces.iter().filter(|face| face.glass).collect();
        assert!(!windshield.is_empty());
        assert!(windshield
            .iter()
            .flat_map(|face| face.indices)
            .all(|index| ship.vertices[index].z < 0.0));
    }

    fn assert_ship_centered(frame: &Frame, view: View) {
        let (x, y) = frame.project_world(view.camera, view.ship).unwrap();
        assert!(
            (0.47..=0.53).contains(&(x as f32 / frame.width as f32)),
            "x={x}"
        );
        assert!(
            (0.57..=0.66).contains(&(y as f32 / frame.height as f32)),
            "y={y}"
        );
    }

    #[test]
    fn mouse_steers_the_ship_and_keeps_it_in_frame() {
        let mut flight = Flight::new();
        let frame = Frame::new(1600, 900);
        let dt = 1.0 / 60.0;
        let initial = flight.view(0.0, dt);
        let mut view = initial;
        for step in 0..120 {
            let time = step as f32 * dt;
            let mouse = if step < 42 { [0.047, 0.013] } else { [0.0; 2] };
            flight.update(
                Input {
                    mouse,
                    ..Input::default()
                },
                time,
                dt,
            );
            view = flight.view(time, dt);
            assert_ship_centered(&frame, view);
            assert!((view.camera.eye - view.ship).dot(view.heading) < 0.0);
        }
        assert!((view.heading - initial.heading).length() > 1.0);
        assert!((view.camera.basis.back - initial.camera.basis.back).length() > 1.0);
        assert_eq!(view.ship, initial.ship);
    }

    #[test]
    fn strafe_moves_sideways_without_turning() {
        let mut flight = Flight::new();
        let heading = flight.heading();
        let right = heading.cross(UP).normalized();
        let start = flight.position;
        let dt = 1.0 / 60.0;
        for step in 0..60 {
            flight.update(
                Input {
                    strafe: 1.0,
                    ..Input::default()
                },
                step as f32 * dt,
                dt,
            );
        }
        assert!((flight.heading() - heading).length() < 1.0e-5);
        assert!((flight.position - start).dot(right) > 5.0);
    }

    #[test]
    fn camera_keeps_the_ship_visible_with_a_planet_directly_behind_it() {
        let body = &BODIES[SCARIF];
        let heading = Vec3::new(0.0, 0.0, 1.0);
        let ship = body.center(0.0) + heading * (body.radius + SHIP_RADIUS + 0.19);
        let view = chase(ship, heading, heading, 0.0, 0.0);
        let frame = Frame::new(1600, 900);
        assert_ship_centered(&frame, view);
        assert!((view.camera.eye - body.center(0.0)).length() > body.radius + CAMERA_RADIUS);
        assert!((view.camera.eye - view.ship).length() > 1.0);
        assert!((view.camera.eye - view.ship).dot(heading) < 0.0);
    }

    #[test]
    fn catalogue_target_follows_the_camera_and_respects_occlusion() {
        let time = 0.0;
        let target = BODIES[SCARIF].center(time);
        let eye = target + Vec3::new(0.0, 25.0, 0.0);
        for (index, body) in BODIES.iter().enumerate().skip(STAR_COUNT) {
            let center = body.center(time);
            let direct = Camera::look(center + Vec3::new(1.0, 25.0, 0.0), center);
            assert_eq!(aimed_planet(direct, time), Some(index));
            assert!(crate::catalogue::planet(index).is_some());
        }

        let empty = Camera::look(eye, eye + Vec3::new(0.0, 1.0, -0.1));
        assert_eq!(aimed_planet(empty, time), None);

        let near = BODIES[0].center(time);
        let toward = (target - near).normalized();
        let behind = near - toward * (BODIES[0].radius + 2.0);
        assert_eq!(aimed_planet(Camera::look(behind, target), time), None);
    }

    #[test]
    fn all_planets_orbit_on_the_ecliptic() {
        for body in &BODIES[STAR_COUNT..] {
            let before = body.center(0.0);
            let after = body.center(1.0);
            assert_eq!(before.y, 0.0);
            assert_eq!(after.y, 0.0);
            assert!((after.length() - body.orbit).abs() < 1.0e-4);
            assert!((before - after).length() > 0.5);
            assert_ne!(body.spin, 0.0);
        }
    }

    #[test]
    fn collision_detects_a_fast_crossing_and_motion_of_the_body() {
        let (star, hit) = first_collision(
            BODIES[0].center(0.0) + Vec3::new(0.0, 0.0, -14.0),
            BODIES[0].center(0.0) + Vec3::new(0.0, 0.0, 14.0),
            0.0,
            0.0,
            SHIP_RADIUS,
        )
        .unwrap();
        assert_eq!(BODIES[star].name, "TATOO I");
        assert!(hit > 0.0 && hit < 0.5);

        let body = &BODIES[SCARIF];
        let position = body.center(1.0) + body.velocity(1.0).normalized() * (body.radius + 1.5);
        let (planet, _) = first_collision(position, position, 1.0, 2.0, SHIP_RADIUS).unwrap();
        assert_eq!(BODIES[planet].name, "SCARIF");
    }

    #[test]
    fn flight_stays_outside_the_surface_when_thrusting_toward_it() {
        let mut flight = Flight::new();
        flight.position = BODIES[0].center(0.0) + Vec3::new(0.0, 0.0, 14.0);
        flight.yaw = 0.0;
        flight.pitch = 0.0;
        flight.steering = [0.0; 2];
        let dt = 1.0 / 60.0;
        let mut blocked = false;
        for step in 0..240 {
            flight.update(
                Input {
                    thrust: 1.0,
                    boost: true,
                    ..Input::default()
                },
                step as f32 * dt,
                dt,
            );
            blocked |= flight.blocked;
            assert!(
                (flight.position - BODIES[0].center((step + 1) as f32 * dt)).length()
                    >= BODIES[0].radius + SHIP_RADIUS
            );
        }
        assert!(blocked);
    }

    #[test]
    fn local_flight_follows_the_moving_planet_while_circling() {
        let mut flight = Flight::new();
        let body = &BODIES[SCARIF];
        let initial = Vec3::new(0.0, 0.0, body.radius + 4.0);
        flight.position = body.center(0.0) + initial;
        flight.toggle_local(0.0);
        assert_eq!(flight.local, Some(SCARIF));
        let dt = 1.0 / 60.0;
        for step in 0..120 {
            flight.update(
                Input {
                    strafe: 1.0,
                    ..Input::default()
                },
                step as f32 * dt,
                dt,
            );
        }
        let relative = flight.position - body.center(2.0);
        assert!((relative - initial).length() > 4.0);
        assert!((relative.length() - initial.length()).abs() < 1.6);
        assert!(relative.length() > body.radius + SHIP_RADIUS);
    }

    #[test]
    fn tour_keeps_the_ship_and_camera_clear_for_the_full_route() {
        let frame = Frame::new(1920, 1080);
        for index in 0..=(TOUR_LENGTH * 120.0) as u32 {
            let time = index as f32 / 120.0;
            let (view, _) = tour(time);
            assert_ship_centered(&frame, view);
            assert!(
                (view.camera.eye - view.ship).dot(view.heading) <= 0.001,
                "cámara delante en t={time}"
            );
            for body in &BODIES {
                let center = body.center(time);
                let ship_distance = (view.ship - center).length();
                let camera_distance = (view.camera.eye - center).length();
                assert!(
                    ship_distance > body.radius + SHIP_RADIUS,
                    "{} t={time} ship {ship_distance}",
                    body.name
                );
                assert!(
                    camera_distance > body.radius + CAMERA_RADIUS,
                    "{} t={time} camera {camera_distance}",
                    body.name
                );
            }
        }
    }
}
