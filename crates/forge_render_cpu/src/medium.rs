//! Trasporto nei mezzi partecipanti (nebbie volumetriche).
//!
//! - Distanza di collisione: delta tracking (Woodcock) con majorant costante a
//!   tratti per strato orizzontale; non distorto, nessun passo di integrazione.
//! - Trasmittanza dei raggi d'ombra: ratio tracking sugli stessi tratti.
//! - Funzione di fase Henyey–Greenstein, campionata esattamente (peso 1).
//!
//! Convenzione: `cos θ = d · w`, con `d` direzione di avanzamento del percorso e
//! `w` la direzione verso la luce o il vertice successivo: con g > 0 la nebbia
//! diffonde "in avanti" (controluce luminoso).

use forge_math::{Onb, Pcg32, Vec3};
use forge_render_core::RenderScene;
use forge_volume::BakedFog;

/// Intervallo del raggio dentro il riquadro della nebbia, limitato a [0, t_max].
fn clip(f: &BakedFog, o: Vec3, d: Vec3, t_max: f32) -> Option<(f32, f32)> {
    let (lo, hi) = f.bounds();
    let inv = d.recip();
    let t0 = (lo - o) * inv;
    let t1 = (hi - o) * inv;
    let tn = t0.min(t1).max_element().max(0.0);
    let tf = t0.max(t1).min_element().min(t_max);
    (tn < tf).then_some((tn, tf))
}

/// Tratti a majorant costante lungo il raggio: chiama `visit(t_a, t_b, majorant)`
/// in ordine finché restituisce `true`.
fn for_each_segment(f: &BakedFog, o: Vec3, d: Vec3, t0: f32, t1: f32, mut visit: impl FnMut(f32, f32, f32) -> bool) {
    let n = f.slab_majorant.len();
    let (y0, h) = (f.spec.min.y, f.spec.cell);
    let mut t = t0;
    while t < t1 {
        let y = o.y + d.y * t;
        let k = (((y - y0) / h - 0.5).floor().max(0.0) as usize).min(n - 1);
        // Confine del tratto: il prossimo cambio di strato lungo y.
        let tb = if d.y > 1e-9 && k + 1 < n {
            (y0 + (k as f32 + 1.5) * h - o.y) / d.y
        } else if d.y < -1e-9 && k > 0 {
            (y0 + (k as f32 + 0.5) * h - o.y) / d.y
        } else {
            f32::INFINITY
        };
        // Avanzamento minimo per non fermarsi sui confini per errori di arrotondamento.
        let tb = tb.max(t + 1e-6 * (1.0 + t.abs())).min(t1);
        if !visit(t, tb, f.slab_majorant[k]) {
            return;
        }
        t = tb;
    }
}

/// Prima collisione reale con una delle nebbie lungo il raggio entro `t_max`:
/// (distanza, indice della nebbia).
pub fn sample_collision(scene: &RenderScene, o: Vec3, d: Vec3, t_max: f32, rng: &mut Pcg32) -> Option<(f32, usize)> {
    let mut best: Option<(f32, usize)> = None;
    for (fi, f) in scene.fogs.iter().enumerate() {
        let limit = best.map(|b| b.0).unwrap_or(t_max);
        let Some((t0, t1)) = clip(f, o, d, limit) else { continue };
        let mut hit = None;
        for_each_segment(f, o, d, t0, t1, |ta, tb, maj| {
            if maj <= 0.0 {
                return true;
            }
            let mut t = ta;
            loop {
                t -= (1.0 - rng.next_f32()).ln() / maj;
                if t >= tb {
                    return true; // nessuna collisione nel tratto (assenza di memoria)
                }
                if rng.next_f32() * maj < f.density(o + d * t) {
                    hit = Some(t);
                    return false;
                }
            }
        });
        if let Some(t) = hit {
            best = Some((t, fi));
        }
    }
    best
}

/// Trasmittanza stimata (ratio tracking) lungo il raggio fino a `t_max`.
pub fn transmittance(scene: &RenderScene, o: Vec3, d: Vec3, t_max: f32, rng: &mut Pcg32) -> f32 {
    let mut tr = 1.0f32;
    for f in &scene.fogs {
        let Some((t0, t1)) = clip(f, o, d, t_max) else { continue };
        for_each_segment(f, o, d, t0, t1, |ta, tb, maj| {
            if maj <= 0.0 {
                return true;
            }
            let mut t = ta;
            loop {
                t -= (1.0 - rng.next_f32()).ln() / maj;
                if t >= tb {
                    return true;
                }
                tr *= 1.0 - (f.density(o + d * t) / maj).min(1.0);
                // Roulette russa sulle trasmittanze piccole (resta non distorta).
                if tr < 0.1 {
                    if rng.next_f32() < 0.5 {
                        tr = 0.0;
                        return false;
                    }
                    tr *= 2.0;
                }
            }
        });
        if tr == 0.0 {
            return 0.0;
        }
    }
    tr
}

/// Densità della funzione di fase Henyey–Greenstein per `cos θ = d·w`.
#[inline]
pub fn hg_pdf(g: f32, cos: f32) -> f32 {
    let den = 1.0 + g * g - 2.0 * g * cos;
    (1.0 - g * g) / (4.0 * std::f32::consts::PI * den * den.max(1e-12).sqrt())
}

/// Direzione campionata dalla Henyey–Greenstein attorno a `d` (pdf = valore della fase).
pub fn hg_sample(g: f32, d: Vec3, u1: f32, u2: f32) -> Vec3 {
    let cos = if g.abs() < 1e-3 {
        1.0 - 2.0 * u1
    } else {
        let s = (1.0 - g * g) / (1.0 + g - 2.0 * g * u1);
        ((1.0 + g * g - s * s) / (2.0 * g)).clamp(-1.0, 1.0)
    };
    let sin = (1.0 - cos * cos).max(0.0).sqrt();
    let phi = std::f32::consts::TAU * u2;
    Onb::from_normal(d).to_world(Vec3::new(sin * phi.cos(), sin * phi.sin(), cos))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hg_sampling_matches_pdf() {
        for g in [-0.6f32, 0.0, 0.3, 0.8] {
            // Integrale della pdf = 1 e media del coseno = g.
            let n = 200_000;
            let mut rng = Pcg32::new(3, 7);
            let d = Vec3::new(0.3, -0.2, 0.93).normalize();
            let mut mean_cos = 0.0f64;
            for _ in 0..n {
                let w = hg_sample(g, d, rng.next_f32(), rng.next_f32());
                assert!((w.length() - 1.0).abs() < 1e-3);
                mean_cos += d.dot(w) as f64;
            }
            mean_cos /= n as f64;
            assert!((mean_cos - g as f64).abs() < 0.01, "g {g}: <cos> {mean_cos}");
            let m = 4000;
            let integral: f64 = (0..m)
                .map(|i| {
                    let c = -1.0 + (i as f32 + 0.5) * 2.0 / m as f32;
                    hg_pdf(g, c) as f64 * 2.0 * std::f64::consts::PI * (2.0 / m as f64)
                })
                .sum();
            assert!((integral - 1.0).abs() < 1e-3, "g {g}: ∫ = {integral}");
        }
    }
}
