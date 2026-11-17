mod catalogue;
mod geometry;
mod hud;
mod pipeline;
mod shaders;
mod system;

use std::error::Error;
use std::io::{self, Write};
use std::process::ExitCode;

use raylib::prelude::*;

use crate::hud::{Hud, Overlay};
use crate::pipeline::Frame;
use crate::system::{tour, Flight, Input, System, SCARIF, STAR_COUNT, TOUR_LENGTH};

const WIDTH: usize = 1600;
const HEIGHT: usize = 900;
const VIDEO_WIDTH: usize = 1920;
const VIDEO_HEIGHT: usize = 1080;
const TOUR_FPS: u32 = 30;
const MOUSE_SENSITIVITY: f32 = 0.0035;

fn run_window(system: &System, hud: &mut Hud, frame: &mut Frame) -> Result<(), Box<dyn Error>> {
    let (mut rl, thread) = raylib::init()
        .size(WIDTH as i32, HEIGHT as i32)
        .title("Star Wars Solar System")
        .build();
    rl.set_target_fps(60);
    let image = Image::gen_image_color(WIDTH as i32, HEIGHT as i32, Color::BLACK);
    let mut texture = rl.load_texture_from_image(&thread, &image)?;
    let mut flight = Flight::new();
    let mut time = 0.0;
    let mut showing_tour = false;
    let mut help = false;
    let mut diagnostics = false;
    let mut orbits = true;
    let mut selected = SCARIF;
    let mut mouse_captured = false;
    let mut mouse_enabled = true;
    let targets = [
        KeyboardKey::KEY_ONE,
        KeyboardKey::KEY_TWO,
        KeyboardKey::KEY_THREE,
        KeyboardKey::KEY_FOUR,
        KeyboardKey::KEY_FIVE,
        KeyboardKey::KEY_SIX,
        KeyboardKey::KEY_SEVEN,
        KeyboardKey::KEY_EIGHT,
    ];
    while !rl.window_should_close() {
        let delta = rl.get_frame_time().min(0.1);
        for (index, key) in targets.into_iter().enumerate() {
            if rl.is_key_pressed(key) {
                selected = index + STAR_COUNT;
            }
        }
        if rl.is_key_pressed(KeyboardKey::KEY_H) {
            help = !help;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_M) {
            mouse_enabled = !mouse_enabled;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_C) {
            flight.recenter_view();
        }
        if rl.is_key_pressed(KeyboardKey::KEY_F3) {
            diagnostics = !diagnostics;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_O) {
            orbits = !orbits;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_R) {
            flight = Flight::new();
            time = 0.0;
            showing_tour = false;
            selected = SCARIF;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_T) {
            if showing_tour {
                flight.resume_from(tour(time).0);
            } else {
                time = 0.0;
            }
            showing_tour = !showing_tour;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_L) && !showing_tour {
            flight.toggle_local(time);
        }
        let should_capture = mouse_enabled && !showing_tour && rl.is_window_focused();
        let mut mouse = [0.0; 2];
        match (mouse_captured, should_capture) {
            (false, true) => rl.disable_cursor(),
            (true, false) => rl.enable_cursor(),
            (true, true) => {
                let movement = rl.get_mouse_delta();
                mouse = [
                    movement.x * MOUSE_SENSITIVITY,
                    -movement.y * MOUSE_SENSITIVITY,
                ];
            }
            (false, false) => {}
        }
        mouse_captured = should_capture;
        let (view, mode, speed, active) = if showing_tour {
            let previous = tour(time).0.ship;
            time = (time + delta).min(TOUR_LENGTH);
            let (view, stage) = tour(time);
            let speed = (view.ship - previous).length() / delta.max(f32::EPSILON);
            if time >= TOUR_LENGTH {
                showing_tour = false;
                flight.resume_from(view);
            }
            (view, stage, speed, SCARIF)
        } else {
            let held = |key| f32::from(rl.is_key_down(key));
            let qe = held(KeyboardKey::KEY_Q) - held(KeyboardKey::KEY_E);
            let local = flight.local.is_some();
            let input = Input {
                thrust: held(KeyboardKey::KEY_W) - held(KeyboardKey::KEY_S),
                strafe: held(KeyboardKey::KEY_D) - held(KeyboardKey::KEY_A),
                altitude: qe,
                mouse,
                boost: rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT)
                    || rl.is_key_down(KeyboardKey::KEY_RIGHT_SHIFT),
                brake: rl.is_key_down(KeyboardKey::KEY_SPACE),
                aim: rl.is_key_down(KeyboardKey::KEY_F).then_some(selected),
            };
            flight.update(input, time, delta);
            time += delta;
            let active = flight.local.unwrap_or(selected);
            let mode = if local { "VUELO LOCAL" } else { "VUELO LIBRE" };
            (
                flight.view(time, delta),
                mode,
                flight.velocity.length(),
                active,
            )
        };
        system.render(frame, view, time, orbits);
        hud.draw(
            frame,
            view,
            Overlay {
                mode,
                body: active,
                time,
                speed,
                help,
                blocked: flight.blocked && !showing_tour,
                diagnostics,
                mouse_enabled: (!showing_tour).then_some(mouse_enabled),
            },
        );
        texture.update_texture(frame.rgba())?;
        let mut draw = rl.begin_drawing(&thread);
        draw.draw_texture(&texture, 0, 0, Color::WHITE);
    }
    if mouse_captured {
        rl.enable_cursor();
    }
    Ok(())
}

fn record_tour(
    system: &System,
    hud: &mut Hud,
    frame: &mut Frame,
    start: u32,
    count: u32,
) -> io::Result<()> {
    let mut out = io::BufWriter::new(io::stdout().lock());
    for index in start..start + count {
        let time = index as f32 / TOUR_FPS as f32;
        let (view, mode) = tour(time);
        let prior = tour((time - (TOUR_FPS as f32).recip()).max(0.0)).0.ship;
        let speed = (view.ship - prior).length() * TOUR_FPS as f32;
        system.render(frame, view, time, true);
        hud.draw(
            frame,
            view,
            Overlay {
                mode,
                body: SCARIF,
                time,
                speed,
                help: false,
                blocked: false,
                diagnostics: false,
                mouse_enabled: None,
            },
        );
        out.write_all(frame.rgba())?;
    }
    out.flush()
}

fn run() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let system = System::new()?;
    let mut hud = Hud::new()?;
    match arguments.as_slice() {
        [] => run_window(&system, &mut hud, &mut Frame::new(WIDTH, HEIGHT)),
        [flag, start, count] if flag == "--tour-frames" => {
            let start: u32 = start.parse()?;
            let count: u32 = count.parse()?;
            let max = (TOUR_LENGTH * TOUR_FPS as f32) as u32;
            if count == 0 || start.checked_add(count).is_none_or(|end| end > max) {
                return Err(format!(
                    "El intervalo debe tener entre 1 y {max} cuadros dentro del recorrido."
                )
                .into());
            }
            record_tour(
                &system,
                &mut hud,
                &mut Frame::new(VIDEO_WIDTH, VIDEO_HEIGHT),
                start,
                count,
            )?;
            Ok(())
        }
        _ => Err(
            "Uso: graf-proyecto3 [--tour-frames <inicio> <cantidad>] (salida RGBA de 1920x1080)"
                .into(),
        ),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
