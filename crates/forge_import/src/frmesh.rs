//! Formato intermedio `.frmesh` prodotto dai convertitori esterni (STEP, SolidWorks).
//!
//! Struttura: `FRMESH01` + lunghezza dell'header (u64 LE) + header JSON +
//! dati binari little-endian (per ogni mesh: posizioni f32×3, normali f32×3, indici u32×3).

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::Path;

pub const MAGIC: &[u8; 8] = b"FRMESH01";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeRecord {
    pub name: String,
    pub parent: Option<u32>,
    /// Matrice 4×4 column-major, unità del file.
    pub matrix: [f64; 16],
    pub color: Option<[f32; 3]>,
    pub mesh: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MeshRecord {
    pub vertex_count: u32,
    pub triangle_count: u32,
    /// Posizione dei dati nel blocco binario (byte).
    pub offset: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Header {
    /// Metri per unità del file (0.001 = millimetri).
    pub unit_m: f64,
    pub source: String,
    pub nodes: Vec<NodeRecord>,
    pub meshes: Vec<MeshRecord>,
}

/// Mesh decodificata.
#[derive(Clone, Debug, Default)]
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<[u32; 3]>,
}

#[derive(Clone, Debug, Default)]
pub struct FrMesh {
    pub header: Header,
    pub meshes: Vec<MeshData>,
}

impl FrMesh {
    pub fn write(&self, path: &Path) -> std::io::Result<()> {
        let mut header = self.header.clone();
        header.meshes.clear();
        let mut blob: Vec<u8> = Vec::new();
        for m in &self.meshes {
            header.meshes.push(MeshRecord {
                vertex_count: m.positions.len() as u32,
                triangle_count: m.indices.len() as u32,
                offset: blob.len() as u64,
            });
            for p in m.positions.iter().chain(&m.normals) {
                for v in p {
                    blob.extend_from_slice(&v.to_le_bytes());
                }
            }
            for t in &m.indices {
                for v in t {
                    blob.extend_from_slice(&v.to_le_bytes());
                }
            }
        }
        let json = serde_json::to_vec(&header).map_err(std::io::Error::other)?;
        let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
        f.write_all(MAGIC)?;
        f.write_all(&(json.len() as u64).to_le_bytes())?;
        f.write_all(&json)?;
        f.write_all(&blob)?;
        f.flush()
    }

    pub fn read(path: &Path) -> std::io::Result<Self> {
        let bad = |m: &str| std::io::Error::new(std::io::ErrorKind::InvalidData, m.to_string());
        let mut f = std::io::BufReader::new(std::fs::File::open(path)?);
        let mut magic = [0u8; 8];
        f.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(bad("file .frmesh non valido"));
        }
        let mut len = [0u8; 8];
        f.read_exact(&mut len)?;
        let len = u64::from_le_bytes(len) as usize;
        if len > 1 << 31 {
            return Err(bad("header .frmesh troppo grande"));
        }
        let mut json = vec![0u8; len];
        f.read_exact(&mut json)?;
        let header: Header = serde_json::from_slice(&json).map_err(|e| bad(&format!("header .frmesh: {e}")))?;
        let mut blob = Vec::new();
        f.read_to_end(&mut blob)?;
        let f32_at = |o: usize| f32::from_le_bytes(blob[o..o + 4].try_into().unwrap());
        let u32_at = |o: usize| u32::from_le_bytes(blob[o..o + 4].try_into().unwrap());
        let mut meshes = Vec::with_capacity(header.meshes.len());
        for m in &header.meshes {
            let (n, t, o) = (m.vertex_count as usize, m.triangle_count as usize, m.offset as usize);
            let end = o + n * 24 + t * 12;
            if end > blob.len() {
                return Err(bad("dati .frmesh troncati"));
            }
            let vec3 = |base: usize, i: usize| [f32_at(base + i * 12), f32_at(base + i * 12 + 4), f32_at(base + i * 12 + 8)];
            let positions = (0..n).map(|i| vec3(o, i)).collect();
            let normals = (0..n).map(|i| vec3(o + n * 12, i)).collect();
            let ib = o + n * 24;
            let indices: Vec<[u32; 3]> = (0..t)
                .map(|i| [u32_at(ib + i * 12), u32_at(ib + i * 12 + 4), u32_at(ib + i * 12 + 8)])
                .collect();
            if indices.iter().flatten().any(|&k| k as usize >= n) {
                return Err(bad("indici .frmesh fuori intervallo"));
            }
            meshes.push(MeshData {
                positions,
                normals,
                indices,
            });
        }
        for node in &header.nodes {
            if node.mesh.is_some_and(|m| m as usize >= meshes.len()) || node.parent.is_some_and(|p| p as usize >= header.nodes.len()) {
                return Err(bad("riferimenti .frmesh non validi"));
            }
        }
        Ok(Self { header, meshes })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let m = FrMesh {
            header: Header {
                unit_m: 0.001,
                source: "x.step".into(),
                nodes: vec![
                    NodeRecord {
                        name: "Assieme".into(),
                        parent: None,
                        matrix: identity(),
                        color: None,
                        mesh: None,
                    },
                    NodeRecord {
                        name: "Vite".into(),
                        parent: Some(0),
                        matrix: identity(),
                        color: Some([0.5, 0.2, 0.1]),
                        mesh: Some(0),
                    },
                ],
                meshes: vec![],
            },
            meshes: vec![MeshData {
                positions: vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                normals: vec![[0.0, 0.0, 1.0]; 3],
                indices: vec![[0, 1, 2]],
            }],
        };
        let p = std::env::temp_dir().join(format!("forge_frmesh_{}.frmesh", std::process::id()));
        m.write(&p).unwrap();
        let r = FrMesh::read(&p).unwrap();
        assert_eq!(r.header.nodes.len(), 2);
        assert_eq!(r.meshes[0].positions, m.meshes[0].positions);
        assert_eq!(r.meshes[0].indices, m.meshes[0].indices);
        std::fs::remove_file(p).ok();
    }

    pub fn identity() -> [f64; 16] {
        let mut m = [0.0; 16];
        m[0] = 1.0;
        m[5] = 1.0;
        m[10] = 1.0;
        m[15] = 1.0;
        m
    }
}
