use forge_math::{Aabb, Vec2, Vec3};
use std::collections::HashMap;

/// Mesh triangolata indicizzata. Le normali e le UV sono opzionali (vettori
/// vuoti se assenti); se presenti hanno la stessa lunghezza di `positions`.
#[derive(Clone, Debug, Default)]
pub struct Mesh {
    pub name: String,
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub uvs: Vec<Vec2>,
    pub indices: Vec<[u32; 3]>,
}

/// Esito del controllo di integrità di una mesh.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeshReport {
    pub triangles: usize,
    pub vertices: usize,
    pub degenerate_triangles: usize,
    pub invalid_indices: usize,
    pub non_finite_vertices: usize,
}

impl Mesh {
    pub fn triangle_count(&self) -> usize {
        self.indices.len()
    }

    pub fn has_normals(&self) -> bool {
        !self.normals.is_empty() && self.normals.len() == self.positions.len()
    }

    pub fn bounds(&self) -> Aabb {
        Aabb::from_points(self.positions.iter())
    }

    /// Controlla indici, vertici non finiti e triangoli degeneri.
    pub fn validate(&self) -> MeshReport {
        let n = self.positions.len() as u32;
        let mut r = MeshReport {
            triangles: self.indices.len(),
            vertices: self.positions.len(),
            ..Default::default()
        };
        r.non_finite_vertices = self.positions.iter().filter(|p| !p.is_finite()).count();
        for tri in &self.indices {
            if tri.iter().any(|&i| i >= n) {
                r.invalid_indices += 1;
                continue;
            }
            let [a, b, c] = tri.map(|i| self.positions[i as usize]);
            if (b - a).cross(c - a).length_squared() <= f32::MIN_POSITIVE {
                r.degenerate_triangles += 1;
            }
        }
        r
    }

    /// Rimuove triangoli con indici non validi o area nulla.
    pub fn remove_invalid_triangles(&mut self) -> usize {
        let n = self.positions.len() as u32;
        let before = self.indices.len();
        let pos = &self.positions;
        self.indices.retain(|tri| {
            if tri.iter().any(|&i| i >= n) {
                return false;
            }
            let [a, b, c] = tri.map(|i| pos[i as usize]);
            a.is_finite() && b.is_finite() && c.is_finite() && (b - a).cross(c - a).length_squared() > f32::MIN_POSITIVE
        });
        before - self.indices.len()
    }

    /// Scala tutte le posizioni (conversione di unità).
    pub fn scale(&mut self, s: f32) {
        for p in &mut self.positions {
            *p *= s;
        }
    }

    /// Calcola normali per vertice con un angolo di piega (crease angle):
    /// le facce che formano un angolo maggiore di `crease_deg` restano spigoli
    /// vivi, le altre vengono smussate. Indispensabile per mesh CAD/STL, dove
    /// superfici curve e spigoli netti convivono.
    ///
    /// I vertici vengono saldati per posizione e poi duplicati dove serve
    /// una discontinuità di normale.
    pub fn compute_normals(&mut self, crease_deg: f32) {
        let cos_crease = crease_deg.to_radians().cos();
        let face_normals: Vec<Vec3> = self
            .indices
            .iter()
            .map(|t| {
                let [a, b, c] = t.map(|i| self.positions[i as usize]);
                (b - a).cross(c - a).normalize_or_zero()
            })
            .collect();

        // Saldatura per posizione esatta (i formati CAD duplicano spesso i vertici).
        let mut weld: HashMap<[u32; 3], u32> = HashMap::new();
        let mut welded_id = Vec::with_capacity(self.positions.len());
        for p in &self.positions {
            let key = [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()];
            let next = weld.len() as u32;
            welded_id.push(*weld.entry(key).or_insert(next));
        }
        let mut incident: Vec<Vec<(u32, u8)>> = vec![Vec::new(); weld.len()];
        for (f, tri) in self.indices.iter().enumerate() {
            for (k, &v) in tri.iter().enumerate() {
                incident[welded_id[v as usize] as usize].push((f as u32, k as u8));
            }
        }

        let mut new_pos = Vec::with_capacity(self.positions.len());
        let mut new_nrm = Vec::with_capacity(self.positions.len());
        let mut new_uv = Vec::new();
        let has_uv = self.uvs.len() == self.positions.len();
        let mut dedup: HashMap<(u32, [u32; 3], [u32; 2]), u32> = HashMap::new();
        let mut new_indices = vec![[0u32; 3]; self.indices.len()];

        for (f, tri) in self.indices.iter().enumerate() {
            let nf = face_normals[f];
            for (k, &v) in tri.iter().enumerate() {
                let p = self.positions[v as usize];
                let own_angle = corner_angle(&self.positions, tri, k);
                let mut acc = nf * own_angle;
                for &(g, gk) in &incident[welded_id[v as usize] as usize] {
                    if g as usize == f {
                        continue;
                    }
                    let ng = face_normals[g as usize];
                    if nf.dot(ng) >= cos_crease {
                        let gt = &self.indices[g as usize];
                        acc += ng * corner_angle(&self.positions, gt, gk as usize);
                    }
                }
                let n = acc.normalize_or(nf);
                let uv = if has_uv { self.uvs[v as usize] } else { Vec2::ZERO };
                let key = (
                    welded_id[v as usize],
                    [n.x.to_bits(), n.y.to_bits(), n.z.to_bits()],
                    [uv.x.to_bits(), uv.y.to_bits()],
                );
                let idx = *dedup.entry(key).or_insert_with(|| {
                    new_pos.push(p);
                    new_nrm.push(n);
                    if has_uv {
                        new_uv.push(uv);
                    }
                    (new_pos.len() - 1) as u32
                });
                new_indices[f][k] = idx;
            }
        }
        self.positions = new_pos;
        self.normals = new_nrm;
        self.uvs = new_uv;
        self.indices = new_indices;
    }
}

fn corner_angle(pos: &[Vec3], tri: &[u32; 3], k: usize) -> f32 {
    let p = pos[tri[k] as usize];
    let a = (pos[tri[(k + 1) % 3] as usize] - p).normalize_or_zero();
    let b = (pos[tri[(k + 2) % 3] as usize] - p).normalize_or_zero();
    a.dot(b).clamp(-1.0, 1.0).acos()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives;

    #[test]
    fn cube_keeps_hard_edges() {
        let mut m = primitives::cuboid(Vec3::ONE);
        m.normals.clear();
        m.compute_normals(30.0);
        // Un cubo ha 6 facce * 4 vertici con normali distinte.
        assert_eq!(m.positions.len(), 24);
        for (i, tri) in m.indices.iter().enumerate() {
            let [a, b, c] = tri.map(|v| m.positions[v as usize]);
            let nf = (b - a).cross(c - a).normalize();
            for &v in tri {
                assert!(m.normals[v as usize].dot(nf) > 0.999, "faccia {i}");
            }
        }
    }

    #[test]
    fn sphere_gets_smoothed() {
        let mut m = primitives::uv_sphere(1.0, 32, 16);
        m.normals.clear();
        m.compute_normals(60.0);
        for (p, n) in m.positions.iter().zip(&m.normals) {
            assert!(p.normalize().dot(*n) > 0.99);
        }
    }

    #[test]
    fn validation_finds_problems() {
        let mut m = Mesh {
            positions: vec![Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::X * 2.0],
            indices: vec![[0, 1, 2], [0, 1, 3], [0, 1, 9]],
            ..Default::default()
        };
        let r = m.validate();
        assert_eq!(r.degenerate_triangles, 1);
        assert_eq!(r.invalid_indices, 1);
        assert_eq!(m.remove_invalid_triangles(), 2);
        assert_eq!(m.indices.len(), 1);
    }
}
