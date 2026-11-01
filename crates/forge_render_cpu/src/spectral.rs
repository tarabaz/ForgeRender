//! Integratore spettrale (modalità "Spettrale"): lo stesso path tracer di `integrator`, con la
//! radianza portata a quattro lunghezze d'onda per percorso (vedi
//! `forge_render_core::spectral`).
//!
//! - I colori della scena (materiali, texture, luci, ambiente, nebbie) restano in RGB fino al
//!   punto in cui servono e lì diventano spettri lisci (sigmoide di un polinomio): riflettanze
//!   limitate a [0, 1], emissioni senza limite.
//! - Le direzioni non dipendono dalla lunghezza d'onda (stesse pdf per tutte e quattro), tranne
//!   nei trasparenti con dispersione (numero di Abbe > 0): lì l'indice di rifrazione è quello della
//!   hero e le altre tre lunghezze d'onda si fermano (PBRT v4, `TerminateSecondary`).
//! - Assorbimento nei volumi (Beer-Lambert) per lunghezza d'onda: T(λ) = R(λ)^(d / d₀), con R lo
//!   spettro del colore del volume allo spessore d₀.
//! - Roulette russa sul massimo della throughput: probabilità indipendente dalla lunghezza d'onda.

use crate::bsdf::Bsdf;
use crate::integrator::{sample_any_light, shadow, LightSlots, Media};
use crate::lights::intersect_area;
use forge_math::glam::Vec4;
use forge_math::{offset_ray_origin, sampling::power_heuristic, Onb, Pcg32, Vec3};
use forge_render_core::spectral::{cauchy_ior, RgbSpectrum, Wavelengths};
use forge_render_core::{RenderMaterial, RenderScene, RenderSettings};

/// Riflettanza RGB → valori alle quattro lunghezze d'onda.
#[inline]
fn albedo(rgb: Vec3, w: &Wavelengths) -> Vec4 {
    if rgb == Vec3::ZERO {
        Vec4::ZERO
    } else {
        Vec4::from_array(RgbSpectrum::albedo(rgb).sample(w))
    }
}

/// Radianza RGB (luci, ambiente, emissione) → valori alle quattro lunghezze d'onda.
#[inline]
fn illuminant(rgb: Vec3, w: &Wavelengths) -> Vec4 {
    if rgb == Vec3::ZERO {
        Vec4::ZERO
    } else {
        Vec4::from_array(RgbSpectrum::unbounded(rgb).sample(w))
    }
}

/// Indice di rifrazione del materiale alla lunghezza d'onda hero.
#[inline]
fn ior_at(m: &RenderMaterial, w: &Wavelengths) -> f32 {
    cauchy_ior(m.ior, m.abbe, w.hero())
}

/// Trasmittanza del volume corrente lungo un tratto di lunghezza `t`, per lunghezza d'onda.
fn transmittance(media: &Media, scene: &RenderScene, t: f32, w: &Wavelengths) -> Vec4 {
    match media.current(scene) {
        Some(m) => {
            let m = &scene.materials[m as usize];
            if m.volume_color == Vec3::ONE || m.volume_depth <= 0.0 {
                return Vec4::ONE;
            }
            let r = albedo(m.volume_color, w).max(Vec4::splat(1e-6));
            let k = t / m.volume_depth;
            Vec4::new(r.x.powf(k), r.y.powf(k), r.z.powf(k), r.w.powf(k))
        }
        None => Vec4::ONE,
    }
}

/// Solo la hero: le altre lunghezze d'onda si fermano (throughput e radianza azzerate).
#[inline]
fn keep_hero(w: &mut Wavelengths, beta: &mut Vec4, l: &mut Vec4) {
    if !w.secondary_terminated() {
        w.terminate_secondary();
        *beta = Vec4::new(beta.x, 0.0, 0.0, 0.0);
        *l = Vec4::new(l.x, 0.0, 0.0, 0.0);
    }
}

/// Radianza lungo il raggio alle lunghezze d'onda `w` (che possono ridursi alla sola hero).
pub fn radiance_spectral(
    scene: &RenderScene,
    settings: &RenderSettings,
    mut o: Vec3,
    mut d: Vec3,
    rng: &mut Pcg32,
    w: &mut Wavelengths,
) -> Vec4 {
    let geo = &*scene.geometry;
    let mut l = Vec4::ZERO;
    let mut beta = Vec4::ONE;
    let mut specular_bounce = true;
    let mut camera_path = true;
    let mut prev_pdf = 0.0f32;
    let n_lights = scene.lights.len();
    let has_env = scene.samples_environment();
    let env_slot = has_env.then_some(n_lights);
    let emissive_slot = scene.emissive.is_some().then_some(n_lights + has_env as usize);
    let n_select = n_lights + has_env as usize + scene.emissive.is_some() as usize;
    let light_select_pdf = if n_select > 0 { 1.0 / n_select as f32 } else { 0.0 };
    let slots = LightSlots {
        n_lights,
        env_slot,
        emissive_slot,
        n_select,
    };
    let clamp = settings.clamp_indirect;
    let add = |l: &mut Vec4, c: Vec4, depth: u32| {
        if clamp > 0.0 && depth > 0 {
            // Equivalente spettrale della luminanza: la media delle lunghezze d'onda.
            let y = (c.x + c.y + c.z + c.w) * 0.25;
            if y > clamp {
                *l += c * (clamp / y);
                return;
            }
        }
        *l += c;
    };

    let mut media = Media::default();

    for depth in 0..settings.max_bounces {
        let mut hit = crate::textures::intersect(scene, o, d, 0.0, f32::INFINITY);
        let mut skipped = 0;
        while let Some(h) = hit {
            let mi = scene.material_index(h.prim as usize);
            let front = geo.geometric_normals[h.prim as usize].dot(d) < 0.0;
            if skipped >= 8 || !media.is_false_hit(scene, mi, front) {
                break;
            }
            beta *= transmittance(&media, scene, h.t, w);
            media.cross(mi, front);
            o = offset_ray_origin(
                geo.surface_point(h.prim as usize, h.u, h.v),
                if front {
                    -geo.geometric_normals[h.prim as usize]
                } else {
                    geo.geometric_normals[h.prim as usize]
                },
            );
            hit = crate::textures::intersect(scene, o, d, 0.0, f32::INFINITY);
            skipped += 1;
        }
        let t_surf = hit.map(|h| h.t).unwrap_or(f32::INFINITY);

        let mut light_hit: Option<(f32, Vec3, f32)> = None;
        for light in &scene.lights {
            if depth == 0 && !light.visible_to_camera {
                continue;
            }
            let t_max = light_hit.map(|x| x.0).unwrap_or(t_surf);
            if let Some(h) = intersect_area(light, o, d, t_max) {
                light_hit = Some(h);
            }
        }
        if media.n > 0 {
            let t_end = light_hit.map(|x| x.0).unwrap_or(t_surf);
            if t_end.is_finite() {
                beta *= transmittance(&media, scene, t_end, w);
            }
        }
        if !scene.fogs.is_empty() {
            let t_end = light_hit.map(|x| x.0).unwrap_or(t_surf);
            if let Some((tm, fi)) = crate::medium::sample_collision(scene, o, d, t_end, camera_path, rng) {
                let fog = &scene.fogs[fi];
                let p = o + d * tm;
                beta *= albedo(fog.albedo, w);
                if beta == Vec4::ZERO {
                    break;
                }
                let g = fog.anisotropy;
                if n_select > 0 {
                    if let Some((ls, cast_shadows)) = sample_any_light(scene, &slots, p, rng) {
                        let phase = crate::medium::hg_pdf(g, d.dot(ls.wi));
                        let vis = shadow(scene, p, &ls, cast_shadows, rng);
                        if vis > 0.0 {
                            let li = illuminant(ls.li, w);
                            let contrib = if ls.delta {
                                li * (phase / light_select_pdf)
                            } else {
                                let lp = ls.pdf * light_select_pdf;
                                li * (phase * power_heuristic(lp, phase) / lp)
                            };
                            add(&mut l, beta * contrib * vis, depth);
                        }
                    }
                }
                if fog.camera_only {
                    break;
                }
                let (u1, u2) = rng.next_2d();
                let wi = crate::medium::hg_sample(g, d, u1, u2);
                prev_pdf = crate::medium::hg_pdf(g, d.dot(wi));
                specular_bounce = false;
                camera_path = false;
                o = p;
                d = wi;
                if depth + 1 >= settings.russian_roulette_start {
                    let q = beta.max_element().min(0.95);
                    if rng.next_f32() >= q {
                        break;
                    }
                    beta /= q;
                }
                continue;
            }
        }

        if let Some((_, le, light_pdf)) = light_hit {
            if le != Vec3::ZERO {
                let wgt = if specular_bounce {
                    1.0
                } else {
                    power_heuristic(prev_pdf, light_pdf * light_select_pdf)
                };
                add(&mut l, beta * illuminant(le, w) * wgt, depth);
            }
            break;
        }

        let Some(hit) = hit else {
            if depth == 0 {
                if settings.environment_visible {
                    add(&mut l, beta * illuminant(scene.env_radiance(d, true), w), depth);
                }
            } else {
                let wgt = if specular_bounce || !has_env {
                    1.0
                } else {
                    power_heuristic(prev_pdf, scene.env_light_pdf(o, d) * light_select_pdf)
                };
                add(&mut l, beta * illuminant(scene.env_radiance(d, false), w) * wgt, depth);
            }
            break;
        };

        let prim = hit.prim as usize;
        let p = geo.surface_point(prim, hit.u, hit.v);
        let mat_idx = scene.material_index(prim);
        let mut mat_local = scene.materials[mat_idx];
        let front_face = geo.geometric_normals[prim].dot(d) < 0.0;
        if mat_local.transmission > 0.0 {
            // Indici di rifrazione (del materiale e del volume dall'altra parte) alla hero: se uno
            // dei due disperde, la direzione dipende dalla lunghezza d'onda e resta solo la hero.
            let outside = media
                .outside_medium(scene, mat_idx, front_face)
                .map(|m| &scene.materials[m as usize]);
            if mat_local.abbe > 0.0 || outside.is_some_and(|m| m.abbe > 0.0) {
                keep_hero(w, &mut beta, &mut l);
            }
            let n_in = ior_at(&mat_local, w);
            let n_out = outside.map(|m| ior_at(m, w)).unwrap_or(1.0);
            mat_local.ior = (n_in / n_out).max(1e-3);
            mat_local.f0_dielectric = ((mat_local.ior - 1.0) / (mat_local.ior + 1.0)).powi(2);
        }
        let mut ng = geo.geometric_normals[prim];
        let [n0, n1, n2] = geo.shading_normals[prim];
        let mut ns = (n0 * (1.0 - hit.u - hit.v) + n1 * hit.u + n2 * hit.v).normalize_or(ng);
        if let Some(tex) = &scene.material_textures[mat_idx] {
            crate::textures::apply(tex, scene, &hit, p, &mut mat_local, &mut ns);
        }
        let mat = &mat_local;
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
            let wgt = match &scene.emissive {
                Some(em) if !specular_bounce => {
                    let cos_l = ng.dot(wo).abs().max(1e-8);
                    let pdf_light = em.area_pdf(prim) * hit.t * hit.t / cos_l * light_select_pdf;
                    power_heuristic(prev_pdf, pdf_light)
                }
                _ => 1.0,
            };
            add(&mut l, beta * illuminant(mat.emission, w) * wgt, depth);
        }

        let bsdf = Bsdf {
            frame: Onb::from_normal(ns),
            ng,
            mat,
            entering,
            color: albedo(mat.base_color, w),
        };

        if n_select > 0 && bsdf.has_non_delta() {
            if let Some((ls, cast_shadows)) = sample_any_light(scene, &slots, p, rng) {
                let (f, bsdf_pdf) = bsdf.eval(wo, ls.wi);
                if f != Vec4::ZERO {
                    let cos = ls.wi.dot(ns).abs();
                    let so = offset_ray_origin(p, if ls.wi.dot(ng) >= 0.0 { ng } else { -ng });
                    let vis = shadow(scene, so, &ls, cast_shadows, rng);
                    if vis > 0.0 {
                        let li = illuminant(ls.li, w);
                        let contrib = if ls.delta {
                            f * li * (cos / light_select_pdf)
                        } else {
                            let lp = ls.pdf * light_select_pdf;
                            f * li * (cos * power_heuristic(lp, bsdf_pdf) / lp)
                        };
                        add(&mut l, beta * contrib * vis, depth);
                    }
                }
            }
        }

        let (u_lobe, u1, u2) = (rng.next_f32(), rng.next_f32(), rng.next_f32());
        let Some(s) = bsdf.sample(wo, u_lobe, u1, u2) else { break };
        beta *= s.weight;
        if !beta.is_finite() || beta == Vec4::ZERO {
            break;
        }
        specular_bounce = s.delta;
        camera_path &= s.delta;
        prev_pdf = s.pdf;
        if s.wi.dot(ng) < 0.0 && mat.transmission > 0.0 {
            media.cross(mat_idx, front_face);
        }
        let side = if s.wi.dot(ng) >= 0.0 { ng } else { -ng };
        o = offset_ray_origin(p, side);
        d = s.wi;

        if depth + 1 >= settings.russian_roulette_start {
            let q = beta.max_element().min(0.95);
            if rng.next_f32() >= q {
                break;
            }
            beta /= q;
        }
    }
    l
}

/// Un campione spettrale per il pixel `(x, y)`, già in RGB lineare della scena.
#[inline]
pub fn render_pixel_spectral(scene: &RenderScene, settings: &RenderSettings, x: u32, y: u32, sample: u32) -> [f32; 3] {
    let mut rng = Pcg32::for_pixel(settings.seed, x, y, sample);
    let (o, dir) = crate::camera_ray(&scene.camera, settings.width, settings.height, x, y, &mut rng);
    // Lunghezze d'onda da una sequenza a parte: i rimbalzi usano gli stessi numeri del render RGB.
    let mut wrng = Pcg32::for_pixel(settings.seed ^ 0x5BEC_7A11, x, y, sample);
    let mut w = Wavelengths::sample(wrng.next_f32());
    let l = radiance_spectral(scene, settings, o, dir, &mut rng, &mut w);
    w.to_rgb(l.to_array()).to_array()
}
