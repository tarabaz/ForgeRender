//! Alone dei granelli luminosi (lucciole): un bagliore morbido del colore del granello attorno a
//! ognuno di quelli che la camera vede, aggiunto all'immagine finita (come il bagliore di una
//! luce piccola e intensa in una foto: diffusione nell'obiettivo e nell'aria).
//!
//! - In [`crate::RenderScene`] ci sono i granelli luminosi ([`GlowPoint`]: centro, diametro,
//!   luce emessa) e i due parametri dell'alone (grandezza in diametri del granello e forza).
//! - All'avvio del render [`compute`] li proietta sull'immagine con la camera del render (stesse
//!   formule dei raggi primari, prospettica o ortografica, con il decentramento) e tiene solo
//!   quelli davanti alla camera e non coperti da un oggetto (un raggio dalla camera al granello).
//! - [`Glow::apply`] aggiunge a ogni pixel la somma delle gaussiane vicine. Uguale per CPU e GPU.

use crate::RenderScene;
use forge_math::Vec3;

/// Un granello luminoso della scena.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlowPoint {
    pub center: Vec3,
    /// Diametro (m).
    pub diameter: f32,
    /// Luce emessa (colore × intensità, lineare).
    pub emission: Vec3,
}

/// Alone sull'immagine: un disco sfumato per granello visibile.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Glow {
    pub width: u32,
    pub height: u32,
    /// (x, y in pixel, raggio della gaussiana in pixel, colore di picco lineare).
    pub sprites: Vec<(f32, f32, f32, [f32; 3])>,
}

/// Il bagliore: un nucleo (gaussiana di raggio σ) più una coda tenue tre volte più larga, che
/// arrivano a zero a [`CUT`] σ senza bordo.
const TAIL: f32 = 3.0;
const TAIL_WEIGHT: f32 = 0.25;
const CUT: f32 = 9.0;

/// Alone per il render `width`×`height` della scena (None se non c'è niente da aggiungere).
pub fn compute(scene: &RenderScene, width: u32, height: u32) -> Option<Glow> {
    let (size, strength) = (scene.glow_size, scene.glow_strength);
    if scene.glow_points.is_empty() || size <= 0.0 || strength <= 0.0 || width == 0 || height == 0 {
        return None;
    }
    let cam = &scene.camera;
    let (w, h) = (width as f32, height as f32);
    let aspect = w / h;
    let (r, u, f) = cam.basis();
    let tan = (cam.vfov_deg.to_radians() * 0.5).tan();
    let ortho = cam.lens.ortho;
    let half = cam.ortho_half_height();
    let mut sprites = Vec::new();
    for g in scene.glow_points.iter() {
        let d = g.center - cam.position;
        let z = d.dot(f);
        // (ndc, metri per unità di ndc in verticale nel punto)
        let (nx, ny, m_per_ndc) = if ortho {
            (d.dot(r) / (half * aspect), d.dot(u) / half, half)
        } else {
            if z <= 1e-4 {
                continue;
            }
            (d.dot(r) / (z * tan * aspect), d.dot(u) / (z * tan), z * tan)
        };
        let px = (nx + 1.0 - 2.0 * cam.lens.shift_x) * 0.5 * w;
        let py = (1.0 - ny + 2.0 * cam.lens.shift_y) * 0.5 * h;
        // Diametro del granello in pixel e raggio dell'alone (almeno un pixel e mezzo).
        let d_px = g.diameter / (2.0 * m_per_ndc) * h;
        // Con grandezza 6 il nucleo è largo quanto il granello e la coda tre volte tanto; mai più
        // del 3 % dell'altezza dell'immagine (un granello vicinissimo non copre tutto).
        let sigma = (d_px * size / 6.0).clamp(0.8, h * 0.03);
        let reach = sigma * CUT;
        if px < -reach || py < -reach || px > w + reach || py > h + reach {
            continue;
        }
        // Coperto da un oggetto? Raggio dalla camera fin quasi al granello (la polvere non copre).
        let (o, dir, dist) = if ortho {
            let o = g.center - f * cam.ortho_back();
            (o, f, cam.ortho_back())
        } else {
            let dist = d.length();
            (cam.position, d / dist.max(1e-6), dist)
        };
        let t_max = (dist - g.diameter * 0.75).max(0.0);
        let blocked = scene
            .geometry
            .accel
            .intersect_filtered(o, dir, 1e-4, t_max, |prim, _, _, _| {
                !scene.materials[scene.material_index(prim as usize)].dust
            })
            .is_some();
        if blocked {
            continue;
        }
        // Picco del nucleo: una parte della luce del granello (resta del suo colore).
        let k = strength * 0.12;
        let c = g.emission * k;
        sprites.push((px, py, sigma, c.to_array()));
    }
    (!sprites.is_empty()).then_some(Glow { width, height, sprites })
}

impl Glow {
    /// Immagine con l'alone aggiunto (stessa dimensione; altrimenti invariata).
    pub fn apply(&self, img: &[[f32; 3]]) -> Vec<[f32; 3]> {
        let (w, h) = (self.width as usize, self.height as usize);
        let mut out = img.to_vec();
        if out.len() != w * h {
            return out;
        }
        for &(x, y, sigma, c) in &self.sprites {
            let reach = sigma * CUT;
            let (x0, x1) = (
                ((x - reach).floor().max(0.0)) as usize,
                ((x + reach).ceil().min(w as f32 - 1.0)).max(0.0) as usize,
            );
            let (y0, y1) = (
                ((y - reach).floor().max(0.0)) as usize,
                ((y + reach).ceil().min(h as f32 - 1.0)).max(0.0) as usize,
            );
            let inv = 1.0 / (2.0 * sigma * sigma);
            let inv_tail = inv / (TAIL * TAIL);
            let reach2 = reach * reach;
            for py in y0..=y1 {
                let dy = py as f32 + 0.5 - y;
                for px in x0..=x1 {
                    let dx = px as f32 + 0.5 - x;
                    let r2 = dx * dx + dy * dy;
                    if r2 > reach2 {
                        continue;
                    }
                    let fade = (1.0 - r2 / reach2).powi(2);
                    let a = ((-r2 * inv).exp() + TAIL_WEIGHT * (-r2 * inv_tail).exp()) * fade;
                    let p = &mut out[py * w + px];
                    p[0] += c[0] * a;
                    p[1] += c[1] * a;
                    p[2] += c[2] * a;
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glow_adds_a_soft_disc_around_the_point() {
        let g = Glow {
            width: 41,
            height: 41,
            sprites: vec![(10.5, 10.5, 1.0, [1.0, 0.5, 0.0])],
        };
        let img = g.apply(&vec![[0.0; 3]; 41 * 41]);
        let at = |x: usize, y: usize| img[y * 41 + x];
        assert!(at(10, 10)[0] > 1.0 && at(10, 10)[0] < 1.3, "picco al centro: {:?}", at(10, 10));
        assert!(
            at(12, 10)[0] < at(11, 10)[0] && at(11, 10)[0] < at(10, 10)[0],
            "cala allontanandosi"
        );
        assert!((at(10, 10)[1] / at(10, 10)[0] - 0.5).abs() < 1e-4, "colore del granello");
        assert_eq!(at(0, 0), [0.0; 3], "lontano niente");
    }
}
