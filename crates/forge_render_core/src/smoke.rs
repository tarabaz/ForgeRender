//! Fumo volumetrico ([`forge_scene::SmokeCloud`]): il campo di densità procedurale e il suo
//! calcolo su griglia ([`BakedFog`]), con cache.
//!
//! La densità nasce da tre strati, tutti nelle coordinate del riquadro (spostare il riquadro
//! sposta la nube identica, senza residui dove era prima):
//! 1. **forma di massima** (nube, pennacchio, filo d'incenso, banco): un valore che vale circa 1
//!    nel cuore e 0 sul bordo, valutato in un punto *distorto* dai vortici, così il contorno
//!    si attorciglia invece di essere una sfera o un cono;
//! 2. **bordo**: sbuffi "a cavolfiore" (frattale di |rumore|, lobi tondi con pieghe nette) e
//!    filamenti (creste del rumore) che sfilacciano la parte esterna;
//! 3. **interno**: variazione lenta della densità, così la nube non è un blocco uniforme.
//!
//! Il fumo è un mezzo fisico (diffonde, assorbe, fa ombra) e non entra negli oggetti.

use crate::fog::{interior_cells, voxelize};
use crate::RenderGeometry;
use forge_math::Vec3;
use forge_scene::{Scene, SmokeCloud, SmokeShape};
use forge_volume::noise::{fbm, fbm_gain, gradient};
use forge_volume::{progress, BakedFog, GridSpec};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Frattale "a sbuffi": somma di 2|rumore| − 1, lobi tondi separati da pieghe nette
/// (l'aspetto a cavolfiore del fumo denso), in circa [−1, 1].
fn billow_fbm(p: Vec3, seed: u32, octaves: u32, gain: f32) -> f32 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..octaves {
        let g = gradient(p * freq + Vec3::splat(o as f32 * 13.7), seed.wrapping_add(o * 7919));
        sum += amp * (2.0 * g.abs() * 1.6 - 1.0);
        norm += amp;
        amp *= gain;
        freq *= 2.0;
    }
    sum / norm
}

/// Distorsione del dominio (vortici), ampiezza tipica ~1.
fn warp(q: Vec3, s: u32) -> Vec3 {
    Vec3::new(
        fbm(q + Vec3::new(5.2, 1.3, 9.1), s ^ 0x68e3_1da4, 3),
        fbm(q + Vec3::new(-7.7, 3.9, 2.4), s ^ 0x1b56_c4e9, 3),
        fbm(q + Vec3::new(1.7, -9.2, 6.3), s ^ 0x2f8a_7d11, 3),
    ) * 2.2
}

/// Pennacchio e incenso: posizione relativa all'asse che serpeggia salendo, larghezza alla quota
/// `t` (0 in fondo, 1 in cima) e quanto il moto è già turbolento a quella quota.
fn column(s: &SmokeCloud, u: Vec3, t: f32, thin: bool) -> (f32, f32) {
    let (r0, grow) = if thin { (0.07, 0.75) } else { (0.16, 1.0) };
    let width = r0 + (1.0 - r0) * s.spread * grow * t.powf(0.65);
    let seed = s.seed;
    let m = s.meander * t.powf(1.2) * 0.55;
    let ax = fbm(Vec3::new(t * 2.2, 0.3, 1.7), seed ^ 0x51ed_270b, 3) * m * 1.6;
    let az = fbm(Vec3::new(t * 2.2, 4.1, -2.3), seed ^ 0x7f4a_7c15, 3) * m * 1.6;
    let d = Vec3::new(u.x - ax, 0.0, u.z - az).length() / width.max(1e-3);
    // In fondo nasce dalla bocca (niente fumo sotto), in cima si chiude piano.
    ((1.0 - d).min(t * 10.0).min((1.0 - t) * 6.0), width)
}

/// Densità del fumo nel punto `p` (1/m).
pub fn smoke_density_at(s: &SmokeCloud, p: Vec3) -> f32 {
    let (lo, hi) = s.bounds();
    let half = (hi - lo) * 0.5;
    let rel = p - s.center;
    let u = rel / half.max(Vec3::splat(1e-4));
    // Fuori dal riquadro (anche con i vortici) niente fumo.
    let box_edge = 1.0 - u.abs().max_element();
    if box_edge <= 0.0 {
        return 0.0;
    }
    // Quote di partenza e di arrivo, misurate dal fondo del riquadro.
    let y = p.y - lo.y;
    let size_y = hi.y - lo.y;
    let fw = (0.05 * size_y).max(0.005);
    let mut cut = 1.0;
    if s.bottom_height > 0.0 {
        cut *= smoothstep(s.bottom_height, s.bottom_height + fw, y);
    }
    if s.top_height > 0.0 {
        cut *= 1.0 - smoothstep(s.top_height - fw, s.top_height, y);
    }
    if cut <= 0.0 {
        return 0.0;
    }
    let l = s.detail_scale.max(0.002);
    let seed = s.seed;
    let q = rel / l + Vec3::new(17.0, -3.0, 9.0) * (seed % 97) as f32;
    let t = (u.y + 1.0) * 0.5;
    // Turbolenza: il pennacchio e soprattutto l'incenso nascono lisci e si rompono salendo.
    let laminar = match s.shape {
        SmokeShape::Plume => 0.25 + 0.75 * t.powf(0.8),
        SmokeShape::Wisp => 0.08 + 0.92 * t.powf(1.6),
        _ => 1.0,
    };
    let turb = s.turbulence * laminar;
    // Uscita veloce dove la forma di massima è lontana (oltre il massimo spostamento dei vortici).
    let w = warp(q, seed);
    let disp = w * (turb * 0.45 * l);
    let up = (rel + disp) / half.max(Vec3::splat(1e-4));
    let tp = ((up.y + 1.0) * 0.5).clamp(0.0, 1.0);
    // Forma e larghezza del pezzo (frazione del riquadro: il filo d'incenso è sottile).
    let (base, width) = match s.shape {
        SmokeShape::Cloud => (1.0 - up.length(), 1.0),
        SmokeShape::Bank => {
            let a = up.abs();
            (1.0 - (a.x.powi(6) + a.y.powi(6) + a.z.powi(6)).powf(1.0 / 6.0), 1.0)
        }
        SmokeShape::Plume => column(s, up, tp, false),
        SmokeShape::Wisp => column(s, up, tp, true),
    };
    // Spessore del bordo in unità della forma: dettagli più grandi = bordo più frastagliato; mai
    // tanto da spezzare un pezzo sottile.
    let feature = (half.x.min(half.z) * width).max(1e-4);
    let rel_l = (l / feature).clamp(0.02, 0.4);
    if base < -1.2 * rel_l - 0.1 {
        return 0.0;
    }
    let qd = q + w * turb * 0.6;
    let oct = s.octaves.clamp(1, 10);
    let gain = s.roughness.clamp(0.2, 0.85);
    // Bordo: sbuffi a cavolfiore e rumore frattale.
    let puffs = billow_fbm(qd * 0.8, seed ^ 0x3c6e_f372, oct, gain);
    let grain = fbm_gain(qd, seed, oct, gain);
    let bump = (s.billows * 0.9 * puffs + (1.0 - s.billows) * 0.5 * grain) * rel_l * 1.4;
    // Filamenti: creste sottili del rumore, che portano il fumo anche poco fuori dal bordo.
    let ridge = (1.0 - fbm_gain(qd * 2.3 + Vec3::splat(4.7), seed ^ 0x9e37_79b9, oct.min(6), gain).abs() * 2.2)
        .max(0.0)
        .powi(4);
    let edge = base + bump + s.wisps * ridge * rel_l * 0.5;
    let soft = 0.02 + 0.45 * s.softness * rel_l.max(0.15);
    let mut cov = smoothstep(0.0, soft, edge);
    if cov <= 0.0 {
        return 0.0;
    }
    // Vicino al bordo la nube si sfilaccia in filamenti (il cuore resta pieno).
    let rim = 1.0 - smoothstep(0.0, (4.0 * soft + 0.25 * rel_l).min(0.45), edge);
    cov *= 1.0 - s.wisps * rim * (1.0 - (ridge * 1.6).min(1.0));
    // Interno: variazione lenta (media ≈ 1).
    let v = fbm(qd * 0.45 + Vec3::splat(-6.1), seed ^ 0x2545_f491, 3);
    let inner = (1.0 - s.variation) + s.variation * 2.0 * smoothstep(-0.55, 0.65, v);
    // Pennacchio e incenso si diradano salendo.
    let fade = match s.shape {
        SmokeShape::Plume | SmokeShape::Wisp => (1.0 - s.fade * t.powf(0.8)).max(0.0).powf(1.5),
        _ => 1.0,
    };
    // Mai un taglio netto sulle facce del riquadro.
    let walls = smoothstep(0.0, 0.08, box_edge);
    s.density * cov * inner * fade * walls * cut
}

/// Qualità: (celle sul lato più lungo, celle al massimo).
const SMOKE_QUALITY: [(usize, f32); 4] = [(128, 3.0e6), (192, 7.0e6), (288, 14.0e6), (400, 24.0e6)];

type Cache = Mutex<HashMap<String, Arc<BakedFog>>>;

fn cache() -> &'static Cache {
    static C: OnceLock<Cache> = OnceLock::new();
    C.get_or_init(Default::default)
}

fn active(scene: &Scene) -> impl Iterator<Item = SmokeCloud> + '_ {
    scene.smokes.iter().filter(|s| s.enabled && s.density > 0.0).map(|s| s.sanitized())
}

fn key(s: &SmokeCloud, geo: &RenderGeometry) -> String {
    let k = SmokeCloud {
        name: String::new(),
        n: 0,
        ..s.clone()
    };
    // Con le collisioni contano gli oggetti (qualsiasi cambio di geometria: semplice e sicuro).
    format!("{k:?}|{}", if s.collisions { geo.id } else { 0 })
}

/// Numero di nubi di fumo accese.
pub fn smoke_count(scene: &Scene) -> usize {
    active(scene).count()
}

/// Nubi di fumo accese già calcolate?
pub fn smokes_ready(scene: &Scene, geo: &RenderGeometry) -> bool {
    let c = cache().lock().unwrap();
    active(scene).all(|s| c.contains_key(&key(&s, geo)))
}

/// Una nube calcolata (dalla cache o calcolata qui, con l'avanzamento sulla barra); None se il
/// calcolo è stato annullato.
fn bake_one(s: &SmokeCloud, geo: &RenderGeometry) -> Option<Arc<BakedFog>> {
    let k = key(s, geo);
    if let Some(b) = cache().lock().unwrap().get(&k) {
        return Some(b.clone());
    }
    let (lo, hi) = s.bounds();
    let ext = hi - lo;
    let (side, max_cells) = SMOKE_QUALITY[(s.quality.clamp(1, 4) - 1) as usize];
    let mut longest = side;
    loop {
        let cell = ext.max_element() / longest as f32;
        if (ext.x / cell).ceil() * (ext.y / cell).ceil() * (ext.z / cell).ceil() <= max_cells || longest <= 16 {
            break;
        }
        longest = (longest as f32 * 0.92) as usize;
    }
    let spec = GridSpec::covering(lo, hi, longest);
    progress::update(0.0);
    let inside = if s.collisions && !geo.accel.is_empty() {
        let shell = voxelize(geo, &spec);
        progress::update(0.1);
        interior_cells(geo, &spec, &shell)
    } else {
        Vec::new()
    };
    progress::update(0.15);
    let done = std::sync::atomic::AtomicUsize::new(0);
    let n = spec.len().max(1);
    let sc = s.clone();
    let albedo = Vec3::from(s.color).clamp(Vec3::ZERO, Vec3::splat(0.999));
    let fog = BakedFog::from_fn(lo, hi, longest, albedo, s.anisotropy, |p| {
        let i = done.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if i.is_multiple_of(32_768) {
            progress::update(0.15 + 0.85 * i as f32 / n as f32);
        }
        if !inside.is_empty() {
            if let Some((x, y, z)) = spec.cell_of(p) {
                if inside[spec.index(x, y, z)] {
                    return 0.0;
                }
            }
        }
        smoke_density_at(&sc, p)
    });
    if progress::cancelled() {
        return None;
    }
    tracing::info!("fumo «{}»: {:?} celle in {:.0} ms", s.name, fog.spec.res, fog.bake_ms);
    let b = Arc::new(fog);
    let mut c = cache().lock().unwrap();
    if c.len() > 16 {
        c.clear();
    }
    c.insert(k, b.clone());
    Some(b)
}

/// Calcola le nubi accese, una alla volta sulla barra degli effetti: `first` è il numero della
/// prima su `total` effetti. Si ferma se il calcolo viene annullato.
pub fn bake_smokes(scene: &Scene, geo: &RenderGeometry, first: usize, total: usize) {
    for (k, s) in active(scene).enumerate() {
        if progress::cancelled() {
            return;
        }
        progress::begin(progress::Phase::Smoke, first + k, total);
        bake_one(&s, geo);
    }
}

/// Nubi di fumo accese per il renderer (dalla cache, o calcolate qui).
pub fn smoke_fogs(scene: &Scene, geo: &RenderGeometry) -> Vec<Arc<BakedFog>> {
    active(scene).filter_map(|s| bake_one(&s, geo)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nube sospesa: piena al centro, vuota fuori dal riquadro e sugli angoli, con il bordo
    /// irregolare (sbuffi), e spostando il riquadro la nube si sposta identica.
    #[test]
    fn smoke_cloud_has_a_core_and_follows_its_box() {
        let s = SmokeCloud {
            center: Vec3::new(0.0, 1.0, 0.0),
            size: Vec3::splat(1.0),
            ..Default::default()
        };
        assert!(smoke_density_at(&s, s.center) > 5.0, "cuore");
        assert_eq!(smoke_density_at(&s, Vec3::new(0.0, 2.0, 0.0)), 0.0, "fuori");
        assert_eq!(smoke_density_at(&s, Vec3::new(0.49, 1.49, 0.49)), 0.0, "angolo");
        // Bordo irregolare: sulla sfera di raggio 0,4 la densità varia molto.
        let vals: Vec<f32> = (0..64)
            .map(|i| {
                let a = i as f32 / 64.0 * std::f32::consts::TAU;
                smoke_density_at(&s, s.center + Vec3::new(a.cos(), 0.2, a.sin()) * 0.4)
            })
            .collect();
        let (lo, hi) = vals.iter().fold((f32::MAX, 0.0f32), |(a, b), v| (a.min(*v), b.max(*v)));
        assert!(hi - lo > 5.0, "bordo uniforme: {lo}..{hi}");
        let moved = SmokeCloud {
            center: s.center + Vec3::new(3.0, 0.5, -2.0),
            ..s.clone()
        };
        for p in [Vec3::ZERO, Vec3::new(0.3, -0.2, 0.1), Vec3::new(-0.35, 0.3, 0.2)] {
            let a = smoke_density_at(&s, s.center + p);
            let b = smoke_density_at(&moved, moved.center + p);
            assert!((a - b).abs() < 1e-3, "{a} vs {b}");
        }
    }

    /// Pennacchio: stretto e denso in basso, largo in alto; quote di partenza e di arrivo tagliano.
    #[test]
    fn plume_widens_and_height_limits_cut() {
        let s = SmokeCloud {
            shape: SmokeShape::Plume,
            center: Vec3::new(0.0, 1.0, 0.0),
            size: Vec3::new(1.0, 2.0, 1.0),
            meander: 0.0,
            turbulence: 0.0,
            billows: 0.0,
            wisps: 0.0,
            variation: 0.0,
            fade: 0.0,
            ..Default::default()
        };
        let at = |y: f32, x: f32| smoke_density_at(&s, Vec3::new(x, y, 0.0));
        assert!(at(0.2, 0.0) > 1.0, "asse in basso");
        assert_eq!(at(0.2, 0.4), 0.0, "in basso è stretto");
        assert!(at(1.6, 0.25) > 1.0, "in alto è largo");
        let cut = SmokeCloud {
            bottom_height: 0.5,
            top_height: 1.2,
            ..s.clone()
        };
        let cut_at = |y: f32| smoke_density_at(&cut, Vec3::new(0.0, y, 0.0));
        assert_eq!(cut_at(0.3), 0.0, "sotto la partenza");
        assert!(cut_at(0.9) > 1.0, "nella fascia");
        assert_eq!(cut_at(1.5), 0.0, "sopra l'arrivo");
    }
}
