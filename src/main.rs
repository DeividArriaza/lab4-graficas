mod color;
mod framebuffer;
mod obj;
mod png;
mod triangle;

use std::collections::BTreeSet;
use std::path::Path;
use std::process;
use std::time::Duration;

use minifb::{Key, KeyRepeat, Window, WindowOptions};
use nalgebra_glm::{self as glm, Mat4, Vec3};

use crate::color::Color;
use crate::framebuffer::Framebuffer;
use crate::obj::{Face, Material, Obj};
use crate::png::WritePng;
use crate::triangle::Triangle;

const SCREEN_WIDTH: usize = 800;
const SCREEN_HEIGHT: usize = 600;
const MARGIN: f32 = 0.8; // La nave ocupa como máximo el 80 % de la ventana.

const DEFAULT_MODEL: &str = "assets/nave.obj";
const DEFAULT_SCREENSHOT: &str = "screenshots/nave_renderizada.png";

// Paleta de horror cósmico. Los colores de cada parte de la nave vienen de
// assets/nave.mtl; aquí están el fondo, las aristas y la luz.
const VOID_CENTER: Vec3 = Vec3::new(0.11, 0.05, 0.16); // Púrpura del abismo.
const VOID_EDGE: Vec3 = Vec3::new(0.01, 0.01, 0.03); // Negro del vacío.
const STAR_COUNT: usize = 260;
const EDGE_COLOR: Color = Color::new(90, 220, 150); // Verde espectral.
// Material para caras sin `usemtl`.
const DEFAULT_DIFFUSE: Vec3 = Vec3::new(0.26, 0.21, 0.34);

// Vista inicial de tres cuartos, parecida a la de la captura de Blender.
const DEFAULT_YAW: f32 = 145.0;
const DEFAULT_PITCH: f32 = 35.0;
const ROTATION_STEP: f32 = 2.0; // Grados por cuadro al mantener una flecha.

// Iluminación difusa (Lambert) con una luz direccional fija respecto a la
// cámara: una luz fría y pálida, más una luz ambiente violeta tenue.
const AMBIENT: Vec3 = Vec3::new(0.30, 0.24, 0.40);
const LIGHT_COLOR: Vec3 = Vec3::new(0.85, 0.95, 0.90);
const EDGE_DEPTH_BIAS: f32 = 2.0;

/// Ángulos de la cámara orbital, en grados.
struct Camera {
    yaw: f32,
    pitch: f32,
}

/// Rota el modelo alrededor de su centro: primero gira sobre Y (yaw) y luego
/// se inclina sobre X (pitch) para verlo desde arriba. El resultado está en
/// espacio de vista: X a la derecha, Y hacia arriba y Z hacia la cámara.
fn to_view_space(vertices: &[Vec3], center: &Vec3, camera: &Camera) -> Vec<Vec3> {
    let identity = Mat4::identity();
    let rotation = glm::rotate_x(&identity, camera.pitch.to_radians())
        * glm::rotate_y(&identity, camera.yaw.to_radians());

    vertices
        .iter()
        .map(|v| (rotation * (v - center).push(1.0)).xyz())
        .collect()
}

/// Proyección ortográfica: centra y escala las posiciones ya rotadas para que
/// la nave ocupe el 80 % de la ventana. Devuelve X e Y en píxeles (origen
/// arriba a la izquierda) y Z como profundidad para el z-buffer.
fn to_screen_space(view: &[Vec3]) -> Result<Vec<Vec3>, String> {
    let first = view.first().ok_or("El modelo no tiene vértices")?;
    let mut min = *first;
    let mut max = *first;
    for v in view {
        min = glm::min2(&min, v);
        max = glm::max2(&max, v);
    }

    let center_x = (min.x + max.x) / 2.0;
    let center_y = (min.y + max.y) / 2.0;
    let width = max.x - min.x;
    let height = max.y - min.y;
    if width <= 0.0 || height <= 0.0 {
        return Err("El modelo no tiene extensión en pantalla; no se puede escalar".into());
    }

    let scale = (MARGIN * SCREEN_WIDTH as f32 / width).min(MARGIN * SCREEN_HEIGHT as f32 / height);

    Ok(view
        .iter()
        .map(|v| {
            Vec3::new(
                SCREEN_WIDTH as f32 / 2.0 + scale * (v.x - center_x),
                // La pantalla crece hacia abajo; el espacio de vista, hacia arriba.
                SCREEN_HEIGHT as f32 / 2.0 - scale * (v.y - center_y),
                scale * v.z,
            )
        })
        .collect())
}

/// Aristas únicas de la malla, para dibujar el alambre opcional.
fn unique_edges(faces: &[Face]) -> Vec<(usize, usize)> {
    let mut edges = BTreeSet::new();
    for face in faces {
        for k in 0..3 {
            let (i, j) = (face[k], face[(k + 1) % 3]);
            edges.insert((i.min(j), i.max(j)));
        }
    }
    edges.into_iter().collect()
}

/// Fracción de la luz direccional que recibe una cara (0 a 1) según el ángulo
/// entre su normal y la luz.
fn lambert(a: &Vec3, b: &Vec3, c: &Vec3, light_dir: &Vec3) -> f32 {
    let normal = (b - a).cross(&(c - a));
    let length = normal.norm();
    if length == 0.0 {
        return 0.0;
    }
    let mut normal = normal / length;
    // En proyección ortográfica las caras visibles miran hacia la cámara (+Z).
    // Orientar así la normal hace el sombreado independiente del orden de los
    // vértices; las caras traseras igual quedan tapadas por el z-buffer.
    if normal.z < 0.0 {
        normal = -normal;
    }
    normal.dot(light_dir).max(0.0)
}

/// Color final de una cara: difuso iluminado (ambiente + luz) más emisión.
fn face_color(material: Option<&Material>, light: f32) -> Color {
    let (diffuse, emission) = match material {
        Some(m) => (m.diffuse, m.emission),
        None => (DEFAULT_DIFFUSE, Vec3::zeros()),
    };
    let lighting = AMBIENT + LIGHT_COLOR * light;
    let color = diffuse.component_mul(&lighting) + emission;
    Color::from_unit(color.x, color.y, color.z)
}

/// Fondo de espacio profundo: degradado radial del púrpura al negro y
/// estrellas en posiciones pseudoaleatorias fijas. Se genera una sola vez.
fn space_background() -> Vec<u32> {
    let (w, h) = (SCREEN_WIDTH as f32, SCREEN_HEIGHT as f32);
    let max_distance = (w * w + h * h).sqrt() / 2.0;
    let mut image = Vec::with_capacity(SCREEN_WIDTH * SCREEN_HEIGHT);
    for y in 0..SCREEN_HEIGHT {
        for x in 0..SCREEN_WIDTH {
            let dx = x as f32 - w / 2.0;
            let dy = y as f32 - h / 2.0;
            let t = ((dx * dx + dy * dy).sqrt() / max_distance).min(1.0);
            let c = glm::lerp(&VOID_CENTER, &VOID_EDGE, t);
            image.push(Color::from_unit(c.x, c.y, c.z).to_hex());
        }
    }

    // Generador congruencial lineal: mismas estrellas en cada ejecución.
    let mut seed: u32 = 0x1A2B_3C4D;
    let mut next = || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        seed >> 8
    };
    for _ in 0..STAR_COUNT {
        let x = next() as usize % SCREEN_WIDTH;
        let y = next() as usize % SCREEN_HEIGHT;
        let brightness = 0.25 + (next() % 1000) as f32 / 1000.0 * 0.6;
        // Algunas estrellas con tinte verdoso o violeta.
        let star = match next() % 5 {
            0 => Color::from_unit(brightness * 0.6, brightness, brightness * 0.75),
            1 => Color::from_unit(brightness * 0.85, brightness * 0.6, brightness),
            _ => Color::from_unit(brightness, brightness, brightness),
        };
        image[y * SCREEN_WIDTH + x] = star.to_hex();
    }
    image
}

struct Scene {
    vertices: Vec<Vec3>,
    faces: Vec<Face>,
    face_materials: Vec<Option<usize>>,
    materials: Vec<Material>,
    edges: Vec<(usize, usize)>,
    center: Vec3,
    light_dir: Vec3,
}

/// Recorre manualmente todas las caras y rellena cada triángulo.
fn render(framebuffer: &mut Framebuffer, scene: &Scene, camera: &Camera, show_edges: bool) {
    let view = to_view_space(&scene.vertices, &scene.center, camera);
    let screen = match to_screen_space(&view) {
        Ok(screen) => screen,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };

    for (face, material) in scene.faces.iter().zip(&scene.face_materials) {
        let light = lambert(&view[face[0]], &view[face[1]], &view[face[2]], &scene.light_dir);
        let material = material.map(|index| &scene.materials[index]);
        framebuffer.set_current_color(face_color(material, light));

        let a = &screen[face[0]];
        let b = &screen[face[1]];
        let c = &screen[face[2]];
        framebuffer.triangle(a, b, c);
    }

    if show_edges {
        framebuffer.set_current_color(EDGE_COLOR);
        for &(i, j) in &scene.edges {
            framebuffer.line(&screen[i], &screen[j], EDGE_DEPTH_BIAS);
        }
    }
}

fn draw_frame(framebuffer: &mut Framebuffer, scene: &Scene, camera: &Camera, show_edges: bool) {
    framebuffer.clear();
    render(framebuffer, scene, camera, show_edges);
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
    let mut show_edges = false;
    let mut yaw = DEFAULT_YAW;
    let mut pitch = DEFAULT_PITCH;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--screenshot" => {
                screenshot_path = Some(args.next().unwrap_or_else(|| DEFAULT_SCREENSHOT.into()));
            }
            "--edges" => show_edges = true,
            "--yaw" | "--pitch" => {
                let value = args.next().and_then(|v| v.parse::<f32>().ok());
                let Some(value) = value else {
                    eprintln!("{arg} necesita un número de grados");
                    process::exit(1);
                };
                if arg == "--yaw" {
                    yaw = value;
                } else {
                    pitch = value.clamp(-89.0, 89.0);
                }
            }
            _ => model_path = arg,
        }
    }

    // El OBJ se lee una sola vez, antes del bucle principal.
    let model = Obj::load(&resolve_model_path(&model_path)).unwrap_or_else(|e| {
        eprintln!("{e}");
        process::exit(1);
    });
    println!(
        "Modelo cargado: {} vértices, {} triángulos, {} materiales",
        model.vertices.len(),
        model.faces.len(),
        model.materials.len()
    );
    if model.vertices.is_empty() {
        eprintln!("El modelo no tiene vértices");
        process::exit(1);
    }

    // Datos estáticos: centro del modelo (para rotar sobre sí mismo) y aristas.
    let mut min = model.vertices[0];
    let mut max = model.vertices[0];
    for v in &model.vertices {
        min = glm::min2(&min, v);
        max = glm::max2(&max, v);
    }
    let scene = Scene {
        edges: unique_edges(&model.faces),
        center: (min + max) / 2.0,
        light_dir: Vec3::new(-0.4, 0.75, 0.55).normalize(),
        vertices: model.vertices,
        faces: model.faces,
        face_materials: model.face_materials,
        materials: model.materials,
    };

    let mut camera = Camera { yaw, pitch };

    let mut framebuffer = Framebuffer::new(SCREEN_WIDTH, SCREEN_HEIGHT);
    framebuffer.set_background(space_background());

    if let Some(path) = screenshot_path {
        draw_frame(&mut framebuffer, &scene, &camera, show_edges);
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
        if window.is_key_down(Key::Left) {
            camera.yaw -= ROTATION_STEP;
        }
        if window.is_key_down(Key::Right) {
            camera.yaw += ROTATION_STEP;
        }
        if window.is_key_down(Key::Up) {
            camera.pitch = (camera.pitch + ROTATION_STEP).min(89.0);
        }
        if window.is_key_down(Key::Down) {
            camera.pitch = (camera.pitch - ROTATION_STEP).max(-89.0);
        }
        if window.is_key_pressed(Key::R, KeyRepeat::No) {
            camera = Camera { yaw: DEFAULT_YAW, pitch: DEFAULT_PITCH };
        }
        if window.is_key_pressed(Key::W, KeyRepeat::No) {
            show_edges = !show_edges;
        }
        if window.is_key_pressed(Key::S, KeyRepeat::No) {
            save_screenshot(&framebuffer, DEFAULT_SCREENSHOT);
        }

        draw_frame(&mut framebuffer, &scene, &camera, show_edges);

        window
            .update_with_buffer(&framebuffer.buffer, SCREEN_WIDTH, SCREEN_HEIGHT)
            .unwrap();

        std::thread::sleep(frame_delay);
    }
}
