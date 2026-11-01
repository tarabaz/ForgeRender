//! Luce laser: un fascio sottile il cui percorso si calcola in anticipo partendo dalla luce
//! (light tracing deterministico), invece di cercarlo dalla camera come le altre luci.
//!
//! Un path tracer che parte dalla camera non troverebbe quasi mai un fascio largo un millimetro
//! dopo un vetro (servirebbe colpire per caso il minuscolo emettitore): qui il fascio si segue dal
//! laser attraverso i vetri lisci (rifrazione, o riflessione totale) e gli specchi, fino alla
//! prima superficie opaca, dove illumina una macchia con profilo gaussiano. Nel render spettrale
//! il percorso si calcola per [`N_LAMBDA`] lunghezze d'onda con l'indice di rifrazione di ognuna
//! (dispersione): dopo un prisma o una gemma il fascio si apre a ventaglio.
//!
//! Il fascio visibile nell'aria è diffusione singola in una foschia uniforme dentro il cilindro
//! del fascio: integrale analitico lungo il raggio della camera, senza rumore.

use crate::spectral::{cauchy_ior, RgbSpectrum};
use crate::RenderScene;
use forge_math::{offset_ray_origin, Vec3};
use forge_scene::{Light, LightKind};

/// Lunghezze d'onda dei percorsi spettrali (da 380 a 780 nm).
pub const N_LAMBDA: usize = 64;
pub const LAMBDA_FIRST: f32 = 380.0;
pub const LAMBDA_LAST: f32 = 780.0;
/// Tratti al massimo per percorso.
pub const MAX_SEGMENTS: usize = 16;

/// Un tratto rettilineo del fascio.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LaserSegment {
    pub start: Vec3,
    pub end: Vec3,
    /// Potenza rimasta (RGB, o lo stesso valore nei tre canali per un percorso spettrale), come
    /// frazione di quella del laser.
    pub throughput: Vec3,
    /// Dentro un vetro (il fascio visibile conta solo nell'aria).
    pub inside: bool,
}

/// Dove il fascio arriva su una superficie opaca.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LaserSpot {
    pub point: Vec3,
    /// Direzione di propagazione dell'ultimo tratto.
    pub dir: Vec3,
    pub throughput: Vec3,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LaserPath {
    pub segments: Vec<LaserSegment>,
    pub spot: Option<LaserSpot>,
}

/// Laser pronto per il render.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderLaser {
    /// Potenza (colore × intensità), in RGB della scena.
    pub power: Vec3,
    /// Raggio del fascio (m).
    pub radius: f32,
    /// Densità della foschia per il fascio visibile (1/m, 0 = fascio invisibile).
    pub beam_density: f32,
    /// Percorso con l'indice di rifrazione di ogni materiale (render RGB).
    pub rgb: LaserPath,
    /// Percorsi alle lunghezze d'onda [`lambda_of`] (render spettrale).
    pub spectral: Vec<LaserPath>,
}

/// Lunghezza d'onda del percorso spettrale `k`.
pub fn lambda_of(k: usize) -> f32 {
    LAMBDA_FIRST + (LAMBDA_LAST - LAMBDA_FIRST) * k as f32 / (N_LAMBDA - 1) as f32
}

/// Percorso spettrale più vicino alla lunghezza d'onda `lambda`.
pub fn path_index(lambda: f32) -> usize {
    let k = (lambda - LAMBDA_FIRST) / (LAMBDA_LAST - LAMBDA_FIRST) * (N_LAMBDA - 1) as f32;
    (k.round().max(0.0) as usize).min(N_LAMBDA - 1)
}

/// I due percorsi della tabella attorno a `lambda` e il peso del secondo.
pub fn path_pair(lambda: f32) -> (usize, usize, f32) {
    let k = ((lambda - LAMBDA_FIRST) / (LAMBDA_LAST - LAMBDA_FIRST) * (N_LAMBDA - 1) as f32).clamp(0.0, (N_LAMBDA - 1) as f32);
    let k0 = (k.floor() as usize).min(N_LAMBDA - 2);
    (k0, k0 + 1, k - k0 as f32)
}

/// Stessa struttura (stesse superfici attraversate): si può interpolare tra i due percorsi.
pub fn compatible(a: &LaserPath, b: &LaserPath) -> bool {
    a.segments.len() == b.segments.len()
        && a.spot.is_some() == b.spot.is_some()
        && a.segments.iter().zip(&b.segments).all(|(x, y)| x.inside == y.inside)
}

/// Tratto interpolato (`t` = peso di `b`).
pub fn lerp_segment(a: &LaserSegment, b: &LaserSegment, t: f32) -> LaserSegment {
    LaserSegment {
        start: a.start.lerp(b.start, t),
        end: a.end.lerp(b.end, t),
        throughput: a.throughput.lerp(b.throughput, t),
        inside: a.inside,
    }
}

/// Macchia interpolata (`t` = peso di `b`).
pub fn lerp_spot(a: &LaserSpot, b: &LaserSpot, t: f32) -> LaserSpot {
    LaserSpot {
        point: a.point.lerp(b.point, t),
        dir: a.dir.lerp(b.dir, t).normalize_or(a.dir),
        throughput: a.throughput.lerp(b.throughput, t),
    }
}

/// Rugosità sotto cui un vetro o un metallo deviano il fascio senza disperderlo.
const SMOOTH: f32 = 0.1;

/// Segue il fascio da `origin` in direzione `dir`; `lambda` = None per il render RGB.
fn trace(scene: &RenderScene, origin: Vec3, dir: Vec3, lambda: Option<f32>) -> LaserPath {
    let geo = &*scene.geometry;
    let far = (geo.bounds.max - geo.bounds.min).length().max(1.0) * 4.0 + (origin - geo.bounds.center()).length();
    let mut path = LaserPath::default();
    let mut o = origin;
    let mut d = dir.normalize_or(-Vec3::Y);
    let mut tp = Vec3::ONE;
    let mut inside = false;
    let mut inside_mat: Option<usize> = None;
    let tint = |rgb: Vec3| match lambda {
        None => rgb,
        Some(l) => Vec3::splat(RgbSpectrum::albedo(rgb).eval(l)),
    };
    for _ in 0..MAX_SEGMENTS {
        let hit = if geo.accel.tris.is_empty() {
            None
        } else {
            geo.accel.intersect(o, d, 0.0, f32::INFINITY)
        };
        let Some(h) = hit else {
            path.segments.push(LaserSegment {
                start: o,
                end: o + d * far,
                throughput: tp,
                inside,
            });
            return path;
        };
        let prim = h.prim as usize;
        let p = geo.surface_point(prim, h.u, h.v);
        // Assorbimento del volume attraversato (Beer-Lambert).
        if let Some(mi) = inside_mat {
            let m = &scene.materials[mi];
            if m.volume_depth > 0.0 && m.volume_color != Vec3::ONE {
                let k = h.t / m.volume_depth;
                let c = tint(m.volume_color).max(Vec3::splat(1e-6));
                tp *= Vec3::new(c.x.powf(k), c.y.powf(k), c.z.powf(k));
            }
        }
        path.segments.push(LaserSegment {
            start: o,
            end: p,
            throughput: tp,
            inside,
        });
        let mi = scene.material_index(prim);
        let m = &scene.materials[mi];
        let mut ng = geo.geometric_normals[prim];
        let [n0, n1, n2] = geo.shading_normals[prim];
        let mut n = (n0 * (1.0 - h.u - h.v) + n1 * h.u + n2 * h.v).normalize_or(ng);
        let entering = ng.dot(d) < 0.0;
        if !entering {
            ng = -ng;
            n = -n;
        }
        if n.dot(ng) <= 0.0 {
            n = ng;
        }
        let cos_i = (-d).dot(n).clamp(0.0, 1.0);
        let reflect = |d: Vec3| d - 2.0 * d.dot(n) * n;
        if m.transmission >= 0.5 && m.roughness < SMOOTH {
            let ior = match lambda {
                Some(l) => cauchy_ior(m.ior, m.abbe, l),
                None => m.ior,
            };
            let eta = if entering { 1.0 / ior } else { ior };
            let sin2_t = eta * eta * (1.0 - cos_i * cos_i);
            if sin2_t >= 1.0 {
                // Riflessione totale interna: resta nel vetro.
                d = reflect(d).normalize();
                o = offset_ray_origin(p, ng);
            } else {
                let cos_t = (1.0 - sin2_t).sqrt();
                let rs = (cos_i - cos_t / eta) / (cos_i + cos_t / eta);
                let rp = (cos_i / eta - cos_t) / (cos_i / eta + cos_t);
                let fr = 0.5 * (rs * rs + rp * rp);
                tp *= (1.0 - fr) * m.transmission;
                if entering {
                    tp *= tint(m.base_color);
                }
                d = (d * eta + n * (eta * cos_i - cos_t)).normalize();
                o = offset_ray_origin(p, -ng);
                inside = entering;
                inside_mat = entering.then_some(mi);
            }
        } else if m.metallic >= 0.5 && m.roughness < SMOOTH {
            tp *= tint(m.base_color);
            d = reflect(d).normalize();
            o = offset_ray_origin(p, ng);
        } else {
            path.spot = Some(LaserSpot {
                point: p,
                dir: d,
                throughput: tp,
            });
            return path;
        }
        if tp.max_element() < 1e-4 {
            return path;
        }
    }
    path
}

/// Percorsi dei laser accesi della scena.
pub fn trace_lasers(scene: &RenderScene, lights: &[Light]) -> Vec<RenderLaser> {
    lights
        .iter()
        .filter(|l| l.enabled && l.intensity > 0.0)
        .filter_map(|l| {
            let LightKind::Laser { diameter_mm } = l.kind else { return None };
            let dir = l.direction();
            Some(RenderLaser {
                power: l.radiance_color(),
                radius: (diameter_mm * 0.5e-3).max(1e-5),
                beam_density: l.beam_density.max(0.0),
                rgb: trace(scene, l.position, dir, None),
                spectral: (0..N_LAMBDA).map(|k| trace(scene, l.position, dir, Some(lambda_of(k)))).collect(),
            })
        })
        .collect()
}

/// Irradianza gaussiana (TEM₀₀) a distanza `rho` dall'asse di un fascio di potenza 1 e raggio
/// `radius` (1/e² del profilo), sul piano perpendicolare al fascio.
#[inline]
pub fn gaussian_irradiance(rho: f32, radius: f32) -> f32 {
    2.0 / (std::f32::consts::PI * radius * radius) * (-2.0 * rho * rho / (radius * radius)).exp()
}

/// Macchia del fascio sul punto `p` (normale geometrica `ng`): direzione verso la luce e
/// irradianza relativa (× potenza × throughput), se `p` è nella macchia.
#[inline]
pub fn spot_at(spot: &LaserSpot, radius: f32, p: Vec3, ng: Vec3) -> Option<(Vec3, f32)> {
    let to = p - spot.point;
    // Solo vicino al punto d'arrivo (non lungo tutta la retta del fascio).
    if to.length_squared() > 16.0 * radius * radius + 1e-10 {
        return None;
    }
    let along = to.dot(spot.dir);
    let rho2 = (to.length_squared() - along * along).max(0.0);
    let cos = ng.dot(spot.dir).abs();
    let e = gaussian_irradiance(rho2.sqrt(), radius) * cos;
    (e > 0.0).then_some((-spot.dir, e))
}

/// Radianza del fascio visibile lungo il raggio della camera `o + t·d`, `t` in [0, `t_end`], per
/// un tratto (diffusione singola isotropa in una foschia di densità `density`), per potenza 1.
#[inline]
pub fn beam_radiance(seg: &LaserSegment, radius: f32, density: f32, o: Vec3, d: Vec3, t_end: f32) -> f32 {
    if seg.inside || density <= 0.0 {
        return 0.0;
    }
    let s = seg.end - seg.start;
    let len = s.length();
    if len <= 0.0 {
        return 0.0;
    }
    let s = s / len;
    let w0 = o - seg.start;
    let b = d.dot(s);
    let denom = 1.0 - b * b;
    if denom < 1e-6 {
        return 0.0; // guardando lungo il fascio: trascurato
    }
    let (dd, e) = (d.dot(w0), s.dot(w0));
    let t_ray = (b * e - dd) / denom;
    let t_seg = (e - b * dd) / denom;
    // Il punto più vicino deve stare sul tratto e prima della superficie che il raggio colpisce.
    let margin = 3.0 * radius;
    if t_ray < 0.0 || t_ray > t_end || t_seg < -margin || t_seg > len + margin {
        return 0.0;
    }
    let rho = ((o + d * t_ray) - (seg.start + s * t_seg)).length();
    if rho > 3.0 * radius {
        return 0.0;
    }
    let sin = denom.sqrt().max(0.02);
    // ∫ E dt lungo il raggio: profilo gaussiano attraversato con angolo φ.
    let line = gaussian_irradiance(rho, radius) * radius * (std::f32::consts::PI * 0.5).sqrt() / sin;
    density * line / (4.0 * std::f32::consts::PI)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gaussian_profile_carries_all_the_power() {
        let r = 0.002;
        let n = 2000;
        let rmax = 4.0 * r;
        let p: f32 = (0..n)
            .map(|i| {
                let rho = (i as f32 + 0.5) / n as f32 * rmax;
                gaussian_irradiance(rho, r) * 2.0 * std::f32::consts::PI * rho * rmax / n as f32
            })
            .sum();
        assert!((p - 1.0).abs() < 1e-3, "{p}");
    }

    #[test]
    fn lambda_table_round_trips() {
        for k in 0..N_LAMBDA {
            assert_eq!(path_index(lambda_of(k)), k);
        }
        assert_eq!(path_index(300.0), 0);
        assert_eq!(path_index(900.0), N_LAMBDA - 1);
    }
}
