use std::fs;

use nalgebra_glm::Vec3;

/// Una cara triangular: tres índices (base 0) dentro del vector de vértices.
pub type Face = [usize; 3];

pub struct Obj {
    pub vertices: Vec<Vec3>,
    pub faces: Vec<Face>,
}

impl Obj {
    /// Lee un OBJ de triángulos línea por línea:
    ///   `v x y z` -> se agrega a `vertices`
    ///   `f i j k` -> se agrega a `faces` (índices convertidos a base 0)
    /// Las demás líneas (`#`, `o`, `s`, ...) se ignoran. Devuelve un error si el
    /// archivo no se puede abrir, si una línea está mal formada o si un índice
    /// queda fuera del vector de vértices.
    pub fn load(path: &str) -> Result<Obj, String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("No se pudo abrir {path}: {e}"))?;

        let mut vertices = Vec::new();
        // Las caras se validan al final, porque un OBJ puede declarar
        // vértices después de usarlos. Se guarda la línea para los errores.
        let mut raw_faces: Vec<([i64; 3], usize)> = Vec::new();

        for (number, line) in content.lines().enumerate() {
            let line_number = number + 1;
            let mut parts = line.split_whitespace();

            match parts.next() {
                Some("v") => {
                    let mut coords = [0.0f32; 3];
                    for coord in coords.iter_mut() {
                        *coord = parts
                            .next()
                            .and_then(|s| s.parse().ok())
                            .ok_or(format!("{path}:{line_number}: vértice inválido"))?;
                    }
                    vertices.push(Vec3::new(coords[0], coords[1], coords[2]));
                }
                Some("f") => {
                    let mut indices = [0i64; 3];
                    for index in indices.iter_mut() {
                        *index = parts.next().and_then(|s| s.parse().ok()).ok_or(format!(
                            "{path}:{line_number}: se esperaban tres índices en la cara"
                        ))?;
                    }
                    if parts.next().is_some() {
                        return Err(format!("{path}:{line_number}: la cara no es un triángulo"));
                    }
                    raw_faces.push((indices, line_number));
                }
                // '#', 'o', 's' y demás prefijos no se necesitan para este modelo.
                _ => {}
            }
        }

        let mut faces = Vec::with_capacity(raw_faces.len());
        for (indices, line_number) in raw_faces {
            let mut face: Face = [0; 3];
            for (slot, &index) in face.iter_mut().zip(indices.iter()) {
                // OBJ empieza en 1; Vec empieza en 0.
                if index < 1 || index as usize > vertices.len() {
                    return Err(format!(
                        "{path}:{line_number}: índice {index} fuera de rango"
                    ));
                }
                *slot = index as usize - 1;
            }
            faces.push(face);
        }

        Ok(Obj { vertices, faces })
    }
}
