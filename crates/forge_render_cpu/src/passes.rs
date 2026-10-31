//! Passaggi di render per la post-produzione: profondità, maschera del prodotto e ID degli
//! oggetti dalla prima superficie colpita da ogni pixel (albedo e normali sono le guide del
//! denoiser, [`crate::render_guides`]). Stessa camera e stesso jitter del path tracer, così i
//! bordi coincidono con l'immagine.

use forge_math::Pcg32;
use forge_render_core::{RenderScene, RenderSettings};
use rayon::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};

/// Gruppo che non appartiene al prodotto (sfondo, pavimento, fondale).
pub const NO_GROUP: u32 = u32::MAX;

/// Profondità, maschera e ID per pixel (riga per riga dall'alto).
#[derive(Clone, Debug)]
pub struct Passes {
    pub width: u32,
    pub height: u32,
    /// Distanza dalla camera lungo l'asse di vista, in metri (`f32::INFINITY` dove non c'è niente).
    pub depth: Vec<f32>,
    /// Frazione del pixel coperta dal prodotto (0..1, bordi sfumati).
    pub mask: Vec<f32>,
    /// Gruppo (componente) colpito al centro del pixel, [`NO_GROUP`] fuori dal prodotto.
    pub group: Vec<u32>,
}

/// Calcola i passaggi. `groups[nodo]` è il componente di ogni nodo di scena ([`NO_GROUP`] per
/// la scenografia). `None` se `cancel` interrompe il calcolo.
pub fn render_passes(scene: &RenderScene, settings: &RenderSettings, groups: &[u32], spp: u32, cancel: &AtomicBool) -> Option<Passes> {
    let (w, h) = (settings.width, settings.height);
    let n = w as usize * h as usize;
    let mut depth = vec![f32::INFINITY; n];
    let mut mask = vec![0.0f32; n];
    let mut group = vec![NO_GROUP; n];
    let spp = spp.max(1);
    let forward = scene.camera.forward();
    let group_of = |prim: u32| {
        let node = scene.geometry.tri_node[prim as usize] as usize;
        groups.get(node).copied().unwrap_or(NO_GROUP)
    };
    depth
        .par_chunks_mut(w as usize)
        .zip(mask.par_chunks_mut(w as usize))
        .zip(group.par_chunks_mut(w as usize))
        .enumerate()
        .for_each(|(y, ((drow, mrow), grow))| {
            if cancel.load(Ordering::Relaxed) {
                return;
            }
            for x in 0..w as usize {
                let (mut covered, mut near) = (0u32, f32::INFINITY);
                for s in 0..spp {
                    let mut rng = Pcg32::for_pixel(settings.seed ^ 0x0050_A55E, x as u32, y as u32, s);
                    let (o, d) = crate::camera_ray(&scene.camera, w, h, x as u32, y as u32, &mut rng);
                    let Some(hit) = crate::textures::intersect(scene, o, d, 0.0, f32::INFINITY) else {
                        continue;
                    };
                    near = near.min(hit.t * d.dot(forward));
                    if group_of(hit.prim) != NO_GROUP {
                        covered += 1;
                    }
                }
                drow[x] = near;
                mrow[x] = covered as f32 / spp as f32;
                // ID senza sfumature: un solo raggio, al centro del pixel.
                let cam = &scene.camera;
                let ndc_x = 2.0 * (x as f32 + 0.5) / w as f32 - 1.0 + 2.0 * cam.lens.shift_x;
                let ndc_y = 1.0 - 2.0 * (y as f32 + 0.5) / h as f32 + 2.0 * cam.lens.shift_y;
                let (o, d) = cam.ray(ndc_x, ndc_y, w as f32 / h as f32);
                if let Some(hit) = crate::textures::intersect(scene, o, d, 0.0, f32::INFINITY) {
                    grow[x] = group_of(hit.prim);
                }
            }
        });
    (!cancel.load(Ordering::Relaxed)).then_some(Passes {
        width: w,
        height: h,
        depth,
        mask,
        group,
    })
}

/// Colore ben distinguibile per un gruppo (sempre lo stesso per lo stesso numero): nero fuori
/// dal prodotto.
pub fn group_color(g: u32) -> [u8; 3] {
    if g == NO_GROUP {
        return [0, 0, 0];
    }
    // Sequenza R3 (costante plastica generalizzata): punti ben sparsi nel cubo dei colori, anche
    // con molti pezzi; ogni canale tra 50 e 255, così nessun pezzo è nero come lo sfondo.
    const A: [f64; 3] = [0.819_172_513_396_164_4, 0.671_043_606_703_789_2, 0.549_700_477_901_970_4];
    A.map(|a| {
        let t = (0.5 + a * (g as f64 + 1.0)).fract();
        (50.0 + t * 205.0).round() as u8
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_colors_are_distinct_and_background_is_black() {
        assert_eq!(group_color(NO_GROUP), [0, 0, 0]);
        let cols: Vec<[u8; 3]> = (0..64).map(group_color).collect();
        for (i, a) in cols.iter().enumerate() {
            assert_ne!(*a, [0, 0, 0]);
            for b in &cols[i + 1..] {
                let d: i32 = a.iter().zip(b).map(|(x, y)| (*x as i32 - *y as i32).abs()).sum();
                assert!(d > 20, "{a:?} {b:?}");
            }
        }
    }
}
