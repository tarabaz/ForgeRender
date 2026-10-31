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
use crate::lights::{intersect_area, sample_light, LightSample};
use forge_math::{luminance, offset_ray_origin, sampling::power_heuristic, Onb, Pcg32, Vec3};
use forge_render_core::{RenderScene, RenderSettings};

pub fn radiance(scene: &RenderScene, settings: &RenderSettings, mut o: Vec3, mut d: Vec3, rng: &mut Pcg32) -> Vec3 {
    let geo = &*scene.geometry;
    let mut l = Vec3::ZERO;
    let mut beta = Vec3::ONE;
    let mut specular_bounce = true; // il raggio camera si comporta come un rimbalzo delta
                                    // Ancora "visto dalla camera" (solo rimbalzi speculari): i fasci delle spot contano solo così.
    let mut camera_path = true;
    let mut prev_pdf = 0.0f32;
    // L'ambiente (HDRI, o qualsiasi ambiente se ci sono portali) è una "luce" in più
    // nella scelta uniforme della NEE.
    let n_lights = scene.lights.len();
    let has_env = scene.samples_environment();
    // Scelta uniforme tra: luci della scena, ambiente (se campionato), superfici emissive (se presenti).
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

    // Volumi trasparenti in cui si trova il percorso (vetro, liquido): assorbimento e IOR relativo.
    let mut media = Media::default();

    for depth in 0..settings.max_bounces {
        // Superficie successiva; le facce di un volume "perdente" (priorità più bassa del
        // volume in cui si è) non sono interfacce vere: si attraversano senza rimbalzo.
        let mut hit = crate::textures::intersect(scene, o, d, 0.0, f32::INFINITY);
        let mut skipped = 0;
        while let Some(h) = hit {
            let mi = scene.material_index(h.prim as usize);
            let front = geo.geometric_normals[h.prim as usize].dot(d) < 0.0;
            if skipped >= 8 || !media.is_false_hit(scene, mi, front) {
                break;
            }
            beta *= media.transmittance(scene, h.t);
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
        // Assorbimento del volume trasparente attraversato fino alla superficie (o alla luce).
        if media.n > 0 {
            let t_end = light_hit.map(|x| x.0).unwrap_or(t_surf);
            if t_end.is_finite() {
                beta *= media.transmittance(scene, t_end);
            }
        }
        // Nebbie: collisione nel mezzo prima della superficie o della luce (delta tracking:
        // se non avviene la throughput resta invariata).
        if !scene.fogs.is_empty() {
            let t_end = light_hit.map(|x| x.0).unwrap_or(t_surf);
            if let Some((tm, fi)) = crate::medium::sample_collision(scene, o, d, t_end, camera_path, rng) {
                let fog = &scene.fogs[fi];
                let p = o + d * tm;
                beta *= fog.albedo;
                if beta == Vec3::ZERO {
                    break;
                }
                let g = fog.anisotropy;
                if n_select > 0 {
                    if let Some((ls, cast_shadows)) = sample_any_light(scene, &slots, p, rng) {
                        let phase = crate::medium::hg_pdf(g, d.dot(ls.wi));
                        let vis = shadow(scene, p, &ls, cast_shadows, rng);
                        if vis > 0.0 {
                            let contrib = if ls.delta {
                                ls.li * (phase / light_select_pdf)
                            } else {
                                let lp = ls.pdf * light_select_pdf;
                                ls.li * (phase * power_heuristic(lp, phase) / lp)
                            };
                            add(&mut l, beta * contrib * vis, depth);
                        }
                    }
                }
                // Fascio di una spot: diffusione singola (luce diretta, poi fine del percorso).
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
            if depth == 0 {
                if settings.environment_visible {
                    add(&mut l, beta * scene.env_radiance(d, true), depth);
                }
            } else {
                // Peso MIS rispetto alla NEE dell'ambiente (HDRI o portali); con i portali la
                // PDF è zero per le direzioni che non passano da una finestra.
                let w = if specular_bounce || !has_env {
                    1.0
                } else {
                    power_heuristic(prev_pdf, scene.env_light_pdf(o, d) * light_select_pdf)
                };
                add(&mut l, beta * scene.env_radiance(d, false) * w, depth);
            }
            break;
        };

        // Geometria del punto colpito.
        let prim = hit.prim as usize;
        let p = geo.surface_point(prim, hit.u, hit.v);
        let mat_idx = scene.material_index(prim);
        let mut mat_local = scene.materials[mat_idx];
        let front_face = geo.geometric_normals[prim].dot(d) < 0.0;
        if mat_local.transmission > 0.0 {
            // IOR relativo al volume dall'altra parte (aria, oppure il vetro attorno al liquido).
            let outside = media.outside_ior(scene, mat_idx, front_face);
            // Può scendere sotto 1 (dal vetro al liquido, 1.34 / 1.5): la BSDF gestisce il caso.
            mat_local.ior = (mat_local.ior / outside).max(1e-3);
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
        // Normale di shading incoerente con la vista: si ripiega sulla geometrica.
        if ns.dot(wo) <= 0.0 || ns.dot(ng) <= 0.0 {
            ns = ng;
        }

        if mat.emission != Vec3::ZERO {
            // Superficie emissiva raggiunta dalla BSDF: peso MIS rispetto al campionamento diretto.
            let w = match &scene.emissive {
                Some(em) if !specular_bounce => {
                    let cos_l = ng.dot(wo).abs().max(1e-8);
                    let pdf_light = em.area_pdf(prim) * hit.t * hit.t / cos_l * light_select_pdf;
                    power_heuristic(prev_pdf, pdf_light)
                }
                _ => 1.0,
            };
            add(&mut l, beta * mat.emission * w, depth);
        }

        let bsdf = Bsdf {
            frame: Onb::from_normal(ns),
            ng,
            mat,
            entering,
        };

        // Next Event Estimation.
        if n_select > 0 && bsdf.has_non_delta() {
            if let Some((ls, cast_shadows)) = sample_any_light(scene, &slots, p, rng) {
                let (f, bsdf_pdf) = bsdf.eval(wo, ls.wi);
                if f != Vec3::ZERO {
                    let cos = ls.wi.dot(ns).abs();
                    let so = offset_ray_origin(p, if ls.wi.dot(ng) >= 0.0 { ng } else { -ng });
                    let vis = shadow(scene, so, &ls, cast_shadows, rng);
                    if vis > 0.0 {
                        let contrib = if ls.delta {
                            f * ls.li * (cos / light_select_pdf)
                        } else {
                            let lp = ls.pdf * light_select_pdf;
                            f * ls.li * (cos * power_heuristic(lp, bsdf_pdf) / lp)
                        };
                        add(&mut l, beta * contrib * vis, depth);
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
        camera_path &= s.delta;
        prev_pdf = s.pdf;
        if s.wi.dot(ng) < 0.0 && mat.transmission > 0.0 {
            // Rifratto: si entra nel volume (o se ne esce).
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

/// Volumi trasparenti attraversati dal percorso (al più 4 annidati), per gestire liquidi
/// nei bicchieri: il volume "corrente" è quello con priorità più alta (a pari priorità
/// l'ultimo in cui si è entrati).
#[derive(Default)]
pub(crate) struct Media {
    ids: [u32; 4],
    pub(crate) n: usize,
}

impl Media {
    fn current(&self, scene: &RenderScene) -> Option<u32> {
        let mut best: Option<u32> = None;
        for &m in &self.ids[..self.n] {
            if best.is_none_or(|b| scene.materials[m as usize].priority >= scene.materials[b as usize].priority) {
                best = Some(m);
            }
        }
        best
    }

    /// Faccia di un volume con priorità minore di quello corrente: non è un'interfaccia.
    fn is_false_hit(&self, scene: &RenderScene, mat: usize, front: bool) -> bool {
        if scene.materials[mat].transmission <= 0.0 {
            return false;
        }
        let Some(cur) = self.current(scene) else { return false };
        let p = scene.materials[mat].priority;
        let pc = scene.materials[cur as usize].priority;
        if front {
            p < pc
        } else {
            cur as usize != mat && self.ids[..self.n].contains(&(mat as u32)) && p < pc
        }
    }

    /// Entra nel volume (faccia frontale) o ne esce.
    fn cross(&mut self, mat: usize, front: bool) {
        if front {
            if self.n < self.ids.len() {
                self.ids[self.n] = mat as u32;
                self.n += 1;
            }
        } else if let Some(i) = self.ids[..self.n].iter().rposition(|&m| m as usize == mat) {
            self.ids.copy_within(i + 1..self.n, i);
            self.n -= 1;
        }
    }

    /// IOR del volume dall'altra parte della superficie del materiale `mat`.
    fn outside_ior(&self, scene: &RenderScene, mat: usize, front: bool) -> f32 {
        let mut other = Media { ids: self.ids, n: self.n };
        if !front {
            other.cross(mat, false);
        }
        other.current(scene).map(|m| scene.materials[m as usize].ior).unwrap_or(1.0)
    }

    /// Trasmittanza del volume corrente lungo un tratto di lunghezza `t`.
    fn transmittance(&self, scene: &RenderScene, t: f32) -> Vec3 {
        match self.current(scene) {
            Some(m) => {
                let s = scene.materials[m as usize].sigma_a;
                Vec3::new((-s.x * t).exp(), (-s.y * t).exp(), (-s.z * t).exp())
            }
            None => Vec3::ONE,
        }
    }
}

/// Luci tra cui scegliere per la NEE: luci della scena, HDRI, superfici emissive.
struct LightSlots {
    n_lights: usize,
    env_slot: Option<usize>,
    emissive_slot: Option<usize>,
    n_select: usize,
}

/// Sceglie uniformemente una "luce" e ne campiona una direzione dal punto `p`.
/// Restituisce anche se la luce proietta ombre.
fn sample_any_light(scene: &RenderScene, s: &LightSlots, p: Vec3, rng: &mut Pcg32) -> Option<(LightSample, bool)> {
    let li_idx = ((rng.next_f32() * s.n_select as f32) as usize).min(s.n_select - 1);
    let (u1, u2) = rng.next_2d();
    if li_idx < s.n_lights {
        let light = &scene.lights[li_idx];
        sample_light(light, p, u1, u2).map(|x| (x, light.cast_shadows))
    } else if Some(li_idx) == s.emissive_slot {
        let em = scene.emissive.as_ref().expect("emissivi");
        sample_emissive(scene, em, p, rng.next_f32(), u1, u2).map(|x| (x, true))
    } else {
        debug_assert_eq!(Some(li_idx), s.env_slot);
        if !scene.portals.is_empty() {
            // Con i portali la luce dell'ambiente si cerca solo attraverso le finestre.
            let u0 = rng.next_f32();
            return scene.portals.sample(p, u0, u1, u2).map(|(wi, pdf)| {
                (
                    LightSample {
                        wi,
                        dist: f32::INFINITY,
                        li: scene.env_radiance(wi, false),
                        pdf,
                        delta: false,
                    },
                    true,
                )
            });
        }
        let (map, strength, rot) = scene.env_map.as_ref().expect("env map");
        map.sample(u1, u2, *rot).map(|(wi, li, pdf)| {
            (
                LightSample {
                    wi,
                    dist: f32::INFINITY,
                    li: li * *strength,
                    pdf,
                    delta: false,
                },
                true,
            )
        })
    }
}

/// Visibilità della luce dal punto `so` (già spostato dalla superficie): 0 se un
/// oggetto la copre, altrimenti la trasmittanza delle nebbie lungo il raggio.
/// Una luce senza ombre ignora oggetti e nebbie.
fn shadow(scene: &RenderScene, so: Vec3, ls: &LightSample, cast_shadows: bool, rng: &mut Pcg32) -> f32 {
    if !cast_shadows {
        return 1.0;
    }
    let t_max = if ls.dist.is_finite() {
        ls.dist * (1.0 - 1e-4)
    } else {
        f32::INFINITY
    };
    if crate::textures::occluded(scene, so, ls.wi, 0.0, t_max) {
        return 0.0;
    }
    if scene.fogs.is_empty() {
        1.0
    } else {
        crate::medium::transmittance(scene, so, ls.wi, t_max, rng)
    }
}

/// Campiona un punto su una superficie emissiva (triangolo scelto in proporzione alla potenza).
fn sample_emissive(scene: &RenderScene, em: &forge_render_core::EmissiveSet, p: Vec3, u0: f32, u1: f32, u2: f32) -> Option<LightSample> {
    let (_, _, i) = em.distribution.sample(u0);
    let prim = em.prims[i] as usize;
    let [a, b, c] = scene.geometry.vertices(prim);
    // Punto uniforme sul triangolo.
    let su = u1.sqrt();
    let q = a * (1.0 - su) + b * (su * (1.0 - u2)) + c * (su * u2);
    let d = q - p;
    let d2 = d.length_squared();
    if d2 <= 0.0 {
        return None;
    }
    let dist = d2.sqrt();
    let wi = d / dist;
    let n = scene.geometry.geometric_normals[prim];
    let cos_l = n.dot(wi).abs(); // emissione su entrambe le facce
    if cos_l <= 1e-8 {
        return None;
    }
    let li = scene.material_of(prim).emission;
    Some(LightSample {
        wi,
        dist,
        li,
        pdf: em.area_pdf(prim) * d2 / cos_l,
        delta: false,
    })
}
