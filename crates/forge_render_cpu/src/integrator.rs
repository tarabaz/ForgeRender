//! Integratore path tracing unidirezionale.
//!
//! Stimatore per ogni vertice del percorso:
//! - Next Event Estimation verso una luce scelta uniformemente, con MIS
//!   (euristica della potenza) tra campionamento della luce e della BSDF
//!   per le luci ad area; peso 1 per le luci delta;
//! - campionamento della BSDF per proseguire il percorso; quando un raggio
//!   BSDF colpisce una luce ad area si somma l'emissione con il peso MIS
//!   complementare (o pieno dopo un rimbalzo delta / per i raggi camera);
//! - l'ambiente e le superfici emissive non sono campionati esplicitamente
//!   (nella Milestone 1) e quindi contribuiscono con peso 1 quando colpiti;
//! - Russian roulette sulla throughput dal rimbalzo `russian_roulette_start`.
//!
//! Lo stimatore è non distorto a parte il troncamento a `max_bounces` e
//! l'eventuale `clamp_indirect` (disattivato per default).

use crate::bsdf::Bsdf;
use crate::lights::{intersect_area, sample_light};
use forge_math::{luminance, offset_ray_origin, sampling::power_heuristic, Onb, Pcg32, Vec3};
use forge_render_core::{RenderScene, RenderSettings};

pub fn radiance(scene: &RenderScene, settings: &RenderSettings, mut o: Vec3, mut d: Vec3, rng: &mut Pcg32) -> Vec3 {
    let geo = &*scene.geometry;
    let mut l = Vec3::ZERO;
    let mut beta = Vec3::ONE;
    let mut specular_bounce = true; // il raggio camera si comporta come un rimbalzo delta
    let mut prev_pdf = 0.0f32;
    let n_lights = scene.lights.len();
    let light_select_pdf = if n_lights > 0 { 1.0 / n_lights as f32 } else { 0.0 };
    let clamp = settings.clamp_indirect;

    let add = |l: &mut Vec3, c: Vec3, depth: u32| {
        if clamp > 0.0 && depth > 0 {
            let y = luminance(c);
            if y > clamp {
                *l += c * (clamp / y);
                return;
            }
        }
        *l += c;
    };

    for depth in 0..settings.max_bounces {
        let hit = geo.accel.intersect(o, d, 0.0, f32::INFINITY);
        let t_surf = hit.map(|h| h.t).unwrap_or(f32::INFINITY);

        // Luci ad area (non sono nella BVH: poche e analitiche).
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
        if let Some((_, le, light_pdf)) = light_hit {
            if le != Vec3::ZERO {
                let w = if specular_bounce {
                    1.0
                } else {
                    power_heuristic(prev_pdf, light_pdf * light_select_pdf)
                };
                add(&mut l, beta * le * w, depth);
            }
            break;
        }

        let Some(hit) = hit else {
            if depth > 0 || settings.environment_visible {
                add(&mut l, beta * scene.environment.radiance(d), depth);
            }
            break;
        };

        // Geometria del punto colpito.
        let prim = hit.prim as usize;
        let p = o + d * hit.t;
        let mat = scene.material_of(prim);
        let mut ng = geo.geometric_normals[prim];
        let [n0, n1, n2] = geo.shading_normals[prim];
        let mut ns = (n0 * (1.0 - hit.u - hit.v) + n1 * hit.u + n2 * hit.v).normalize_or(ng);
        let wo = -d;
        let entering = ng.dot(wo) > 0.0;
        if !entering {
            ng = -ng;
            ns = -ns;
        }
        // Normale di shading incoerente con la vista: si ripiega sulla geometrica.
        if ns.dot(wo) <= 0.0 || ns.dot(ng) <= 0.0 {
            ns = ng;
        }

        if mat.emission != Vec3::ZERO {
            add(&mut l, beta * mat.emission, depth);
        }

        let bsdf = Bsdf {
            frame: Onb::from_normal(ns),
            ng,
            mat,
            entering,
        };

        // Next Event Estimation.
        if n_lights > 0 && bsdf.has_non_delta() {
            let li_idx = ((rng.next_f32() * n_lights as f32) as usize).min(n_lights - 1);
            let light = &scene.lights[li_idx];
            let (u1, u2) = rng.next_2d();
            if let Some(ls) = sample_light(light, p, u1, u2) {
                let (f, bsdf_pdf) = bsdf.eval(wo, ls.wi);
                if f != Vec3::ZERO {
                    let cos = ls.wi.dot(ns).abs();
                    let visible = !light.cast_shadows || {
                        let so = offset_ray_origin(p, ng);
                        !geo.accel.occluded(so, ls.wi, 0.0, ls.dist * (1.0 - 1e-4))
                    };
                    if visible {
                        let contrib = if ls.delta {
                            f * ls.li * (cos / light_select_pdf)
                        } else {
                            let lp = ls.pdf * light_select_pdf;
                            f * ls.li * (cos * power_heuristic(lp, bsdf_pdf) / lp)
                        };
                        add(&mut l, beta * contrib, depth);
                    }
                }
            }
        }

        // Campionamento della BSDF.
        let (u_lobe, u1, u2) = (rng.next_f32(), rng.next_f32(), rng.next_f32());
        let Some(s) = bsdf.sample(wo, u_lobe, u1, u2) else { break };
        beta *= s.weight;
        if !beta.is_finite() || beta == Vec3::ZERO {
            break;
        }
        specular_bounce = s.delta;
        prev_pdf = s.pdf;
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
