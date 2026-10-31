//! Displacement vero: i triangoli dei componenti con una mappa height e
//! un'escursione > 0 vengono suddivisi e i vertici spostati lungo la normale.
//!
//! - Ogni lato riceve un numero di segmenti (potenza di 2) che dipende solo
//!   dalla sua lunghezza, quindi è lo stesso per i due triangoli che lo
//!   condividono; il triangolo usa il massimo dei suoi lati e i punti di bordo
//!   in più vengono interpolati sul lato più grossolano: niente crepe.
//! - I vertici coincidenti (saldati per posizione) si spostano tutti insieme
//!   lungo la normale media: niente crepe neanche sugli spigoli vivi.
//! - Con la proiezione box l'altezza di ogni punto è la media delle tre
//!   proiezioni sfumate secondo la direzione di spostamento (come le altre
//!   mappe): dipende solo da posizione e direzione, che sui vertici e sui lati
//!   condivisi sono le stesse per tutti i triangoli, quindi il campo è continuo.
//!   Ogni vertice porta negli UV l'altezza applicata e la direzione: lo shading
//!   ricostruisce la superficie originale e usa la stessa sfumatura.

use crate::texture::Texture;
use forge_math::{Mat3, Mat4, Vec2, Vec3};
use forge_scene::{Mesh, TextureMapping};
use std::collections::HashMap;
use std::sync::Arc;

/// Triangolo della superficie spostata con proiezione box: l'UV di ogni vertice contiene
/// l'altezza applicata (m) e la direzione dello spostamento ([`pack_dir`]).
pub const FLAG_DISPLACED_BOX: u32 = 1;

/// Direzione unitaria in un `f32` intero esatto (ottaedrica, 12 + 12 bit): passa intatta nei
/// buffer della GPU e si decodifica allo stesso modo nello shader.
pub fn pack_dir(d: Vec3) -> f32 {
    let s = d.x.abs() + d.y.abs() + d.z.abs();
    let n = if s > 0.0 { d / s } else { Vec3::Z };
    let sign = |v: f32| if v >= 0.0 { 1.0 } else { -1.0 };
    let (x, y) = if n.z >= 0.0 {
        (n.x, n.y)
    } else {
        ((1.0 - n.y.abs()) * sign(n.x), (1.0 - n.x.abs()) * sign(n.y))
    };
    let q = |v: f32| ((v.clamp(-1.0, 1.0) * 0.5 + 0.5) * 4095.0).round() as u32;
    (q(x) * 4096 + q(y)) as f32
}

/// Inverso di [`pack_dir`].
pub fn unpack_dir(v: f32) -> Vec3 {
    let c = v.max(0.0) as u32;
    let x = (c / 4096) as f32 / 4095.0 * 2.0 - 1.0;
    let y = (c % 4096) as f32 / 4095.0 * 2.0 - 1.0;
    let z = 1.0 - x.abs() - y.abs();
    let sign = |v: f32| if v >= 0.0 { 1.0 } else { -1.0 };
    let (x, y) = if z < 0.0 {
        ((1.0 - y.abs()) * sign(x), (1.0 - x.abs()) * sign(y))
    } else {
        (x, y)
    };
    Vec3::new(x, y, z).normalize_or(Vec3::Z)
}

/// Riferimento tangente (t, b) della proiezione box per l'asse indicato.
pub fn box_frame(axis: usize) -> (Vec3, Vec3) {
    match axis {
        0 => (Vec3::Z, Vec3::Y),
        1 => (Vec3::X, Vec3::Z),
        _ => (Vec3::X, Vec3::Y),
    }
}

/// Coordinate box di base sull'asse indicato (0 = X, 1 = Y, 2 = Z).
pub fn box_coords(p: Vec3, axis: usize) -> Vec2 {
    match axis {
        0 => Vec2::new(p.z, p.y),
        1 => Vec2::new(p.x, p.z),
        _ => Vec2::new(p.x, p.y),
    }
}

/// Esponente della sfumatura tra le tre proiezioni box: alto = passaggio stretto (niente
/// immagini sovrapposte sulle facce), ma senza il taglio netto da un triangolo all'altro.
pub const BOX_BLEND_POWER: i32 = 8;

/// Sotto questo peso una proiezione non si legge (quasi ovunque basta una lettura).
pub const BOX_WEIGHT_CUT: f32 = 0.01;

/// Pesi delle tre proiezioni box per una normale (nello spazio dell'oggetto). Ai pesi si toglie
/// [`BOX_WEIGHT_CUT`] (sotto zero diventano 0) e poi si rinormalizza: le proiezioni deboli
/// spariscono senza salti, perché il peso arriva a zero con continuità.
pub fn box_weights(n: Vec3) -> [f32; 3] {
    let a = n.abs();
    let mut w = [a.x, a.y, a.z].map(|v| v.powi(BOX_BLEND_POWER));
    let sum: f32 = w.iter().sum();
    if sum.is_nan() || sum <= 0.0 {
        return [0.0, 1.0, 0.0];
    }
    for v in &mut w {
        *v = (*v / sum - BOX_WEIGHT_CUT).max(0.0);
    }
    // Il peso maggiore è almeno 1/3: la somma resta positiva.
    let sum: f32 = w.iter().sum();
    w.map(|v| v / sum)
}

/// Riferimento dell'oggetto per la proiezione box: le texture seguono l'oggetto (assi e
/// posizione) invece del mondo, quindi non scivolano quando lo si sposta o lo si ruota.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxFrame {
    /// Rotazione oggetto → mondo.
    pub rot: forge_math::Quat,
    pub origin: Vec3,
    /// Coordinate per metro: 1 con la scala in metri; 1 / lato più lungo dell'oggetto con
    /// "Adatta all'oggetto" (la ripetizione segue la dimensione del pezzo).
    pub scale: f32,
}

impl Default for BoxFrame {
    fn default() -> Self {
        Self {
            rot: forge_math::Quat::IDENTITY,
            origin: Vec3::ZERO,
            scale: 1.0,
        }
    }
}

impl BoxFrame {
    /// Punto del mondo nello spazio dell'oggetto: in metri reali, oppure in frazioni
    /// dell'oggetto con "Adatta all'oggetto" (vedi [`BoxFrame::scale`]).
    #[inline]
    pub fn to_local(&self, p: Vec3) -> Vec3 {
        self.rot.inverse() * (p - self.origin) * self.scale
    }

    #[inline]
    pub fn dir_to_local(&self, d: Vec3) -> Vec3 {
        self.rot.inverse() * d
    }

    #[inline]
    pub fn dir_to_world(&self, d: Vec3) -> Vec3 {
        self.rot * d
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
    /// Metri di un'unità delle coordinate box: 1, oppure il lato dell'oggetto con "Adatta
    /// all'oggetto" (serve a stimare quanto fitta suddividere).
    pub box_unit_m: f32,
}

impl Params {
    /// Spostamento (m) con le coordinate UV della mesh.
    fn height_at(&self, base: Vec2) -> f32 {
        let uv = self.mapping.apply(base, false);
        (self.height.sample(uv).x - self.mid) * self.amount_m
    }

    /// Spostamento (m) con la proiezione box sfumata, nel punto `pos` del mondo con direzione
    /// di spostamento `dir`: le stesse letture pesate della mappa height nello shading.
    pub fn height_box(&self, frame: &BoxFrame, pos: Vec3, dir: Vec3) -> f32 {
        let pl = frame.to_local(pos);
        let w = box_weights(frame.dir_to_local(dir));
        let h: f32 = (0..3)
            .filter(|&k| w[k] > 0.0)
            .map(|k| self.height.sample(self.mapping.apply(box_coords(pl, k), true)).x * w[k])
            .sum();
        (h - self.mid) * self.amount_m
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
            self.mapping.scale.max(1e-6) * self.box_unit_m.max(1e-6)
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
/// Le coordinate box della superficie originale sono nello spazio dell'oggetto (`frame`).
pub fn tessellate(mesh: &Mesh, xf: &Mat4, mirrored: bool, params: &Params, target: f32, frame: &BoxFrame) -> Vec<OutTri> {
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

    // Vertici prodotti (condivisi tra triangoli dove serve), con altezza applicata e direzione
    // di spostamento di ognuno (per lo shading della proiezione box).
    let mut verts: Vec<Vec3> = Vec::new();
    let mut attrs: Vec<(f32, Vec3)> = Vec::new();
    let mut corner_cache: HashMap<u32, u32> = HashMap::new();
    let mut edge_cache: HashMap<(u32, u32, u32), u32> = HashMap::new();
    let mut tris: Vec<([u32; 3], [Vec2; 3])> = Vec::new();
    // Spostamento nel punto: UV della mesh, oppure proiezione box sfumata secondo la direzione
    // (dipende solo da posizione e direzione: uguale per tutti i triangoli che condividono il punto).
    let height = |pos: Vec3, dir: Vec3, uv: Vec2| {
        if has_uv {
            params.height_at(uv)
        } else {
            params.height_box(frame, pos, dir)
        }
    };
    let push = |verts: &mut Vec<Vec3>, attrs: &mut Vec<(f32, Vec3)>, pos: Vec3, dir: Vec3, uv: Vec2| -> u32 {
        let h = height(pos, dir, uv);
        verts.push(pos + dir * h);
        attrs.push((h, dir));
        verts.len() as u32 - 1
    };

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
        // UV della mesh (solo con la proiezione UV; con la box non servono).
        let base = if has_uv {
            idx.map(|k| mesh.uvs[k as usize])
        } else {
            [Vec2::ZERO; 3]
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
                let mut corner = |verts: &mut Vec<Vec3>, attrs: &mut Vec<(f32, Vec3)>, wv: u32| -> u32 {
                    *corner_cache
                        .entry(wv)
                        .or_insert_with(|| push(verts, attrs, wpos[wv as usize], wnorm[wv as usize], uv_of(wv, &w, &base)))
                };
                let id = match edge {
                    Some((u, _, 0)) => corner(&mut verts, &mut attrs, w[u]),
                    Some((_, v, s)) if s == n => corner(&mut verts, &mut attrs, w[v]),
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
                        let mut sample = |q: u32, verts: &mut Vec<Vec3>, attrs: &mut Vec<(f32, Vec3)>| -> u32 {
                            if q == 0 {
                                return corner(verts, attrs, wa);
                            }
                            if q == MAX_SEGMENTS {
                                return corner(verts, attrs, wb);
                            }
                            *edge_cache.entry((wa, wb, q)).or_insert_with(|| {
                                let t = q as f32 / MAX_SEGMENTS as f32;
                                let pos = wpos[wa as usize].lerp(wpos[wb as usize], t);
                                let (da, db) = edge_dirs.get(&(wa, wb)).copied().unwrap_or((fnrm, fnrm));
                                let dir = da.normalize_or(fnrm).lerp(db.normalize_or(fnrm), t).normalize_or(fnrm);
                                // UV dal triangolo corrente, nel verso originale del lato.
                                let uva = uv_of(wa, &w, &base);
                                let uvb = uv_of(wb, &w, &base);
                                push(verts, attrs, pos, dir, uva.lerp(uvb, t))
                            })
                        };
                        if q % step == 0 {
                            sample(q, &mut verts, &mut attrs)
                        } else {
                            // Punto in più rispetto al lato: sulla retta tra i due campioni del lato.
                            let (q0, q1) = (q / step * step, q / step * step + step);
                            let (v0, v1) = (sample(q0, &mut verts, &mut attrs), sample(q1, &mut verts, &mut attrs));
                            let t = (q - q0) as f32 / step as f32;
                            *edge_cache.entry((wa, wb, q)).or_insert_with(|| {
                                let (h0, d0) = attrs[v0 as usize];
                                let (h1, d1) = attrs[v1 as usize];
                                verts.push(verts[v0 as usize].lerp(verts[v1 as usize], t));
                                attrs.push((h0 + (h1 - h0) * t, d0.lerp(d1, t).normalize_or(d0)));
                                verts.len() as u32 - 1
                            })
                        }
                    }
                    None => {
                        let pos = p[0] * a + p[1] * b + p[2] * c;
                        let dir = (dirs[0] * a + dirs[1] * b + dirs[2] * c).normalize_or(fnrm);
                        push(&mut verts, &mut attrs, pos, dir, bary_uv(a, b, c))
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
                ));
                if i + j + 1 < n {
                    tris.push((
                        [grid[gi(i + 1, j)], grid[gi(i + 1, j + 1)], grid[gi(i, j + 1)]],
                        [uv_at(i + 1, j), uv_at(i + 1, j + 1), uv_at(i, j + 1)],
                    ));
                }
            }
        }
    }

    // Normali di shading della superficie spostata (media pesata per area).
    let mut vn = vec![Vec3::ZERO; verts.len()];
    for (t, _) in &tris {
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
        .map(|(t, uv)| {
            let p = t.map(|k| verts[k as usize]);
            let mut g = (p[1] - p[0]).cross(p[2] - p[0]);
            if mirrored {
                g = -g;
            }
            let g = g.normalize_or_zero();
            // Box: nell'UV di ogni vertice altezza applicata e direzione di spostamento.
            let (uv, flags) = if has_uv {
                (uv, 0)
            } else {
                (
                    t.map(|k| {
                        let (h, d) = attrs[k as usize];
                        Vec2::new(h, pack_dir(d))
                    }),
                    FLAG_DISPLACED_BOX,
                )
            };
            OutTri {
                p,
                n: t.map(|k| vn[k as usize].normalize_or(g)),
                uv,
                flags,
            }
        })
        .collect()
}

/// UV del triangolo corrente nel vertice saldato `wv`.
fn uv_of(wv: u32, w: &[u32; 3], base: &[Vec2; 3]) -> Vec2 {
    let i = w.iter().position(|x| *x == wv).unwrap_or(0);
    base[i]
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
            box_unit_m: 1.0,
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

    /// La direzione compressa torna uguale entro mezzo millesimo di radiante, in tutte le
    /// direzioni (anche sugli spigoli dell'ottaedro), e il valore è un intero esatto in f32.
    #[test]
    fn packed_directions_round_trip() {
        let mut worst = 0.0f32;
        for i in 0..64 {
            for j in 0..32 {
                let (th, ph) = (i as f32 / 64.0 * std::f32::consts::TAU, j as f32 / 31.0 * std::f32::consts::PI);
                let d = Vec3::new(ph.sin() * th.cos(), ph.cos(), ph.sin() * th.sin());
                let code = pack_dir(d);
                assert_eq!(code, code.round());
                assert!(code < 16_777_216.0);
                worst = worst.max(unpack_dir(code).angle_between(d.normalize()));
            }
        }
        for d in [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z, Vec3::new(1.0, -1.0, -1.0)] {
            worst = worst.max(unpack_dir(pack_dir(d)).angle_between(d.normalize()));
        }
        assert!(worst < 1e-3, "{worst}");
    }

    /// Ogni lato dei triangoli prodotti è condiviso da due triangoli (cubo chiuso → superficie chiusa).
    #[test]
    fn displaced_cube_stays_watertight() {
        let mesh = forge_scene::primitives::cuboid(Vec3::ONE);
        let p = params(noise_tex(), 0.05);
        // 0.08: lati del cubo a 16 segmenti, diagonali a 32 (punti di bordo interpolati).
        let out = tessellate(&mesh, &Mat4::IDENTITY, false, &p, 0.08, &BoxFrame::default());
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
        let out = tessellate(&mesh, &Mat4::IDENTITY, false, &params(white, 0.1), 0.2, &BoxFrame::default());
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
