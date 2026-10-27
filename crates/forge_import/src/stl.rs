use crate::{ImportError, ImportedObject};
use forge_math::Vec3;
use forge_scene::Mesh;
use std::path::Path;

pub(crate) fn read(path: &Path) -> Result<Vec<ImportedObject>, ImportError> {
    let mut f = std::io::BufReader::new(std::fs::File::open(path)?);
    let stl = stl_io::read_stl(&mut f).map_err(|e| ImportError::Parse(format!("STL: {e}")))?;
    let positions = stl.vertices.iter().map(|v| Vec3::new(v[0], v[1], v[2])).collect();
    let indices = stl
        .faces
        .iter()
        .map(|f| [f.vertices[0] as u32, f.vertices[1] as u32, f.vertices[2] as u32])
        .collect();
    let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("stl").to_string();
    // Le normali per faccia dell'STL sono spesso inaffidabili: si ricalcolano dal winding.
    Ok(vec![ImportedObject {
        name: name.clone(),
        mesh: Mesh {
            name,
            positions,
            normals: Vec::new(),
            uvs: Vec::new(),
            indices,
        },
        material: None,
    }])
}
