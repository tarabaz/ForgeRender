//! Caustiche con il photon mapping progressivo (vedi `forge_render_core::CausticSettings`).
//!
//! A ogni passata:
//! 1. si emettono fotoni da tutte le luci (punto, spot, direzionale, area), diretti verso la sfera
//!    che contiene i vetri lisci della scena (mappa di proiezione: le caustiche nascono solo da lì,
//!    quindi restringere le direzioni non introduce errori e non spreca fotoni);
//! 2. ogni fotone attraversa i vetri lisci: riflessione o rifrazione scelte con la probabilità di
//!    Fresnel, legge di Snell con l'indice della sua lunghezza d'onda (render spettrale), riflessione
//!    totale, tinta del vetro e assorbimento di Beer-Lambert;
//! 3. alla prima superficie non speculare, dopo almeno un vetro, si posa (caustica); i fotoni che
//!    arrivano direttamente su una superficie non servono (luce diretta: la calcola il path tracer);
//! 4. il primo vertice non speculare del percorso dalla camera raccoglie i fotoni entro il raggio
//!    della passata (che si restringe con le passate) e il path tracer non conta più i percorsi
//!    "vetro → luce" che partono da quel vertice, così nulla viene contato due volte.
//!
//! I fotoni del render spettrale portano una lunghezza d'onda ciascuno (campionata con la
//! distribuzione visibile) e la loro luce arriva all'immagine già in RGB.

use crate::bsdf::{fresnel_dielectric, Bsdf, ROUGH_GLASS};
use forge_math::{offset_ray_origin, Onb, Pcg32, Vec3};
use forge_render_core::spectral::{cauchy_ior, lambda_to_rgb, sample_visible, visible_pdf, RgbSpectrum};
use forge_render_core::{RenderLight, RenderLightKind, RenderMaterial, RenderScene, RenderSettings};
use rayon::prelude::*;
use std::collections::HashMap;
use std::f32::consts::PI;

/// Fotone posato su una superficie.
#[derive(Clone, Copy, Debug)]
pub struct Photon {
    pub p: Vec3,
    /// Direzione verso cui era la luce (da cui arrivava il fotone).
    pub wi: Vec3,
    /// Normale geometrica della superficie, dal lato da cui arrivava.
    pub n: Vec3,
    /// Potenza: RGB, oppure (render spettrale) il valore alla sua lunghezza d'onda nel canale x.
    pub power: Vec3,
    /// Lunghezza d'onda (0 nel render RGB).
    pub lambda: f32,
}

/// Fotoni di una passata con la griglia per trovarli.
pub struct PhotonMap {
    pub photons: Vec<Photon>,
    pub radius: f32,
    cell: f32,
    grid: HashMap<(i32, i32, i32), Vec<u32>>,
    /// Fotoni emessi (per controllo).
    pub emitted: u32,
}

/// Il vetro liscio devia la luce senza disperderla: è dove nascono le caustiche.
#[inline]
pub(crate) fn is_delta_glass(m: &RenderMaterial) -> bool {
    m.transmission > 0.0 && m.roughness < ROUGH_GLASS
}

/// Sfera che contiene tutti i triangoli di vetro liscio (None = nessun vetro: niente caustiche).
pub fn specular_bounds(scene: &RenderScene) -> Option<(Vec3, f32)> {
    let geo = &*scene.geometry;
    let (mut lo, mut hi) = (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY));
    for prim in 0..geo.prim_to_slot.len() {
        if !is_delta_glass(&scene.materials[scene.material_index(prim)]) {
            continue;
        }
        for v in geo.vertices(prim) {
            lo = lo.min(v);
            hi = hi.max(v);
        }
    }
    (lo.x <= hi.x).then(|| ((lo + hi) * 0.5, ((hi - lo) * 0.5).length().max(1e-4)))
}

/// Raggio di raccolta iniziale: quello scelto, oppure l'1 % della dimensione della scena.
pub fn initial_radius(scene: &RenderScene, settings: &RenderSettings) -> f32 {
    if settings.caustics.radius_mm > 0.0 {
        return settings.caustics.radius_mm * 1e-3;
    }
    let b = &scene.geometry.bounds;
    ((b.max - b.min).length() * 0.5 * 0.01).max(1e-4)
}

/// Direzione uniforme nel cono di semiapertura con coseno `cos_max` attorno a `axis`.
fn sample_cone(axis: Vec3, cos_max: f32, u1: f32, u2: f32) -> Vec3 {
    let cos = 1.0 - u1 * (1.0 - cos_max);
    let sin = (1.0 - cos * cos).max(0.0).sqrt();
    let phi = 2.0 * PI * u2;
    Onb::from_normal(axis).to_world(Vec3::new(sin * phi.cos(), sin * phi.sin(), cos))
}

/// Direzione verso la sfera (`c`, `r`) vista da `x`: (direzione, pdf in angolo solido).
fn toward_sphere(x: Vec3, c: Vec3, r: f32, u1: f32, u2: f32) -> (Vec3, f32) {
    let d = c - x;
    let dist2 = d.length_squared();
    if dist2 <= r * r {
        let w = forge_math::sampling::uniform_sphere(u1, u2);
        return (w, 1.0 / (4.0 * PI));
    }
    let cos_max = (1.0 - r * r / dist2).max(0.0).sqrt();
    let w = sample_cone(d / dist2.sqrt(), cos_max, u1, u2);
    (w, 1.0 / (2.0 * PI * (1.0 - cos_max)).max(1e-12))
}

/// Peso di scelta di una luce (proporzionale alla potenza diretta verso i vetri, circa).
fn light_weight(l: &RenderLight, c: Vec3, r: f32) -> f32 {
    let lum = l.emission.max_element().max(0.0);
    let solid = |x: Vec3| {
        let d2 = (c - x).length_squared();
        if d2 <= r * r {
            4.0 * PI
        } else {
            2.0 * PI * (1.0 - (1.0 - r * r / d2).max(0.0).sqrt())
        }
    };
    match l.kind {
        RenderLightKind::Point { position } | RenderLightKind::Spot { position, .. } => lum * solid(position),
        RenderLightKind::Directional { .. } => lum * PI * r * r,
        RenderLightKind::Area { center, area, .. } => lum * area * solid(center),
    }
}

#[inline]
fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Un fotone emesso dalla luce `l` verso la sfera dei vetri: (origine, direzione, potenza RGB, e la
/// distanza a cui vale il raggio d'azione della luce: `Some(origine)` per punto e spot).
fn emit(l: &RenderLight, c: Vec3, r: f32, rng: &mut Pcg32) -> Option<(Vec3, Vec3, Vec3)> {
    let (u1, u2) = rng.next_2d();
    match l.kind {
        RenderLightKind::Point { position } => {
            let (w, pdf) = toward_sphere(position, c, r, u1, u2);
            Some((position, w, l.emission / pdf))
        }
        RenderLightKind::Spot {
            position,
            direction,
            cos_inner,
            cos_outer,
        } => {
            let (w, pdf) = toward_sphere(position, c, r, u1, u2);
            let f = smoothstep(cos_outer, cos_inner.max(cos_outer + 1e-4), w.dot(direction));
            (f > 0.0).then(|| (position, w, l.emission * (f / pdf)))
        }
        RenderLightKind::Directional { direction } => {
            // Disco perpendicolare alla luce che copre la sfera dei vetri, ben prima della scena.
            let d = rng.next_2d();
            let disk = forge_math::sampling::concentric_disk(d.0, d.1) * r;
            let frame = Onb::from_normal(direction);
            let o = c + frame.t * disk.x + frame.b * disk.y - direction * (r * 4.0 + 100.0);
            Some((o, direction, l.emission * (PI * r * r)))
        }
        RenderLightKind::Area {
            center,
            normal,
            u,
            v,
            half_w,
            half_h,
            area,
        } => {
            let (a, b) = rng.next_2d();
            let q = center + u * ((2.0 * a - 1.0) * half_w) + v * ((2.0 * b - 1.0) * half_h);
            let (w, pdf) = toward_sphere(q, c, r, u1, u2);
            let cos = w.dot(normal);
            // Radianza × coseno × area / pdf della direzione (punto uniforme sulla luce).
            (cos > 0.0).then(|| (q, w, l.emission * (cos * area / pdf)))
        }
    }
}

/// Segue un fotone dai vetri fino alla prima superficie non speculare: posato solo se ha
/// attraversato o toccato almeno un vetro liscio.
#[allow(clippy::too_many_arguments)]
fn trace_photon(
    scene: &RenderScene,
    settings: &RenderSettings,
    mut o: Vec3,
    mut d: Vec3,
    mut power: Vec3,
    lambda: f32,
    light: &RenderLight,
    rng: &mut Pcg32,
) -> Option<Photon> {
    let geo = &*scene.geometry;
    let mut glass_events = 0u32;
    // Volumi attraversati, come nel path tracer: liquido nel bicchiere con l'interfaccia giusta.
    let mut media = crate::integrator::Media::default();
    let absorb = |media: &crate::integrator::Media, t: f32, power: &mut Vec3| {
        if let Some(mi) = media.current(scene) {
            let m = &scene.materials[mi as usize];
            if m.volume_depth > 0.0 && m.volume_color != Vec3::ONE {
                let k = t / m.volume_depth;
                let c = if lambda > 0.0 {
                    Vec3::splat(RgbSpectrum::albedo(m.volume_color).eval(lambda))
                } else {
                    m.volume_color
                }
                .max(Vec3::splat(1e-6));
                *power *= Vec3::new(c.x.powf(k), c.y.powf(k), c.z.powf(k));
            }
        }
    };
    let ior_of = |m: &RenderMaterial| if lambda > 0.0 { cauchy_ior(m.ior, m.abbe, lambda) } else { m.ior };
    let mut first = true;
    for _ in 0..settings.max_bounces.max(2) {
        let mut hit = crate::textures::intersect(scene, o, d, 0.0, f32::INFINITY)?;
        // Facce di un volume "perdente" (il liquido dentro lo spessore del vetro): si attraversano.
        let mut skipped = 0;
        loop {
            let mi = scene.material_index(hit.prim as usize);
            let ng = geo.geometric_normals[hit.prim as usize];
            let front = ng.dot(d) < 0.0;
            if skipped >= 8 || !media.is_false_hit(scene, mi, front) {
                break;
            }
            absorb(&media, hit.t, &mut power);
            media.cross(mi, front);
            o = offset_ray_origin(geo.surface_point(hit.prim as usize, hit.u, hit.v), if front { -ng } else { ng });
            hit = crate::textures::intersect(scene, o, d, 0.0, f32::INFINITY)?;
            skipped += 1;
        }
        if first {
            // Raggio d'azione di punto e spot: conta la distanza fino al primo contatto.
            power *= light.range_window(hit.t);
            first = false;
        }
        let prim = hit.prim as usize;
        let p = geo.surface_point(prim, hit.u, hit.v);
        absorb(&media, hit.t, &mut power);
        let mi = scene.material_index(prim);
        let m = &scene.materials[mi];
        let mut ng = geo.geometric_normals[prim];
        let entering = ng.dot(d) < 0.0;
        if !entering {
            ng = -ng;
        }
        // Vetro liscio (con probabilità `transmission` nei materiali misti): evento speculare.
        if is_delta_glass(m) && rng.next_f32() < m.transmission {
            let [n0, n1, n2] = geo.shading_normals[prim];
            let mut n = (n0 * (1.0 - hit.u - hit.v) + n1 * hit.u + n2 * hit.v).normalize_or(ng);
            if n.dot(ng) <= 0.0 {
                n = if entering { n } else { -n };
            }
            if n.dot(-d) <= 0.0 {
                n = ng;
            }
            // Indici dei due lati: il materiale e il volume dall'altra parte (aria, il vetro
            // attorno a un liquido o il liquido dentro un vetro).
            let n_mat = ior_of(m);
            let n_other = media
                .outside_medium(scene, mi, entering)
                .map_or(1.0, |k| ior_of(&scene.materials[k as usize]));
            let (n_from, n_to) = if entering { (n_other, n_mat) } else { (n_mat, n_other) };
            let cos_i = (-d).dot(n);
            let (fr, cos_t) = fresnel_dielectric(cos_i, n_to / n_from);
            if rng.next_f32() < fr {
                d = (d + n * (2.0 * cos_i)).normalize();
                o = offset_ray_origin(p, ng);
            } else {
                let eta = n_from / n_to;
                d = (d * eta + n * (eta * cos_i - cos_t)).normalize();
                o = offset_ray_origin(p, -ng);
                // Tinta del vetro a ogni rifrazione (come la BSDF del path tracer).
                power *= if lambda > 0.0 {
                    Vec3::splat(RgbSpectrum::albedo(m.base_color).eval(lambda))
                } else {
                    m.base_color
                };
                media.cross(mi, entering);
            }
            glass_events += 1;
            if power.max_element() <= 0.0 {
                return None;
            }
            continue;
        }
        // Superficie non speculare: caustica se il fotone è passato da un vetro.
        return (glass_events > 0).then_some(Photon {
            p,
            wi: -d,
            n: ng,
            power,
            lambda,
        });
    }
    None
}

impl PhotonMap {
    /// Emette e segue i fotoni della passata `pass` (0, 1, 2…). `spectral`: una lunghezza d'onda
    /// per fotone.
    pub fn build(scene: &RenderScene, settings: &RenderSettings, target: (Vec3, f32), pass: u32, spectral: bool) -> PhotonMap {
        let (c, r) = target;
        let n = settings.caustics.photons();
        let weights: Vec<f32> = scene.lights.iter().map(|l| light_weight(l, c, r)).collect();
        let total: f32 = weights.iter().sum();
        let radius = settings.caustics.radius_at(initial_radius(scene, settings), pass);
        let mut photons: Vec<Photon> = Vec::new();
        if total > 0.0 {
            photons = (0..n)
                .into_par_iter()
                .filter_map(|i| {
                    let mut rng = Pcg32::for_pixel(settings.seed ^ 0x0F07_0A55, i, pass, 0);
                    // Luce scelta in proporzione al peso.
                    let mut x = rng.next_f32() * total;
                    let mut li = 0;
                    while li + 1 < weights.len() && x >= weights[li] {
                        x -= weights[li];
                        li += 1;
                    }
                    let l = &scene.lights[li];
                    let select = weights[li] / total;
                    let (o, d, mut power) = emit(l, c, r, &mut rng)?;
                    power /= select * n as f32;
                    let mut lambda = 0.0;
                    if spectral {
                        lambda = sample_visible(rng.next_f32());
                        let v = RgbSpectrum::unbounded(power).eval(lambda) / visible_pdf(lambda);
                        power = Vec3::new(v, 0.0, 0.0);
                    }
                    trace_photon(scene, settings, o, d, power, lambda, l, &mut rng)
                })
                .collect();
        }
        let cell = radius * 2.0;
        let mut grid: HashMap<(i32, i32, i32), Vec<u32>> = HashMap::new();
        for (i, ph) in photons.iter().enumerate() {
            grid.entry(cell_of(ph.p, cell)).or_default().push(i as u32);
        }
        PhotonMap {
            photons,
            radius,
            cell,
            grid,
            emitted: n,
        }
    }

    /// Fotoni entro il raggio da `p` sulla stessa faccia (normale `ng`): chiama `f` per ognuno.
    pub fn for_each_near(&self, p: Vec3, ng: Vec3, mut f: impl FnMut(&Photon)) {
        let (cx, cy, cz) = cell_of(p, self.cell);
        let r2 = self.radius * self.radius;
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let Some(list) = self.grid.get(&(cx + dx, cy + dy, cz + dz)) else {
                        continue;
                    };
                    for &i in list {
                        let ph = &self.photons[i as usize];
                        if (ph.p - p).length_squared() <= r2 && ph.n.dot(ng).abs() > 0.7 {
                            f(ph);
                        }
                    }
                }
            }
        }
    }

    /// Radianza riflessa verso `wo` dalle caustiche nel punto `p` (render RGB).
    pub fn gather_rgb(&self, bsdf: &Bsdf<'_, Vec3>, wo: Vec3, p: Vec3, ng: Vec3) -> Vec3 {
        let mut sum = Vec3::ZERO;
        self.for_each_near(p, ng, |ph| {
            let (f, _) = bsdf.eval(wo, ph.wi);
            sum += f * ph.power;
        });
        sum / (PI * self.radius * self.radius)
    }

    /// Come [`Self::gather_rgb`] nel render spettrale: ogni fotone con la sua lunghezza d'onda
    /// (colore del materiale a quella lunghezza d'onda), già convertito in RGB.
    pub fn gather_spectral(&self, mat: &RenderMaterial, frame: Onb, ng: Vec3, entering: bool, wo: Vec3, p: Vec3) -> Vec3 {
        let mut sum = Vec3::ZERO;
        self.for_each_near(p, ng, |ph| {
            let a = RgbSpectrum::albedo(mat.base_color).eval(ph.lambda);
            let bsdf = Bsdf {
                frame,
                ng,
                mat,
                entering,
                color: Vec3::splat(a),
            };
            let (f, _) = bsdf.eval(wo, ph.wi);
            if f.x > 0.0 {
                sum += lambda_to_rgb(ph.lambda, f.x * ph.power.x);
            }
        });
        sum / (PI * self.radius * self.radius)
    }
}

/// Parole per fotone nei dati per la GPU: posizione, direzione della luce, normale, potenza RGB.
pub const GPU_PHOTON_WORDS: usize = 12;

/// Cella della griglia → secchio della tabella di hash (`size` potenza di 2); lo shader usa la
/// stessa formula.
#[inline]
pub fn gpu_bucket(c: (i32, i32, i32), size: u32) -> u32 {
    ((c.0 as u32).wrapping_mul(73_856_093) ^ (c.1 as u32).wrapping_mul(19_349_663) ^ (c.2 as u32).wrapping_mul(83_492_791)) & (size - 1)
}

impl PhotonMap {
    /// Dati per lo shader: `[raggio, lato della cella, secchi, fotoni]`, poi (inizio, numero) per
    /// secchio, poi i fotoni ordinati per secchio. I fotoni spettrali arrivano già in RGB (la tinta
    /// della superficie che li raccoglie si applica in RGB).
    pub fn gpu_words(&self) -> Vec<u32> {
        let size = (self.photons.len().max(1) as u32).next_power_of_two();
        let bucket = |ph: &Photon| gpu_bucket(cell_of(ph.p, self.cell), size);
        let mut order: Vec<u32> = (0..self.photons.len() as u32).collect();
        order.sort_by_key(|&i| bucket(&self.photons[i as usize]));
        let mut w = Vec::with_capacity(4 + 2 * size as usize + GPU_PHOTON_WORDS * order.len());
        w.extend([self.radius.to_bits(), self.cell.to_bits(), size, order.len() as u32]);
        let mut counts = vec![0u32; size as usize];
        for ph in &self.photons {
            counts[bucket(ph) as usize] += 1;
        }
        let mut start = 0;
        for c in counts {
            w.extend([start, c]);
            start += c;
        }
        for i in order {
            let ph = &self.photons[i as usize];
            let rgb = if ph.lambda > 0.0 {
                lambda_to_rgb(ph.lambda, ph.power.x)
            } else {
                ph.power
            };
            for v in [ph.p, ph.wi, ph.n, rgb] {
                w.extend(v.to_array().map(f32::to_bits));
            }
        }
        w
    }
}

#[inline]
fn cell_of(p: Vec3, cell: f32) -> (i32, i32, i32) {
    (
        (p.x / cell).floor() as i32,
        (p.y / cell).floor() as i32,
        (p.z / cell).floor() as i32,
    )
}
