use std::fs;
use std::path::Path;

use nalgebra_glm::Vec3;

/// Una cara triangular: tres índices (base 0) dentro del vector de vértices.
pub type Face = [usize; 3];

/// Material de un archivo MTL. Solo se usan los colores base.
#[derive(Clone, Debug)]
pub struct Material {
    pub name: String,
    /// `Kd`: color difuso, el que se ilumina.
    pub diffuse: Vec3,
    /// `Ke`: emisión, color que se suma sin depender de la luz.
    pub emission: Vec3,
}

impl Material {
    fn new(name: &str) -> Self {
        Material {
            name: name.to_string(),
            diffuse: Vec3::new(0.8, 0.8, 0.8),
            emission: Vec3::zeros(),
        }
    }
}

pub struct Obj {
    pub vertices: Vec<Vec3>,
    pub faces: Vec<Face>,
    /// Material de cada cara (índice en `materials`), o `None` si la cara no
    /// tiene `usemtl` antes.
    pub face_materials: Vec<Option<usize>>,
    pub materials: Vec<Material>,
}

/// Lee los tres números flotantes que siguen al prefijo de una línea.
fn parse_vec3<'a>(
    parts: &mut impl Iterator<Item = &'a str>,
    error: impl Fn() -> String,
) -> Result<Vec3, String> {
    let mut values = [0.0f32; 3];
    for value in values.iter_mut() {
        *value = parts.next().and_then(|s| s.parse().ok()).ok_or_else(&error)?;
    }
    Ok(Vec3::new(values[0], values[1], values[2]))
}

/// Lee un archivo MTL: `newmtl`, `Kd` y `Ke`. El resto de propiedades se ignora.
fn load_mtl(path: &Path) -> Result<Vec<Material>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("No se pudo abrir {}: {e}", path.display()))?;

    let mut materials: Vec<Material> = Vec::new();
    for (number, line) in content.lines().enumerate() {
        let error = || format!("{}:{}: color inválido", path.display(), number + 1);
        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("newmtl") => {
                let name = parts.next().ok_or_else(|| {
                    format!("{}:{}: newmtl sin nombre", path.display(), number + 1)
                })?;
                materials.push(Material::new(name));
            }
            Some("Kd") => {
                if let Some(material) = materials.last_mut() {
                    material.diffuse = parse_vec3(&mut parts, error)?;
                }
            }
            Some("Ke") => {
                if let Some(material) = materials.last_mut() {
                    material.emission = parse_vec3(&mut parts, error)?;
                }
            }
            _ => {}
        }
    }
    Ok(materials)
}

impl Obj {
    /// Lee un OBJ de triángulos línea por línea:
    ///   `v x y z`     -> se agrega a `vertices`
    ///   `f i j k`     -> se agrega a `faces` (índices convertidos a base 0)
    ///   `mtllib a.mtl`-> carga los materiales (ruta relativa al OBJ)
    ///   `usemtl nom`  -> las caras siguientes usan ese material
    /// Las demás líneas (`#`, `o`, `s`, ...) se ignoran. Devuelve un error si un
    /// archivo no se puede abrir, si una línea está mal formada o si un índice
    /// queda fuera del vector de vértices.
    pub fn load(path: &str) -> Result<Obj, String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("No se pudo abrir {path}: {e}"))?;
        let base_dir = Path::new(path).parent().unwrap_or(Path::new("."));

        let mut vertices = Vec::new();
        let mut materials: Vec<Material> = Vec::new();
        let mut current_material: Option<usize> = None;
        // Las caras se validan al final, porque un OBJ puede declarar
        // vértices después de usarlos. Se guarda la línea para los errores.
        let mut raw_faces: Vec<([i64; 3], usize)> = Vec::new();
        let mut face_materials = Vec::new();

        for (number, line) in content.lines().enumerate() {
            let line_number = number + 1;
            let mut parts = line.split_whitespace();

            match parts.next() {
                Some("v") => {
                    let error = || format!("{path}:{line_number}: vértice inválido");
                    vertices.push(parse_vec3(&mut parts, error)?);
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
                    face_materials.push(current_material);
                }
                Some("mtllib") => {
                    if let Some(file) = parts.next() {
                        materials.extend(load_mtl(&base_dir.join(file))?);
                    }
                }
                Some("usemtl") => {
                    let name = parts.next().unwrap_or("");
                    current_material = materials.iter().position(|m| m.name == name);
                    if current_material.is_none() {
                        eprintln!("{path}:{line_number}: material '{name}' no definido");
                    }
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

        Ok(Obj { vertices, faces, face_materials, materials })
    }
}
