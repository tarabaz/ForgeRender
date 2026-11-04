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

/// Immagine con il tilt-shift. Sfocatura a disco "raccolta": un campione vicino conta solo se la
/// sua sfocatura arriva fin qui (un pixel nitido non si spalma sui vicini sfocati) e, con la
/// profondità, se non è un oggetto molto più vicino e più nitido del pixel.
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
    let golden = std::f32::consts::PI * (3.0 - 5f32.sqrt());
    (0..w * h)
        .into_par_iter()
        .map(|i| {
            let r = radius[i];
            if r < 0.5 {
                return img[i];
            }
            let (x, y) = ((i % w) as f32, (i / w) as f32);
            let d0 = depth.map(|d| d.depth[i]).unwrap_or(0.0);
            let mut acc = [img[i][0], img[i][1], img[i][2]];
            let mut wsum = 1.0f32;
            for k in 0..TAPS {
                let rr = r * ((k as f32 + 0.5) / TAPS as f32).sqrt();
                let a = k as f32 * golden;
                let sx = (x + rr * a.cos()).round();
                let sy = (y + rr * a.sin()).round();
                if sx < 0.0 || sy < 0.0 || sx >= w as f32 || sy >= h as f32 {
                    continue;
                }
                let j = sy as usize * w + sx as usize;
                // La sfocatura del campione deve arrivare fin qui.
                let mut wt = ((radius[j] - rr) / 1.5 + 1.0).clamp(0.0, 1.0);
                if let Some(d) = depth {
                    // Un oggetto molto più vicino del pixel e più nitido non entra nello sfondo.
                    let dj = d.depth[j];
                    if dj < d0 * 0.85 && radius[j] < r * 0.6 {
                        wt *= 0.0;
                    }
                }
                if wt > 0.0 {
                    let c = img[j];
                    acc[0] += c[0] * wt;
                    acc[1] += c[1] * wt;
                    acc[2] += c[2] * wt;
                    wsum += wt;
                }
            }
            [acc[0] / wsum, acc[1] / wsum, acc[2] / wsum]
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
