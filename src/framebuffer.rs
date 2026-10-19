use crate::color::Color;

pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub buffer: Vec<u32>,
    // Profundidad del píxel más cercano dibujado hasta ahora. Valores mayores
    // están más cerca de la cámara.
    zbuffer: Vec<f32>,
    // Imagen de fondo que clear() copia en cada cuadro.
    background: Vec<u32>,
    current_color: u32,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Framebuffer {
            width,
            height,
            buffer: vec![0; width * height],
            zbuffer: vec![f32::NEG_INFINITY; width * height],
            background: vec![0; width * height],
            current_color: 0xFFFFFF,
        }
    }

    pub fn clear(&mut self) {
        self.buffer.copy_from_slice(&self.background);
        for depth in self.zbuffer.iter_mut() {
            *depth = f32::NEG_INFINITY;
        }
    }

    // Pinta el píxel solo si está más cerca que lo que ya hay en el z-buffer.
    pub fn point_with_depth(&mut self, x: usize, y: usize, depth: f32) {
        if x < self.width && y < self.height {
            let index = y * self.width + x;
            if depth > self.zbuffer[index] {
                self.zbuffer[index] = depth;
                self.buffer[index] = self.current_color;
            }
        }
    }

    /// Usa una imagen (0x00RRGGBB, del tamaño del framebuffer) como fondo.
    /// Si el tamaño no coincide, se conserva el fondo anterior.
    pub fn set_background(&mut self, image: Vec<u32>) {
        if image.len() == self.background.len() {
            self.background = image;
        }
    }

    pub fn set_current_color(&mut self, color: Color) {
        self.current_color = color.to_hex();
    }
}
