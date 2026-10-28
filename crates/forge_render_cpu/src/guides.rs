//! Buffer guida per il denoiser: albedo e normale della prima superficie
//! "visibile" per ogni pixel, mediati su qualche campione con lo stesso
//! jitter di pixel del path tracer (così gli spigoli coincidono).
//!
//! Attraverso il vetro liscio (BSDF delta) si prosegue fino alla prima superficie
//! non speculare, come raccomandato dalla documentazione di Open Image Denoise:
//! l'albedo viene moltiplicato per la trasmittanza del percorso speculare.
//! Sfondo e luci: albedo = radianza limitata a [0, 1], normale nulla.
//! Nebbie: albedo della nebbia e normale verso la camera, pesati per la trasmittanza.

use crate::bsdf::Bsdf;
use crate::lights::intersect_area;
use forge_math::{offset_ray_origin, Onb, Pcg32, Vec3};
use forge_render_core::{Guides, RenderScene, RenderSettings};
use rayon::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};

/// Campioni per pixel dei buffer guida (bastano pochi: sono quasi privi di rumore).
pub const GUIDE_SAMPLES: u32 = 8;
/// Rimbalzi speculari massimi seguiti attraverso vetri e specchi.
const MAX_SPECULAR: u32 = 6;

/// Calcola albedo e normali; `None` se `cancel` viene attivato durante il calcolo.
pub fn render_guides(scene: &RenderScene, settings: &RenderSettings, spp: u32, cancel: &AtomicBool) -> Option<Guides> {
    let (w, h) = (settings.width, settings.height);
    let n = w as usize * h as usize;
    let mut albedo = vec![[0.0f32; 3]; n];
    let mut normal = vec![[0.0f32; 3]; n];
    let spp = spp.max(1);
    let inv = 1.0 / spp as f32;
    albedo
        .par_chunks_mut(w as usize)
        .zip(normal.par_chunks_mut(w as usize))
        .enumerate()
        .for_each(|(y, (arow, nrow))| {
            if cancel.load(Ordering::Relaxed) {
                return;
            }
            for x in 0..w as usize {
                let (mut a, mut nn) = (Vec3::ZERO, Vec3::ZERO);
                for s in 0..spp {
                    // Sequenza casuale distinta da quella del rendering (seed diverso).
                    let mut rng = Pcg32::for_pixel(settings.seed ^ 0x006A_1DE5, x as u32, y as u32, s);
                    // Stessa camera del render (anche profondità di campo e shift): le guide
                    // sono sfocate come l'immagine, come richiede il denoiser.
                    let (o, dir) = crate::camera_ray(&scene.camera, w, h, x as u32, y as u32, &mut rng);
                    let (sa, sn) = guide_sample(scene, settings, o, dir, &mut rng);
                    a += sa;
                    nn += sn;
                }
                arow[x] = (a * inv).to_array();
                // Media delle normali: lungo gli spigoli si accorcia, come previsto da OIDN.
                nrow[x] = (nn * inv).clamp(Vec3::splat(-1.0), Vec3::splat(1.0)).to_array();
            }
        });
    (!cancel.load(Ordering::Relaxed)).then_some(Guides {
        width: w,
        height: h,
        albedo,
        normal,
    })
}

fn clamp01(c: Vec3) -> Vec3 {
    c.clamp(Vec3::ZERO, Vec3::ONE)
}

fn guide_sample(scene: &RenderScene, settings: &RenderSettings, mut o: Vec3, mut d: Vec3, rng: &mut Pcg32) -> (Vec3, Vec3) {
    let geo = &*scene.geometry;
    let mut tint = Vec3::ONE;
    for depth in 0..=MAX_SPECULAR {
        let hit = crate::textures::intersect(scene, o, d, 0.0, f32::INFINITY);
        let t_surf = hit.map(|h| h.t).unwrap_or(f32::INFINITY);
        let mut light: Option<(f32, Vec3, f32)> = None;
        for l in &scene.lights {
            if depth == 0 && !l.visible_to_camera {
                continue;
            }
            let t_max = light.map(|x| x.0).unwrap_or(t_surf);
            if let Some(h) = intersect_area(l, o, d, t_max) {
                light = Some(h);
            }
        }
        // Nebbia: dove il raggio vi si ferma la guida è l'albedo della nebbia con la normale
        // rivolta alla camera (la media sui campioni pesa nebbia e superficie per la trasmittanza).
        if !scene.fogs.is_empty() {
            let t_end = light.map(|x| x.0).unwrap_or(t_surf);
            if let Some((_, fi)) = crate::medium::sample_collision(scene, o, d, t_end, rng) {
                return (tint * clamp01(scene.fogs[fi].albedo), -d);
            }
        }
        if let Some((_, le, _)) = light {
            return (tint * clamp01(le), Vec3::ZERO);
        }
        let Some(hit) = hit else {
            let bg = if depth == 0 && !settings.environment_visible {
                Vec3::ZERO
            } else {
                scene.env_radiance(d, depth == 0)
            };
            return (tint * clamp01(bg), Vec3::ZERO);
        };
        let prim = hit.prim as usize;
        let p = o + d * hit.t;
        let mat_idx = scene.material_index(prim);
        let mut mat = scene.materials[mat_idx];
        let mut ng = geo.geometric_normals[prim];
        let [n0, n1, n2] = geo.shading_normals[prim];
        let mut ns = (n0 * (1.0 - hit.u - hit.v) + n1 * hit.u + n2 * hit.v).normalize_or(ng);
        if let Some(tex) = &scene.material_textures[mat_idx] {
            crate::textures::apply(tex, geo, &hit, p, &mut mat, &mut ns);
        }
        let wo = -d;
        let entering = ng.dot(wo) > 0.0;
        if !entering {
            ng = -ng;
            ns = -ns;
        }
        if ns.dot(wo) <= 0.0 || ns.dot(ng) <= 0.0 {
            ns = ng;
        }
        if mat.emission != Vec3::ZERO {
            return (tint * clamp01(mat.emission), ns);
        }
        let bsdf = Bsdf {
            frame: Onb::from_normal(ns),
            ng,
            mat: &mat,
            entering,
        };
        if bsdf.has_non_delta() || depth == MAX_SPECULAR {
            return (tint * mat.base_color, ns);
        }
        // Vetro liscio: si segue il raggio riflesso o rifratto (scelto con Fresnel).
        let (u_lobe, u1, u2) = (rng.next_f32(), rng.next_f32(), rng.next_f32());
        let Some(s) = bsdf.sample(wo, u_lobe, u1, u2) else {
            return (tint * mat.base_color, ns);
        };
        // La scelta riflessione/rifrazione segue già Fresnel; il peso del campione contiene anche
        // la scala 1/η² della radianza, che per l'albedo non va applicata: si usa solo la tinta
        // del vetro sui raggi rifratti (la riflessione speculare è bianca).
        if s.wi.dot(ng) < 0.0 {
            tint *= clamp01(mat.base_color);
        }
        let side = if s.wi.dot(ng) >= 0.0 { ng } else { -ng };
        o = offset_ray_origin(p, side);
        d = s.wi;
    }
    (tint, Vec3::ZERO)
}
