use nalgebra_glm::Vec2;

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
    /// Rellena UN triángulo, dado en coordenadas de pantalla, con el color
    /// actual. Los triángulos degenerados (proyectados sobre una línea o un
    /// punto) se descartan sin dividir entre cero.
    fn triangle(&mut self, a: &Vec2, b: &Vec2, c: &Vec2);
}

impl Triangle for Framebuffer {
    fn triangle(&mut self, a: &Vec2, b: &Vec2, c: &Vec2) {
        let v0 = *a;
        let (mut v1, mut v2) = (*b, *c);

        let area = edge(&v0, &v1, &v2);
        if area.abs() < 1e-6 {
            return; // Triángulo degenerado: no cubre área en pantalla.
        }
        if area < 0.0 {
            // Normalizar el orden para que el interior tenga funciones positivas.
            std::mem::swap(&mut v1, &mut v2);
        }

        // Caja envolvente recortada a los límites del framebuffer.
        let min_x = v0.x.min(v1.x).min(v2.x).floor().max(0.0) as usize;
        let min_y = v0.y.min(v1.y).min(v2.y).floor().max(0.0) as usize;
        let max_x = v0.x.max(v1.x).max(v2.x).ceil();
        let max_y = v0.y.max(v1.y).max(v2.y).ceil();
        if max_x < 0.0 || max_y < 0.0 {
            return;
        }
        let max_x = (max_x as usize).min(self.width.saturating_sub(1));
        let max_y = (max_y as usize).min(self.height.saturating_sub(1));

        let top_left0 = is_top_left(&v1, &v2);
        let top_left1 = is_top_left(&v2, &v0);
        let top_left2 = is_top_left(&v0, &v1);

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                // Se evalúa el centro del píxel.
                let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);

                let w0 = edge(&v1, &v2, &p);
                let w1 = edge(&v2, &v0, &p);
                let w2 = edge(&v0, &v1, &p);

                let inside0 = w0 > 0.0 || (w0 == 0.0 && top_left0);
                let inside1 = w1 > 0.0 || (w1 == 0.0 && top_left1);
                let inside2 = w2 > 0.0 || (w2 == 0.0 && top_left2);

                if inside0 && inside1 && inside2 {
                    self.point(x, y);
                }
            }
        }
    }
}
