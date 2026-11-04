//! Tilt-shift sull'immagine finale: una fascia nitida (posizione, larghezza, angolo) e la sfocatura
//! che cresce ai lati con una transizione regolabile, indipendente dalla profondità di campo
//! fisica (diaframma, focale e messa a fuoco non cambiano).
//!
//! È un effetto di post-produzione, come i contorni: si applica in [`crate::Frame::display`] su
//! CPU e GPU allo stesso modo, anche dopo il denoiser, e cambiarne i parametri non rifà il render.
//! La sfocatura tiene conto della profondità (un buffer calcolato una volta all'avvio del render):
//! un oggetto vicino e nitido non "sbava" sullo sfondo sfocato e viceversa.
//!
//! Con la profondità di campo fisica accesa le due cose si sommano: prima la lente (nel render),
//! poi la fascia del tilt-shift sull'immagine.

use crate::{RenderScene, RenderSettings};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// Impostazioni del tilt-shift (salvate con le impostazioni del render).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TiltShift {
    pub enabled: bool,
    /// Centro della fascia nitida lungo la perpendicolare alla fascia: 0 = in alto, 1 = in basso
    /// (con angolo 0).
    pub center: f32,
    /// Angolo della fascia (gradi): 0 = orizzontale, 90 = verticale.
    pub angle_deg: f32,
    /// Larghezza della fascia nitida (frazione dell'altezza dell'immagine).
    pub width: f32,
    /// Transizione dal nitido al massimo sfocato, sopra / a sinistra della fascia (frazione
    /// dell'altezza; 0 = passaggio netto).
    pub transition_a: f32,
    /// Transizione sotto / a destra della fascia.
    pub transition_b: f32,
    /// Sfocatura massima (raggio in frazione della larghezza dell'immagine).
    pub blur: f32,
    /// La sfocatura rispetta la profondità (niente aloni tra oggetti a distanze diverse).
    pub depth_aware: bool,
}

impl Default for TiltShift {
    fn default() -> Self {
        Self {
            enabled: false,
            center: 0.55,
            angle_deg: 0.0,
            width: 0.18,
            transition_a: 0.2,
            transition_b: 0.2,
            blur: 0.012,
            depth_aware: true,
        }
    }
}

impl TiltShift {
    /// Valori nei limiti.
    pub fn sanitized(&self) -> TiltShift {
        let f = |v: f32, d: f32, lo: f32, hi: f32| if v.is_finite() { v.clamp(lo, hi) } else { d };
        TiltShift {
            enabled: self.enabled,
            center: f(self.center, 0.5, -0.5, 1.5),
            angle_deg: f(self.angle_deg, 0.0, -180.0, 180.0),
            width: f(self.width, 0.2, 0.0, 2.0),
            transition_a: f(self.transition_a, 0.2, 0.0, 2.0),
            transition_b: f(self.transition_b, 0.2, 0.0, 2.0),
            blur: f(self.blur, 0.01, 0.0, 0.05),
            depth_aware: self.depth_aware,
        }
    }

    /// Quanto è sfocato il pixel (0 = nitido, 1 = sfocatura massima), in coordinate normalizzate
    /// dell'immagine (x in [0, larghezza/altezza], y in [0, 1]).
    pub fn amount(&self, x: f32, y: f32, aspect: f32) -> f32 {
        let a = self.angle_deg.to_radians();
        // Perpendicolare alla fascia: con angolo 0 è l'asse verticale (verso il basso).
        let (s, c) = a.sin_cos();
        let n = (-s, c);
        let cx = aspect * 0.5 + n.0 * (self.center - 0.5);
        let cy = 0.5 + n.1 * (self.center - 0.5);
        let d = (x - cx) * n.0 + (y - cy) * n.1;
        let half = self.width * 0.5;
        let (dist, t) = if d < 0.0 {
            (-d - half, self.transition_a)
        } else {
            (d - half, self.transition_b)
        };
        if dist <= 0.0 {
            return 0.0;
        }
        if t <= 1e-4 {
            return 1.0;
        }
        let u = (dist / t).clamp(0.0, 1.0);
        u * u * (3.0 - 2.0 * u)
    }
}

/// Profondità per pixel (distanza lungo l'asse della camera; infinito dove non c'è niente).
#[derive(Clone, Debug)]
pub struct DepthBuffer {
    pub width: u32,
    pub height: u32,
    pub depth: Vec<f32>,
}

/// Profondità al centro di ogni pixel, se il tilt-shift è acceso.
pub fn depth_buffer(scene: &RenderScene, s: &RenderSettings) -> Option<DepthBuffer> {
    if !s.tilt_shift.enabled || s.width == 0 || s.height == 0 {
        return None;
    }
    let (w, h) = (s.width as usize, s.height as usize);
    let cam = &scene.camera;
    let forward = cam.forward();
    let aspect = w as f32 / h as f32;
    let geo = &scene.geometry;
    let solid = |prim: u32| !scene.materials[scene.material_index(prim as usize)].dust;
    let depth = (0..w * h)
        .into_par_iter()
        .map(|i| {
            let (x, y) = (i % w, i / w);
            let nx = 2.0 * (x as f32 + 0.5) / w as f32 - 1.0 + 2.0 * cam.lens.shift_x;
            let ny = 1.0 - 2.0 * (y as f32 + 0.5) / h as f32 + 2.0 * cam.lens.shift_y;
            let (ro, rd) = cam.ray(nx, ny, aspect);
            let hit = if scene.has_dust {
                geo.accel.intersect_filtered(ro, rd, 0.0, f32::INFINITY, |p, _, _, _| solid(p))
            } else {
                geo.accel.intersect(ro, rd, 0.0, f32::INFINITY)
            };
            hit.map(|h| h.t * rd.dot(forward)).unwrap_or(f32::INFINITY)
        })
        .collect();
    Some(DepthBuffer {
        width: s.width,
        height: s.height,
        depth,
    })
}

/// Campioni del disco di sfocatura (spirale aurea).
const TAPS: usize = 48;
/// Raggio del disco (in pixel del livello) sul livello della piramide da cui si raccoglie: con 48
/// campioni su un disco di questo raggio i campioni distano meno di un pixel e si fondono.
const LEVEL_RADIUS: f32 = 3.0;

/// Un livello della piramide: l'immagine ridotta di 2^n con raggio di sfocatura e profondità.
struct Level {
    w: usize,
    h: usize,
    color: Vec<[f32; 3]>,
    radius: Vec<f32>,
    depth: Vec<f32>,
}

impl Level {
    /// Livello successivo: media di 2×2 pixel (profondità: media di quelle finite).
    fn half(&self) -> Level {
        let (w, h) = (self.w.div_ceil(2), self.h.div_ceil(2));
        let mut out = Level {
            w,
            h,
            color: vec![[0.0; 3]; w * h],
            radius: vec![0.0; w * h],
            depth: vec![f32::INFINITY; w * h],
        };
        out.color
            .par_iter_mut()
            .zip(out.radius.par_iter_mut())
            .zip(out.depth.par_iter_mut())
            .enumerate()
            .for_each(|(i, ((c, r), d))| {
                let (x, y) = (i % w, i / w);
                let (mut sc, mut sr, mut sd, mut nd) = ([0.0f32; 3], 0.0, 0.0, 0);
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let (xx, yy) = ((2 * x + dx).min(self.w - 1), (2 * y + dy).min(self.h - 1));
                    let j = yy * self.w + xx;
                    for (k, v) in sc.iter_mut().enumerate() {
                        *v += self.color[j][k] * 0.25;
                    }
                    sr += self.radius[j] * 0.25;
                    if self.depth[j].is_finite() {
                        sd += self.depth[j];
                        nd += 1;
                    }
                }
                *c = sc;
                *r = sr;
                if nd > 0 {
                    *d = sd / nd as f32;
                }
            });
        out
    }

    /// Colore bilineare in coordinate del livello (centri dei pixel a i + 0,5).
    fn color_at(&self, x: f32, y: f32) -> [f32; 3] {
        let fx = (x - 0.5).clamp(0.0, (self.w - 1) as f32);
        let fy = (y - 0.5).clamp(0.0, (self.h - 1) as f32);
        let (x0, y0) = (fx.floor() as usize, fy.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(self.w - 1), (y0 + 1).min(self.h - 1));
        let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
        let at = |x: usize, y: usize| self.color[y * self.w + x];
        let (a, b, c, d) = (at(x0, y0), at(x1, y0), at(x0, y1), at(x1, y1));
        std::array::from_fn(|k| {
            let top = a[k] + (b[k] - a[k]) * tx;
            let bot = c[k] + (d[k] - c[k]) * tx;
            top + (bot - top) * ty
        })
    }
}

/// Immagine con il tilt-shift. Sfocatura a disco "raccolta": un campione vicino conta solo se la
/// sua sfocatura arriva fin qui (un pixel nitido non si spalma sui vicini sfocati) e, con la
/// profondità, se non è un oggetto molto più vicino e più nitido del pixel.
///
/// I raggi grandi si raccolgono da una piramide di immagini già ridotte (2×, 4×, 8×…): sul
/// livello giusto il disco ha un raggio di circa [`LEVEL_RADIUS`] pixel, i campioni si toccano e
/// i riflessi forti diventano una sfumatura continua invece di tante copie separate.
pub fn apply(img: &[[f32; 3]], w: u32, h: u32, t: &TiltShift, depth: Option<&DepthBuffer>) -> Vec<[f32; 3]> {
    let t = t.sanitized();
    let (w, h) = (w as usize, h as usize);
    if !t.enabled || img.len() != w * h || t.blur <= 0.0 {
        return img.to_vec();
    }
    let aspect = w as f32 / h as f32;
    let max_r = t.blur * w as f32;
    let radius: Vec<f32> = (0..w * h)
        .into_par_iter()
        .map(|i| {
            let (x, y) = (i % w, i / w);
            t.amount((x as f32 + 0.5) / h as f32, (y as f32 + 0.5) / h as f32, aspect) * max_r
        })
        .collect();
    let depth = depth.filter(|d| d.depth.len() == w * h && t.depth_aware);
    let mut levels = vec![Level {
        w,
        h,
        color: img.to_vec(),
        radius: radius.clone(),
        depth: depth.map(|d| d.depth.clone()).unwrap_or_else(|| vec![0.0; w * h]),
    }];
    while levels.len() < 8
        && max_r / (1usize << (levels.len() - 1)) as f32 > LEVEL_RADIUS
        && levels.last().is_some_and(|l| l.w > 4 && l.h > 4)
    {
        let next = levels.last().unwrap().half();
        levels.push(next);
    }
    let golden = std::f32::consts::PI * (3.0 - 5f32.sqrt());
    let aware = depth.is_some();
    (0..w * h)
        .into_par_iter()
        .map(|i| {
            let r = radius[i];
            if r < 0.5 {
                return img[i];
            }
            // Livello (anche frazionario): si raccoglie dai due livelli vicini e si mescola, così
            // il passaggio da un livello all'altro lungo la sfumatura non si vede.
            let lf = (r / LEVEL_RADIUS).log2().max(0.0).min((levels.len() - 1) as f32);
            let l0 = lf.floor() as usize;
            let frac = lf - l0 as f32;
            let (x, y) = ((i % w) as f32 + 0.5, (i / w) as f32 + 0.5);
            let d0 = levels[0].depth[i];
            let gather = |lvl: usize| -> [f32; 3] {
                let l = &levels[lvl];
                let scale = (1usize << lvl) as f32;
                let mut acc = img[i];
                let mut wsum = 1.0f32;
                for k in 0..TAPS {
                    let rr = r * ((k as f32 + 0.5) / TAPS as f32).sqrt();
                    let a = k as f32 * golden;
                    let (sx, sy) = (x + rr * a.cos(), y + rr * a.sin());
                    if sx < 0.0 || sy < 0.0 || sx >= w as f32 || sy >= h as f32 {
                        continue;
                    }
                    let (lx, ly) = (sx / scale, sy / scale);
                    let j = (ly as usize).min(l.h - 1) * l.w + (lx as usize).min(l.w - 1);
                    // La sfocatura del campione deve arrivare fin qui.
                    let mut wt = ((l.radius[j] - rr) / 1.5 + 1.0).clamp(0.0, 1.0);
                    // Un oggetto molto più vicino del pixel e più nitido non entra nello sfondo.
                    if aware && l.depth[j] < d0 * 0.85 && l.radius[j] < r * 0.6 {
                        wt = 0.0;
                    }
                    if wt > 0.0 {
                        let c = l.color_at(lx, ly);
                        acc[0] += c[0] * wt;
                        acc[1] += c[1] * wt;
                        acc[2] += c[2] * wt;
                        wsum += wt;
                    }
                }
                acc.map(|v| v / wsum)
            };
            let c0 = gather(l0);
            if frac < 0.01 || l0 + 1 >= levels.len() {
                return c0;
            }
            let c1 = gather(l0 + 1);
            std::array::from_fn(|k| c0[k] + (c1[k] - c0[k]) * frac)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fascia orizzontale al centro: nitida dentro, sfocata del tutto oltre la transizione, con
    /// le due transizioni indipendenti e quella nulla che fa un passaggio netto.
    #[test]
    fn band_and_transitions() {
        let t = TiltShift {
            enabled: true,
            center: 0.5,
            width: 0.2,
            transition_a: 0.1,
            transition_b: 0.0,
            ..Default::default()
        };
        let a = 16.0 / 9.0;
        let cx = a * 0.5;
        assert_eq!(t.amount(cx, 0.5, a), 0.0, "centro");
        assert_eq!(t.amount(cx, 0.41, a), 0.0, "dentro la fascia");
        assert!(t.amount(cx, 0.35, a) > 0.2 && t.amount(cx, 0.35, a) < 0.8, "transizione sopra");
        assert_eq!(t.amount(cx, 0.1, a), 1.0, "oltre la transizione");
        assert_eq!(t.amount(cx, 0.61, a), 1.0, "passaggio netto sotto");
        // Fascia verticale: conta la x.
        let v = TiltShift { angle_deg: 90.0, ..t };
        assert_eq!(v.amount(cx, 0.05, a), 0.0, "verticale: nitida su tutta la colonna centrale");
        assert_eq!(v.amount(0.05, 0.5, a), 1.0, "verticale: sfocata ai lati");
    }

    /// Un punto luminoso su fondo scuro, sfocato con un raggio grande: diventa una macchia
    /// continua, senza le copie separate dei singoli campioni (anelli o puntini).
    #[test]
    fn large_blur_has_no_ghost_copies() {
        let (w, h) = (400u32, 200u32);
        let mut img = vec![[0.0f32; 3]; (w * h) as usize];
        for y in 98..102 {
            for x in 198..202 {
                img[(y * w + x) as usize] = [50.0; 3];
            }
        }
        // Fascia nitida in alto: tutta la parte centrale ha la sfocatura massima (20 px).
        let t = TiltShift {
            enabled: true,
            center: 0.0,
            width: 0.0,
            transition_a: 0.0,
            transition_b: 0.0,
            blur: 0.05,
            ..Default::default()
        };
        let out = apply(&img, w, h, &t, None);
        // Lungo la riga del punto la luce cala in modo regolare, senza buchi tra copie.
        let row: Vec<f32> = (186..215).map(|x| out[(100 * w + x) as usize][0]).collect();
        let peak = row.iter().copied().fold(0.0f32, f32::max);
        assert!(peak > 0.0);
        let mut rising = true;
        for k in 1..row.len() {
            // Salita fino al centro, poi discesa: niente risalite (anelli) di oltre il 10 %.
            if k > row.len() / 2 {
                rising = false;
            }
            let (a, b) = (row[k - 1], row[k]);
            if rising {
                assert!(b >= a * 0.9 - 1e-4, "risalita a {k}: {row:?}");
            } else {
                assert!(b <= a * 1.1 + 1e-4, "anello a {k}: {row:?}");
            }
        }
    }

    /// Un'immagine a righe si sfoca fuori dalla fascia e resta identica dentro.
    #[test]
    fn blur_only_outside_the_band() {
        let (w, h) = (96u32, 54u32);
        let img: Vec<[f32; 3]> = (0..w * h).map(|i| if (i % w) % 2 == 0 { [1.0; 3] } else { [0.0; 3] }).collect();
        let t = TiltShift {
            enabled: true,
            center: 0.5,
            width: 0.2,
            transition_a: 0.05,
            transition_b: 0.05,
            blur: 0.03,
            ..Default::default()
        };
        let out = apply(&img, w, h, &t, None);
        let row = |y: u32| (0..w).map(|x| out[(y * w + x) as usize][0]).collect::<Vec<_>>();
        let mid = row(h / 2);
        assert!(
            mid.iter().zip(&img[(h / 2 * w) as usize..]).all(|(a, b)| (a - b[0]).abs() < 1e-6),
            "nitido al centro"
        );
        let top = row(2);
        let spread = top.iter().fold((1.0f32, 0.0f32), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
        assert!(spread.1 - spread.0 < 0.5, "righe sfocate in alto: {spread:?}");
    }
}
