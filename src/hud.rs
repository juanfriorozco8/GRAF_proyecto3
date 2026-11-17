use std::collections::HashMap;
use std::f32::consts::TAU;

use fontdue::{Font, FontSettings, Metrics};

use crate::catalogue::PlanetInfo;
use crate::geometry::Vec3;
use crate::pipeline::Frame;
use crate::system::{aimed_planet, View, BODIES, OUTER_ORBIT};

const WHITE: [u8; 4] = [248, 248, 248, 255];
const SECONDARY: [u8; 4] = [229, 229, 229, 248];
const MUTED: [u8; 4] = [194, 194, 194, 238];
const RULE: [u8; 4] = [244, 244, 244, 81];
const MARGIN: i32 = 34;

pub struct Hud {
    font: Font,
    glyphs: HashMap<(char, u8), (Metrics, Vec<u8>)>,
}

pub struct Overlay {
    pub mode: &'static str,
    pub body: usize,
    pub time: f32,
    pub speed: f32,
    pub help: bool,
    pub blocked: bool,
    pub diagnostics: bool,
    pub mouse_enabled: Option<bool>,
}

impl Hud {
    pub fn new() -> Result<Self, &'static str> {
        let font = Font::from_bytes(
            include_bytes!("../assets/fonts/Lato-Medium.ttf") as &[u8],
            FontSettings::default(),
        )?;
        Ok(Self {
            font,
            glyphs: HashMap::new(),
        })
    }

    fn text(
        &mut self,
        frame: &mut Frame,
        text: &str,
        position: (i32, i32),
        style: (u8, f32, [u8; 4]),
    ) {
        let (size, tracking, color) = style;
        let mut pen = position.0 as f32;
        for character in text.chars() {
            let (glyph, bitmap) = self
                .glyphs
                .entry((character, size))
                .or_insert_with(|| self.font.rasterize(character, f32::from(size)));
            let left = pen.round() as i32 + glyph.xmin;
            let top = position.1 - glyph.ymin - glyph.height as i32;
            for (index, &alpha) in bitmap.iter().enumerate() {
                if alpha != 0 {
                    let mut pixel = color;
                    pixel[3] = (u16::from(alpha) * u16::from(color[3]) / 255) as u8;
                    frame.blend(
                        left + (index % glyph.width) as i32,
                        top + (index / glyph.width) as i32,
                        pixel,
                    );
                }
            }
            pen += glyph.advance_width + tracking;
        }
    }

    fn wrap(&self, text: &str, size: u8, max_width: f32) -> Vec<String> {
        let mut lines = Vec::new();
        let mut line = String::new();
        let mut width = 0.0;
        let advance = |c| self.font.metrics(c, f32::from(size)).advance_width + 0.25;
        let space = advance(' ');
        for word in text.split_whitespace() {
            let word_width = word.chars().map(advance).sum::<f32>();
            if !line.is_empty() && width + space + word_width > max_width {
                lines.push(std::mem::take(&mut line));
                width = 0.0;
            }
            if !line.is_empty() {
                line.push(' ');
                width += space;
            }
            line.push_str(word);
            width += word_width;
        }
        if !line.is_empty() {
            lines.push(line);
        }
        lines
    }

    fn panel(frame: &mut Frame, x: i32, y: i32, width: i32, height: i32, alpha: u8) {
        for row in y..y + height {
            for column in x..x + width {
                frame.blend(column, row, [4, 6, 10, alpha]);
            }
        }
        frame.line((x, y), (x + width - 1, y), RULE);
    }

    pub fn draw(&mut self, frame: &mut Frame, view: View, overlay: Overlay) {
        let right = frame.width as i32 - MARGIN;
        let bottom = frame.height as i32 - 29;
        Self::panel(frame, MARGIN, 28, 300, 94, 180);
        self.text(frame, "STAR WARS", (MARGIN + 12, 62), (30, 3.6, WHITE));
        self.text(
            frame,
            "SOLAR SYSTEM",
            (MARGIN + 14, 86),
            (14, 3.5, SECONDARY),
        );
        self.text(
            frame,
            overlay.mode,
            (MARGIN + 13, 109),
            (13, 1.35, SECONDARY),
        );

        Self::panel(frame, right - 230, 28, 230, 80, 180);
        let body = &BODIES[overlay.body];
        let distance = ((view.ship - body.center(overlay.time)).length() - body.radius).max(0.0);
        self.text(
            frame,
            &format!("RUMBO / {}", body.name),
            (right - 216, 52),
            (13, 1.0, WHITE),
        );
        self.text(
            frame,
            &format!("ALT {:05.1}  ·  VEL {:04.1}", distance, overlay.speed),
            (right - 216, 76),
            (15, 0.5, SECONDARY),
        );
        if overlay.blocked {
            self.text(
                frame,
                "LÍMITE DE SEGURIDAD",
                (right - 216, 97),
                (11, 1.1, WHITE),
            );
        }

        let target = aimed_planet(view.camera, overlay.time);
        Self::reticle(frame, target.is_some());
        if let Some(index) = target {
            if let Some(info) = crate::catalogue::planet(index) {
                self.planet_card(
                    frame,
                    index,
                    info,
                    bottom
                        - if overlay.mouse_enabled.is_some() {
                            32
                        } else {
                            0
                        },
                );
            }
        }
        if let Some(mouse_enabled) = overlay.mouse_enabled {
            let controls = if mouse_enabled {
                "H  AYUDA     M  SOLTAR MOUSE     C  CENTRAR"
            } else {
                "H  AYUDA     M  ACTIVAR MOUSE     C  CENTRAR"
            };
            Self::panel(frame, MARGIN, bottom - 25, 390, 31, 155);
            self.text(
                frame,
                controls,
                (MARGIN + 13, bottom - 4),
                (12, 0.9, SECONDARY),
            );
        }
        self.map(frame, view.ship, &overlay, right, bottom + 6);
        if overlay.diagnostics {
            Self::panel(frame, MARGIN, 133, 300, 36, 220);
            self.text(
                frame,
                &format!(
                    "TRIÁNGULOS {:05}  ·  FUERA {}",
                    frame.stats.triangles, frame.stats.culled
                ),
                (MARGIN + 13, 156),
                (13, 0.7, SECONDARY),
            );
        }
        if overlay.help {
            self.help(frame, right, overlay.mode == "VUELO LOCAL");
        }
    }

    fn reticle(frame: &mut Frame, locked: bool) {
        let x = frame.width as i32 / 2;
        let y = frame.height as i32 / 2;
        let color = [255, 255, 255, if locked { 205 } else { 101 }];
        for sign in [-1, 1] {
            frame.line((x + sign * 7, y), (x + sign * 12, y), color);
            frame.line((x, y + sign * 7), (x, y + sign * 12), color);
        }
    }

    fn planet_card(&mut self, frame: &mut Frame, index: usize, info: &PlanetInfo, bottom: i32) {
        const WIDTH: i32 = 474;
        const TEXT_WIDTH: f32 = (WIDTH - 34) as f32;
        let description = self.wrap(info.description, 17, TEXT_WIDTH);
        let scene = self.wrap(info.scene, 16, TEXT_WIDTH);
        let height = 170 + 22 * (description.len() + scene.len()) as i32;
        let top = bottom - height;
        let x = MARGIN + 16;
        Self::panel(frame, MARGIN, top, WIDTH, height, 228);
        self.text(
            frame,
            "REGISTRO PLANETARIO",
            (x, top + 21),
            (11, 2.2, MUTED),
        );
        self.text(frame, BODIES[index].name, (x, top + 54), (30, 2.0, WHITE));
        let mut y = top + 79;
        for line in description {
            self.text(frame, &line, (x, y), (17, 0.25, WHITE));
            y += 22;
        }
        y += 2;
        for (label, value) in [
            ("TIPO", info.kind),
            ("CLIMA", info.climate),
            ("HABITANTES", info.inhabitants),
        ] {
            self.text(frame, label, (x, y), (11, 1.35, MUTED));
            self.text(frame, value, (x + 112, y), (16, 0.2, WHITE));
            y += 21;
        }
        frame.line((x, y - 9), (MARGIN + WIDTH - 17, y - 9), RULE);
        y += 7;
        self.text(
            frame,
            &format!("ESCENA / {}", info.film.to_uppercase()),
            (x, y),
            (11, 1.5, MUTED),
        );
        for line in scene {
            y += 22;
            self.text(frame, &line, (x, y), (16, 0.25, WHITE));
        }
    }

    fn map(&mut self, frame: &mut Frame, ship: Vec3, overlay: &Overlay, right: i32, bottom: i32) {
        let (w, h) = (146, 150);
        let (left, top) = (right - w, bottom - h);
        Self::panel(frame, left, top, w, h, 178);
        self.text(
            frame,
            "MAPA ORBITAL",
            (left + 11, top + 19),
            (10, 1.4, MUTED),
        );
        let center = (left + w / 2, top + 82);
        let scale = 54.0 / OUTER_ORBIT;
        let place = |x: f32, z: f32| {
            (
                center.0 + (x * scale).round() as i32,
                center.1 + (z * scale).round() as i32,
            )
        };
        for (index, body) in BODIES.iter().enumerate() {
            for step in 0..108 {
                let (sin, cos) = (step as f32 * TAU / 108.0).sin_cos();
                let (x, y) = place(body.orbit * cos, body.orbit * sin);
                frame.blend(x, y, [178, 178, 178, 65]);
            }
            let point = body.center(overlay.time);
            let (x, y) = place(point.x, point.z);
            let [r, g, b, _] = body.color;
            let marker = [r, g, b, if index == overlay.body { 255 } else { 159 }];
            frame.line((x - 1, y), (x + 1, y), marker);
            frame.line((x, y - 1), (x, y + 1), marker);
            if index == overlay.body {
                frame.line((x - 2, y - 3), (x + 2, y - 3), WHITE);
            }
        }
        let (x, y) = place(ship.x, ship.z);
        let (x, y) = (x.clamp(left + 6, right - 6), y.clamp(top + 26, bottom - 6));
        frame.line((x - 2, y), (x + 2, y), WHITE);
        frame.line((x, y - 2), (x, y + 2), WHITE);
    }

    fn help(&mut self, frame: &mut Frame, right: i32, local: bool) {
        let x = right - 465;
        let y = 122;
        Self::panel(frame, x, y, 465, 314, 238);
        self.text(
            frame,
            "CONTROLES DE VUELO",
            (x + 16, y + 28),
            (16, 2.0, WHITE),
        );
        let rows = [
            ("MOUSE", "Girar e inclinar la nave y la vista"),
            ("M / C", "Soltar o capturar el cursor / centrar la vista"),
            (
                "W / S",
                if local {
                    "Acercarse / alejarse"
                } else {
                    "Avanzar / retroceder"
                },
            ),
            (
                "A / D",
                if local {
                    "Rodear el planeta"
                } else {
                    "Desplazarse a izquierda / derecha"
                },
            ),
            ("Q / E", "Subir / bajar"),
            ("MAYÚS / ESPACIO", "Impulso en vuelo libre / frenar"),
            ("1–8 / F", "Elegir destino / orientar la nave"),
            ("L / T", "Vuelo local / recorrido de muestra"),
            ("O / F3", "Trazas orbitales / diagnóstico"),
            ("R / ESC", "Reiniciar / cerrar"),
        ];
        for (index, (key, description)) in rows.into_iter().enumerate() {
            let baseline = y + 54 + index as i32 * 27;
            self.text(frame, key, (x + 16, baseline), (12, 0.8, MUTED));
            self.text(frame, description, (x + 147, baseline), (14, 0.16, WHITE));
        }
    }
}
