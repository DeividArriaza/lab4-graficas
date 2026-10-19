use nalgebra_glm::{Vec2, Vec3};

use crate::framebuffer::Framebuffer;

/// Doble del área con signo del triángulo (p0, p1, p). Es positiva si `p`
/// queda a un lado de la arista p0 -> p1 y negativa si queda al otro.
fn edge(p0: &Vec2, p1: &Vec2, p: &Vec2) -> f32 {
    (p1.x - p0.x) * (p.y - p0.y) - (p1.y - p0.y) * (p.x - p0.x)
}

/// Regla top-left: un píxel exactamente sobre una arista compartida se asigna
/// a un solo triángulo, así no quedan huecos ni píxeles pintados dos veces.
fn is_top_left(p0: &Vec2, p1: &Vec2) -> bool {
    let d = p1 - p0;
    (d.y == 0.0 && d.x > 0.0) || d.y < 0.0
}

pub trait Triangle {
    /// Rellena UN triángulo con el color actual. `x` e `y` de cada vértice
    /// están en píxeles y `z` es su profundidad (mayor = más cerca), que se
    /// interpola para el z-buffer. Los triángulos degenerados (proyectados
    /// sobre una línea o un punto) se descartan sin dividir entre cero.
    fn triangle(&mut self, a: &Vec3, b: &Vec3, c: &Vec3);

    /// Dibuja un segmento con el color actual, respetando el z-buffer.
    /// `bias` acerca la línea a la cámara para que no la tape su propia cara.
    fn line(&mut self, a: &Vec3, b: &Vec3, bias: f32);
}

impl Triangle for Framebuffer {
    fn triangle(&mut self, a: &Vec3, b: &Vec3, c: &Vec3) {
        let v0 = *a;
        let (mut v1, mut v2) = (*b, *c);

        let mut area = edge(&v0.xy(), &v1.xy(), &v2.xy());
        if area.abs() < 1e-6 {
            return; // Triángulo degenerado: no cubre área en pantalla.
        }
        if area < 0.0 {
            // Normalizar el orden para que el interior tenga funciones positivas.
            std::mem::swap(&mut v1, &mut v2);
            area = -area;
        }
        let (p0, p1, p2) = (v0.xy(), v1.xy(), v2.xy());

        // Caja envolvente recortada a los límites del framebuffer.
        let min_x = p0.x.min(p1.x).min(p2.x).floor().max(0.0) as usize;
        let min_y = p0.y.min(p1.y).min(p2.y).floor().max(0.0) as usize;
        let max_x = p0.x.max(p1.x).max(p2.x).ceil();
        let max_y = p0.y.max(p1.y).max(p2.y).ceil();
        if max_x < 0.0 || max_y < 0.0 {
            return;
        }
        let max_x = (max_x as usize).min(self.width.saturating_sub(1));
        let max_y = (max_y as usize).min(self.height.saturating_sub(1));

        let top_left0 = is_top_left(&p1, &p2);
        let top_left1 = is_top_left(&p2, &p0);
        let top_left2 = is_top_left(&p0, &p1);

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                // Se evalúa el centro del píxel.
                let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);

                let w0 = edge(&p1, &p2, &p);
                let w1 = edge(&p2, &p0, &p);
                let w2 = edge(&p0, &p1, &p);

                let inside0 = w0 > 0.0 || (w0 == 0.0 && top_left0);
                let inside1 = w1 > 0.0 || (w1 == 0.0 && top_left1);
                let inside2 = w2 > 0.0 || (w2 == 0.0 && top_left2);

                if inside0 && inside1 && inside2 {
                    // Coordenadas baricéntricas para interpolar la profundidad.
                    let depth = (w0 * v0.z + w1 * v1.z + w2 * v2.z) / area;
                    self.point_with_depth(x, y, depth);
                }
            }
        }
    }

    fn line(&mut self, a: &Vec3, b: &Vec3, bias: f32) {
        let delta = b - a;
        let steps = delta.x.abs().max(delta.y.abs()).ceil() as usize;
        if steps == 0 {
            if a.x >= 0.0 && a.y >= 0.0 {
                self.point_with_depth(a.x as usize, a.y as usize, a.z + bias);
            }
            return;
        }
        for i in 0..=steps {
            let p = a + delta * (i as f32 / steps as f32);
            if p.x >= 0.0 && p.y >= 0.0 {
                self.point_with_depth(p.x as usize, p.y as usize, p.z + bias);
            }
        }
    }
}
