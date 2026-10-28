//! Displacement vero: i triangoli dei componenti con una mappa height e
//! un'escursione > 0 vengono suddivisi e i vertici spostati lungo la normale.
//!
//! - Ogni lato riceve un numero di segmenti (potenza di 2) che dipende solo
//!   dalla sua lunghezza, quindi è lo stesso per i due triangoli che lo
//!   condividono; il triangolo usa il massimo dei suoi lati e i punti di bordo
//!   in più vengono interpolati sul lato più grossolano: niente crepe.
//! - I vertici coincidenti (saldati per posizione) si spostano tutti insieme
//!   lungo la normale media: niente crepe neanche sugli spigoli vivi.
//! - Le coordinate texture restano quelle della superficie originale: con la
//!   proiezione box le coordinate di base finiscono negli UV del triangolo e
//!   un flag dice a `coordinates` di usarle con l'asse indicato.

use crate::texture::Texture;
use forge_math::{Mat3, Mat4, Vec2, Vec3};
use forge_scene::{Mesh, TextureMapping};
use std::collections::HashMap;
use std::sync::Arc;

/// Gli UV del triangolo sono coordinate di proiezione box (in metri) lungo l'asse nei bit 1-2.
pub const FLAG_BOX_UV: u32 = 1;

pub fn box_flags(axis: usize) -> u32 {
    FLAG_BOX_UV | ((axis as u32) << 1)
}

/// Asse della proiezione box di un triangolo con flag [`FLAG_BOX_UV`].
pub fn box_axis(flags: u32) -> usize {
    ((flags >> 1) & 3) as usize
}

/// Coordinate box di base (prima del mapping) e asse dominante della normale.
pub fn box_base(p: Vec3, n: Vec3) -> (Vec2, usize) {
    let a = n.abs();
    if a.x >= a.y && a.x >= a.z {
        (Vec2::new(p.z, p.y), 0)
    } else if a.y >= a.z {
        (Vec2::new(p.x, p.z), 1)
    } else {
        (Vec2::new(p.x, p.y), 2)
    }
}

/// Riferimento tangente (t, b) della proiezione box per l'asse indicato.
pub fn box_frame(axis: usize) -> (Vec3, Vec3) {
    match axis {
        0 => (Vec3::Z, Vec3::Y),
        1 => (Vec3::X, Vec3::Z),
        _ => (Vec3::X, Vec3::Y),
    }
}

/// Massimo di triangoli prodotti dal displacement in tutta la scena.
pub const BUDGET: usize = 3_000_000;
/// Segmenti massimi per lato di un triangolo originale.
const MAX_SEGMENTS: u32 = 128;

pub struct Params {
    pub height: Arc<Texture>,
    pub mapping: TextureMapping,
    /// Escursione totale in metri.
    pub amount_m: f32,
    pub mid: f32,
    /// Coordinate dagli UV della mesh (altrimenti proiezione box).
    pub use_uv: bool,
}

impl Params {
    fn height_at(&self, base: Vec2) -> f32 {
        let uv = self.mapping.apply(base, !self.use_uv);
        (self.height.sample(uv).x - self.mid) * self.amount_m
    }

    /// Lato dei triangoli desiderato (metri): circa due texel e mezzo della mappa height.
    pub fn target_edge(&self, mesh: &Mesh, xf: &Mat4) -> f32 {
        let res = self.height.width.max(self.height.height).max(1) as f32;
        let m_per_uv = if self.use_uv {
            // Metri per unità UV stimati dal rapporto tra aree reali e aree UV.
            let (mut aw, mut au) = (0.0f64, 0.0f64);
            for idx in &mesh.indices {
                let p = idx.map(|k| xf.transform_point3(mesh.positions[k as usize]));
                let t = idx.map(|k| mesh.uvs[k as usize]);
                aw += (p[1] - p[0]).cross(p[2] - p[0]).length() as f64;
                let (d1, d2) = (t[1] - t[0], t[2] - t[0]);
                au += (d1.x * d2.y - d1.y * d2.x).abs() as f64;
            }
            if au > 0.0 {
                (aw / au).sqrt() as f32 / self.mapping.scale.max(1e-6)
            } else {
                1.0
            }
        } else {
            self.mapping.scale.max(1e-6)
        };
        (2.5 * m_per_uv / res).max(1e-5)
    }
}

fn segments(len: f32, target: f32) -> u32 {
    let n = (len / target).ceil().max(1.0) as u32;
    n.next_power_of_two().min(MAX_SEGMENTS)
}

/// Triangoli che produrrebbe la suddivisione con lato `target`.
pub fn estimate(mesh: &Mesh, xf: &Mat4, target: f32) -> usize {
    mesh.indices
        .iter()
        .map(|idx| {
            let p = idx.map(|k| xf.transform_point3(mesh.positions[k as usize]));
            let n = (0..3)
                .map(|e| segments((p[(e + 1) % 3] - p[e]).length(), target))
                .max()
                .unwrap_or(1) as usize;
            n * n
        })
        .sum()
}

/// Triangolo prodotto: vertici, normali di shading, UV (o coordinate box) e flag.
pub struct OutTri {
    pub p: [Vec3; 3],
    pub n: [Vec3; 3],
    pub uv: [Vec2; 3],
    pub flags: u32,
}

/// Suddivide e sposta la mesh (in spazio mondo).
pub fn tessellate(mesh: &Mesh, xf: &Mat4, mirrored: bool, params: &Params, target: f32) -> Vec<OutTri> {
    let nxf = Mat3::from_mat4(*xf).inverse().transpose();
    let wp: Vec<Vec3> = mesh.positions.iter().map(|p| xf.transform_point3(*p)).collect();
    let has_n = mesh.has_normals();
    let has_uv = params.use_uv && mesh.uvs.len() == mesh.positions.len();

    // Saldatura dei vertici coincidenti.
    let mut lo = Vec3::splat(f32::INFINITY);
    let mut hi = Vec3::splat(f32::NEG_INFINITY);
    for p in &wp {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    let eps = ((hi - lo).length() * 1e-6).max(1e-9);
    let mut weld_map: HashMap<[i64; 3], u32> = HashMap::new();
    let weld: Vec<u32> = wp
        .iter()
        .map(|p| {
            let k = [0, 1, 2].map(|i| (p[i] / eps).round() as i64);
            let n = weld_map.len() as u32;
            *weld_map.entry(k).or_insert(n)
        })
        .collect();
    let mut wpos = vec![Vec3::ZERO; weld_map.len()];
    let mut wnorm = vec![Vec3::ZERO; weld_map.len()];
    let face_n = |idx: &[u32; 3]| {
        let p = idx.map(|k| wp[k as usize]);
        let n = (p[1] - p[0]).cross(p[2] - p[0]);
        if mirrored {
            -n
        } else {
            n
        }
    };
    for idx in &mesh.indices {
        let n = face_n(idx); // pesata per area
        for &k in idx {
            let w = weld[k as usize] as usize;
            wpos[w] = wp[k as usize];
            wnorm[w] += n;
        }
    }
    for n in &mut wnorm {
        *n = n.normalize_or_zero();
    }

    // Vertici prodotti (condivisi tra triangoli dove serve) e normali accumulate.
    let mut verts: Vec<Vec3> = Vec::new();
    let mut corner_cache: HashMap<u32, u32> = HashMap::new();
    let mut edge_cache: HashMap<(u32, u32, u32), u32> = HashMap::new();
    let mut tris: Vec<([u32; 3], [Vec2; 3], u32)> = Vec::new();

    // Direzione di spostamento dei punti interni: normali della mesh o della faccia.
    let dirs_of = |idx: &[u32; 3], fnrm: Vec3| {
        if has_n {
            idx.map(|k| (nxf * mesh.normals[k as usize]).normalize_or(fnrm))
        } else {
            [fnrm; 3]
        }
    };
    // Lungo un lato si usa la media delle direzioni dei triangoli che lo condividono:
    // sui lati tra facce complanari resta la normale della faccia, sugli spigoli vivi
    // è la bisettrice, uguale per i due lati (niente crepe).
    let mut edge_dirs: HashMap<(u32, u32), (Vec3, Vec3)> = HashMap::new();
    for idx in &mesh.indices {
        let fnrm = face_n(idx).normalize_or_zero();
        if fnrm == Vec3::ZERO {
            continue;
        }
        let w = idx.map(|k| weld[k as usize]);
        let d = dirs_of(idx, fnrm);
        for e in 0..3 {
            let (a, b) = (e, (e + 1) % 3);
            let (wa, wb, da, db) = if w[a] < w[b] {
                (w[a], w[b], d[a], d[b])
            } else {
                (w[b], w[a], d[b], d[a])
            };
            let x = edge_dirs.entry((wa, wb)).or_default();
            x.0 += da;
            x.1 += db;
        }
    }

    for idx in &mesh.indices {
        let fnrm = face_n(idx).normalize_or_zero();
        if fnrm == Vec3::ZERO {
            continue;
        }
        let w = idx.map(|k| weld[k as usize]);
        if w[0] == w[1] || w[1] == w[2] || w[0] == w[2] {
            continue;
        }
        let p = idx.map(|k| wp[k as usize]);
        let dirs = dirs_of(idx, fnrm);
        let (base, flags) = if has_uv {
            (idx.map(|k| mesh.uvs[k as usize]), 0)
        } else {
            let axis = box_base(Vec3::ZERO, fnrm).1;
            (p.map(|v| box_base(v, fnrm).0), box_flags(axis))
        };
        let seg = [0, 1, 2].map(|e| segments((p[(e + 1) % 3] - p[e]).length(), target));
        let n = seg.iter().copied().max().unwrap_or(1);
        let bary_uv = |a: f32, b: f32, c: f32| base[0] * a + base[1] * b + base[2] * c;

        // Punto esatto su un lato (canonico: dal welded minore al maggiore, s in 1/MAX_SEGMENTS).
        let mut grid = vec![0u32; ((n + 1) * (n + 2) / 2) as usize];
        let gi = |i: u32, j: u32| (j * (n + 1) - j * (j.saturating_sub(1)) / 2 + i) as usize;
        for j in 0..=n {
            for i in 0..=(n - j) {
                let k = n - i - j;
                let (a, b, c) = (k as f32 / n as f32, i as f32 / n as f32, j as f32 / n as f32);
                // Lato su cui giace il punto (se di bordo): (vertice da, vertice a, passi da `da`).
                let edge = if j == 0 {
                    Some((0usize, 1usize, i))
                } else if i == 0 {
                    Some((0, 2, j))
                } else if k == 0 {
                    Some((1, 2, j))
                } else {
                    None
                };
                let id = match edge {
                    Some((u, _, 0)) => corner(&mut verts, &mut corner_cache, w[u], &wpos, &wnorm, params, base[u]),
                    Some((_, v, s)) if s == n => corner(&mut verts, &mut corner_cache, w[v], &wpos, &wnorm, params, base[v]),
                    Some((u, v, s)) => {
                        let e = if (u, v) == (0, 1) {
                            0
                        } else if (u, v) == (1, 2) {
                            1
                        } else {
                            2
                        };
                        let ne = seg[e];
                        // Parametro canonico in unità di 1/MAX_SEGMENTS.
                        let (wa, wb, mut q) = (w[u], w[v], s * (MAX_SEGMENTS / n));
                        let (wa, wb) = if wa < wb {
                            (wa, wb)
                        } else {
                            q = MAX_SEGMENTS - q;
                            (wb, wa)
                        };
                        let step = MAX_SEGMENTS / ne;
                        let mut sample = |q: u32, verts: &mut Vec<Vec3>| -> u32 {
                            if q == 0 {
                                return corner(verts, &mut corner_cache, wa, &wpos, &wnorm, params, uv_of(wa, &w, &base));
                            }
                            if q == MAX_SEGMENTS {
                                return corner(verts, &mut corner_cache, wb, &wpos, &wnorm, params, uv_of(wb, &w, &base));
                            }
                            *edge_cache.entry((wa, wb, q)).or_insert_with(|| {
                                let t = q as f32 / MAX_SEGMENTS as f32;
                                let pos = wpos[wa as usize].lerp(wpos[wb as usize], t);
                                let (da, db) = edge_dirs.get(&(wa, wb)).copied().unwrap_or((fnrm, fnrm));
                                let dir = da.normalize_or(fnrm).lerp(db.normalize_or(fnrm), t).normalize_or(fnrm);
                                // UV dal triangolo corrente, nel verso originale del lato.
                                let uva = uv_of(wa, &w, &base);
                                let uvb = uv_of(wb, &w, &base);
                                verts.push(pos + dir * params.height_at(uva.lerp(uvb, t)));
                                verts.len() as u32 - 1
                            })
                        };
                        if q % step == 0 {
                            sample(q, &mut verts)
                        } else {
                            // Punto in più rispetto al lato: sulla retta tra i due campioni del lato.
                            let (q0, q1) = (q / step * step, q / step * step + step);
                            let (v0, v1) = (sample(q0, &mut verts), sample(q1, &mut verts));
                            let t = (q - q0) as f32 / step as f32;
                            *edge_cache.entry((wa, wb, q)).or_insert_with(|| {
                                verts.push(verts[v0 as usize].lerp(verts[v1 as usize], t));
                                verts.len() as u32 - 1
                            })
                        }
                    }
                    None => {
                        let pos = p[0] * a + p[1] * b + p[2] * c;
                        let dir = (dirs[0] * a + dirs[1] * b + dirs[2] * c).normalize_or(fnrm);
                        verts.push(pos + dir * params.height_at(bary_uv(a, b, c)));
                        verts.len() as u32 - 1
                    }
                };
                grid[gi(i, j)] = id;
            }
        }
        let uv_at = |i: u32, j: u32| {
            let (b, c) = (i as f32 / n as f32, j as f32 / n as f32);
            bary_uv(1.0 - b - c, b, c)
        };
        for j in 0..n {
            for i in 0..(n - j) {
                tris.push((
                    [grid[gi(i, j)], grid[gi(i + 1, j)], grid[gi(i, j + 1)]],
                    [uv_at(i, j), uv_at(i + 1, j), uv_at(i, j + 1)],
                    flags,
                ));
                if i + j + 1 < n {
                    tris.push((
                        [grid[gi(i + 1, j)], grid[gi(i + 1, j + 1)], grid[gi(i, j + 1)]],
                        [uv_at(i + 1, j), uv_at(i + 1, j + 1), uv_at(i, j + 1)],
                        flags,
                    ));
                }
            }
        }
    }

    // Normali di shading della superficie spostata (media pesata per area).
    let mut vn = vec![Vec3::ZERO; verts.len()];
    for (t, ..) in &tris {
        let p = t.map(|k| verts[k as usize]);
        let mut n = (p[1] - p[0]).cross(p[2] - p[0]);
        if mirrored {
            n = -n;
        }
        for &k in t {
            vn[k as usize] += n;
        }
    }
    tris.into_iter()
        .map(|(t, uv, flags)| {
            let p = t.map(|k| verts[k as usize]);
            let mut g = (p[1] - p[0]).cross(p[2] - p[0]);
            if mirrored {
                g = -g;
            }
            let g = g.normalize_or_zero();
            OutTri {
                p,
                n: t.map(|k| vn[k as usize].normalize_or(g)),
                uv,
                flags,
            }
        })
        .collect()
}

/// UV (o coordinate box) del triangolo corrente nel vertice saldato `wv`.
fn uv_of(wv: u32, w: &[u32; 3], base: &[Vec2; 3]) -> Vec2 {
    let i = w.iter().position(|x| *x == wv).unwrap_or(0);
    base[i]
}

fn corner(verts: &mut Vec<Vec3>, cache: &mut HashMap<u32, u32>, wv: u32, wpos: &[Vec3], wnorm: &[Vec3], params: &Params, uv: Vec2) -> u32 {
    *cache.entry(wv).or_insert_with(|| {
        verts.push(wpos[wv as usize] + wnorm[wv as usize] * params.height_at(uv));
        verts.len() as u32 - 1
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(tex: Texture, amount: f32) -> Params {
        Params {
            height: Arc::new(tex),
            mapping: TextureMapping {
                scale: 0.25,
                ..Default::default()
            },
            amount_m: amount,
            mid: 0.0,
            use_uv: false,
        }
    }

    fn noise_tex() -> Texture {
        let n = 32;
        let data = (0..n * n)
            .map(|i| {
                let v = ((i * 7919 + (i / n) * 104729) % 251) as u8;
                [v, v, v]
            })
            .collect();
        Texture::from_rgb8(n, n, data, false)
    }

    /// Ogni lato dei triangoli prodotti è condiviso da due triangoli (cubo chiuso → superficie chiusa).
    #[test]
    fn displaced_cube_stays_watertight() {
        let mesh = forge_scene::primitives::cuboid(Vec3::ONE);
        let p = params(noise_tex(), 0.05);
        // 0.08: lati del cubo a 16 segmenti, diagonali a 32 (punti di bordo interpolati).
        let out = tessellate(&mesh, &Mat4::IDENTITY, false, &p, 0.08);
        assert!(out.len() > 12 * 64, "{} triangoli", out.len());
        let key = |v: Vec3| [v.x, v.y, v.z].map(|x| (x * 1e5).round() as i64);
        let mut edges: HashMap<([i64; 3], [i64; 3]), i32> = HashMap::new();
        for t in &out {
            for e in 0..3 {
                let (a, b) = (key(t.p[e]), key(t.p[(e + 1) % 3]));
                // Lato orientato: nella superficie chiusa compare una volta per verso.
                *edges.entry((a, b)).or_default() += 1;
                *edges.entry((b, a)).or_default() -= 1;
            }
        }
        let open = edges.values().filter(|c| **c != 0).count();
        assert_eq!(open, 0, "{open} lati aperti");
    }

    /// Height costante = 1 con mid 0: ogni faccia si sposta in fuori dell'escursione.
    #[test]
    fn constant_height_offsets_faces() {
        let mesh = forge_scene::primitives::cuboid(Vec3::ONE);
        let white = Texture::from_rgb8(1, 1, vec![[255; 3]], false);
        let out = tessellate(&mesh, &Mat4::IDENTITY, false, &params(white, 0.1), 0.2);
        // I centri delle facce (lontani dagli spigoli) stanno a 0.5 + 0.1.
        let far = out
            .iter()
            .flat_map(|t| t.p)
            .filter(|p| {
                let a = p.abs();
                let m = a.max_element();
                [a.x, a.y, a.z].iter().filter(|x| **x < m - 0.2).count() == 2
            })
            .map(|p| p.abs().max_element())
            .fold((f32::INFINITY, 0.0f32), |(lo, hi), v| (lo.min(v), hi.max(v)));
        assert!((far.0 - 0.6).abs() < 1e-4 && (far.1 - 0.6).abs() < 1e-4, "{far:?}");
    }
}
