use crate::{ImportError, ImportedObject};
use forge_math::{Vec2, Vec3};
use forge_scene::{material::srgb_to_linear, Material, MaterialId, Mesh};
use std::path::Path;

pub(crate) fn read(path: &Path, warnings: &mut Vec<String>) -> Result<Vec<ImportedObject>, ImportError> {
    let opts = tobj::LoadOptions {
        triangulate: true,
        single_index: true,
        ignore_points: true,
        ignore_lines: true,
    };
    let (models, mats) = tobj::load_obj(path, &opts).map_err(|e| ImportError::Parse(format!("OBJ: {e}")))?;
    let mats = match mats {
        Ok(m) => m,
        Err(e) => {
            warnings.push(format!("libreria materiali .mtl non caricata: {e}"));
            Vec::new()
        }
    };
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("obj");
    let mut out = Vec::with_capacity(models.len());
    for (i, model) in models.into_iter().enumerate() {
        let m = model.mesh;
        let positions: Vec<Vec3> = m.positions.chunks_exact(3).map(|c| Vec3::new(c[0], c[1], c[2])).collect();
        let normals: Vec<Vec3> = if m.normals.len() == m.positions.len() {
            m.normals.chunks_exact(3).map(|c| Vec3::new(c[0], c[1], c[2])).collect()
        } else {
            Vec::new()
        };
        let uvs: Vec<Vec2> = if m.texcoords.len() / 2 == positions.len() {
            m.texcoords.chunks_exact(2).map(|c| Vec2::new(c[0], c[1])).collect()
        } else {
            Vec::new()
        };
        let indices = m.indices.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();
        let name = if model.name.trim().is_empty() {
            format!("oggetto-{}", i + 1)
        } else {
            model.name.clone()
        };
        let material = m.material_id.and_then(|mi| mats.get(mi)).map(|mtl| {
            let kd = mtl.diffuse.unwrap_or([0.8, 0.8, 0.8]);
            Material {
                id: MaterialId::new(format!("obj:{stem}:{}", mtl.name)),
                name: mtl.name.clone(),
                // I colori .mtl sono di fatto in sRGB.
                base_color: kd.map(srgb_to_linear),
                roughness: mtl
                    .shininess
                    .map(|ns| (2.0 / (ns + 2.0)).sqrt().sqrt().clamp(0.05, 1.0))
                    .unwrap_or(0.5),
                category: "Importati".into(),
                ..Default::default()
            }
        });
        out.push(ImportedObject {
            name,
            mesh: Mesh {
                name: model.name,
                positions,
                normals,
                uvs,
                indices,
            },
            material,
        });
    }
    Ok(out)
}
