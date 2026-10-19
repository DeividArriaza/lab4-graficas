mod color;
mod framebuffer;
mod obj;
mod png;
mod triangle;

use std::path::Path;
use std::process;
use std::time::Duration;

use minifb::{Key, KeyRepeat, Window, WindowOptions};
use nalgebra_glm::{Vec2, Vec3};

use crate::color::Color;
use crate::framebuffer::Framebuffer;
use crate::obj::{Face, Obj};
use crate::png::WritePng;
use crate::triangle::Triangle;

const SCREEN_WIDTH: usize = 800;
const SCREEN_HEIGHT: usize = 600;
const MARGIN: f32 = 0.8; // La nave ocupa como máximo el 80 % de la ventana.

const DEFAULT_MODEL: &str = "assets/nave.obj";
const DEFAULT_SCREENSHOT: &str = "screenshots/nave_renderizada.png";

const BACKGROUND: Color = Color::new(16, 18, 32);
const YELLOW: Color = Color::new(255, 220, 0);

/// Proyección ortográfica superior: el plano X-Z del modelo se centra y se
/// escala para caber en la ventana. La proa (-Z) queda hacia arriba.
fn project_vertices(vertices: &[Vec3]) -> Result<Vec<Vec2>, String> {
    let first = vertices.first().ok_or("El modelo no tiene vértices")?;
    let mut min = *first;
    let mut max = *first;
    for v in vertices {
        min = nalgebra_glm::min2(&min, v);
        max = nalgebra_glm::max2(&max, v);
    }

    let center_x = (min.x + max.x) / 2.0;
    let center_z = (min.z + max.z) / 2.0;
    let width = max.x - min.x;
    let length = max.z - min.z;
    if width <= 0.0 || length <= 0.0 {
        return Err("El modelo no tiene extensión en X o Z; no se puede escalar".into());
    }

    let scale = (MARGIN * SCREEN_WIDTH as f32 / width).min(MARGIN * SCREEN_HEIGHT as f32 / length);

    Ok(vertices
        .iter()
        .map(|v| {
            Vec2::new(
                SCREEN_WIDTH as f32 / 2.0 + scale * (v.x - center_x),
                SCREEN_HEIGHT as f32 / 2.0 + scale * (v.z - center_z),
            )
        })
        .collect())
}

/// Recorre manualmente todas las caras y rellena cada triángulo.
fn render(framebuffer: &mut Framebuffer, faces: &[Face], screen_vertices: &[Vec2]) {
    for face in faces {
        let a = &screen_vertices[face[0]];
        let b = &screen_vertices[face[1]];
        let c = &screen_vertices[face[2]];
        framebuffer.triangle(a, b, c);
    }
}

fn draw_frame(framebuffer: &mut Framebuffer, faces: &[Face], screen_vertices: &[Vec2]) {
    framebuffer.clear();
    framebuffer.set_current_color(YELLOW);
    render(framebuffer, faces, screen_vertices);
}

fn save_screenshot(framebuffer: &Framebuffer, path: &str) {
    if let Some(parent) = Path::new(path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match framebuffer.render_png(path) {
        Ok(()) => println!("Captura guardada en {path}"),
        Err(e) => eprintln!("No se pudo guardar la captura en {path}: {e}"),
    }
}

/// Busca el modelo relativo al directorio actual (convención: ejecutar desde la
/// raíz del repositorio) y, si no está, relativo a la raíz del crate.
fn resolve_model_path(requested: &str) -> String {
    if Path::new(requested).exists() {
        return requested.to_string();
    }
    let from_crate = Path::new(env!("CARGO_MANIFEST_DIR")).join(requested);
    if from_crate.exists() {
        return from_crate.to_string_lossy().into_owned();
    }
    requested.to_string()
}

fn main() {
    let mut model_path = DEFAULT_MODEL.to_string();
    let mut screenshot_path: Option<String> = None; // Guarda un cuadro y termina.

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--screenshot" {
            screenshot_path = Some(args.next().unwrap_or_else(|| DEFAULT_SCREENSHOT.into()));
        } else {
            model_path = arg;
        }
    }

    // El OBJ se lee una sola vez, antes del bucle principal.
    let model = Obj::load(&resolve_model_path(&model_path)).unwrap_or_else(|e| {
        eprintln!("{e}");
        process::exit(1);
    });
    println!(
        "Modelo cargado: {} vértices, {} triángulos",
        model.vertices.len(),
        model.faces.len()
    );

    // La proyección es estática: también se calcula una sola vez.
    let screen_vertices = project_vertices(&model.vertices).unwrap_or_else(|e| {
        eprintln!("{e}");
        process::exit(1);
    });

    let mut framebuffer = Framebuffer::new(SCREEN_WIDTH, SCREEN_HEIGHT);
    framebuffer.set_background_color(BACKGROUND);

    if let Some(path) = screenshot_path {
        draw_frame(&mut framebuffer, &model.faces, &screen_vertices);
        save_screenshot(&framebuffer, &path);
        return;
    }

    let mut window = Window::new(
        "Lab 4 - Nave OBJ",
        SCREEN_WIDTH,
        SCREEN_HEIGHT,
        WindowOptions::default(),
    )
    .unwrap_or_else(|e| {
        eprintln!("No se pudo crear la ventana: {e}");
        process::exit(1);
    });

    let frame_delay = Duration::from_millis(16);

    // minifb procesa los eventos (incluido cerrar la ventana) dentro de
    // update_with_buffer, que además presenta el framebuffer.
    while window.is_open() && !window.is_key_down(Key::Escape) {
        if window.is_key_pressed(Key::S, KeyRepeat::No) {
            save_screenshot(&framebuffer, DEFAULT_SCREENSHOT);
        }

        draw_frame(&mut framebuffer, &model.faces, &screen_vertices);

        window
            .update_with_buffer(&framebuffer.buffer, SCREEN_WIDTH, SCREEN_HEIGHT)
            .unwrap();

        std::thread::sleep(frame_delay);
    }
}
