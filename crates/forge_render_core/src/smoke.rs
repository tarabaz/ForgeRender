//! Fumo volumetrico ([`forge_scene::SmokeCloud`]): il campo di densità procedurale e il suo
//! calcolo su griglia ([`BakedFog`]), con cache.
//!
//! La densità nasce da tre strati, tutti nelle coordinate del riquadro (spostare il riquadro
//! sposta la nube identica, senza residui dove era prima):
//! 1. **forma di massima** (nube, pennacchio, filo d'incenso, banco): un valore che vale circa 1
//!    nel cuore e 0 sul bordo, valutato in un punto *distorto* dai vortici, così il contorno
//!    si attorciglia invece di essere una sfera o un cono;
//! 2. **bordo**: sbuffi "a cavolfiore" (frattale di |rumore|, lobi tondi con pieghe nette) e
//!    filamenti (creste del rumore) che sfilacciano la parte esterna;
//! 3. **interno**: variazione lenta della densità, così la nube non è un blocco uniforme.
//!
//! Il fumo è un mezzo fisico (diffonde, assorbe, fa ombra) e non entra negli oggetti.

use crate::fog::{interior_cells, voxelize};
use crate::RenderGeometry;
use forge_math::Vec3;
use forge_scene::{Scene, SmokeCloud, SmokeMode, SmokeShape};
use forge_volume::noise::{fbm, fbm_gain, gradient};
#[cfg(test)]
use forge_volume::smoke_sim::simulate_smoke;
use forge_volume::smoke_sim::{simulate_smoke_frames, SmokeField, SmokeSimParams};
use forge_volume::{progress, BakedFog, GridSpec};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Frattale "a sbuffi": somma di 2|rumore| − 1, lobi tondi separati da pieghe nette
/// (l'aspetto a cavolfiore del fumo denso), in circa [−1, 1].
fn billow_fbm(p: Vec3, seed: u32, octaves: u32, gain: f32) -> f32 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..octaves {
        let g = gradient(p * freq + Vec3::splat(o as f32 * 13.7), seed.wrapping_add(o * 7919));
        sum += amp * (2.0 * g.abs() * 1.6 - 1.0);
        norm += amp;
        amp *= gain;
        freq *= 2.0;
    }
    sum / norm
}

/// Distorsione del dominio (vortici), ampiezza tipica ~1.
fn warp(q: Vec3, s: u32) -> Vec3 {
    Vec3::new(
        fbm(q + Vec3::new(5.2, 1.3, 9.1), s ^ 0x68e3_1da4, 3),
        fbm(q + Vec3::new(-7.7, 3.9, 2.4), s ^ 0x1b56_c4e9, 3),
        fbm(q + Vec3::new(1.7, -9.2, 6.3), s ^ 0x2f8a_7d11, 3),
    ) * 2.2
}

/// Pennacchio e incenso: posizione relativa all'asse che serpeggia salendo, larghezza alla quota
/// `t` (0 in fondo, 1 in cima) e quanto il moto è già turbolento a quella quota.
fn column(s: &SmokeCloud, u: Vec3, t: f32, thin: bool) -> (f32, f32) {
    let (r0, grow) = if thin { (0.07, 0.75) } else { (0.16, 1.0) };
    let width = r0 + (1.0 - r0) * s.spread * grow * t.powf(0.65);
    let seed = s.seed;
    let m = s.meander * t.powf(1.2) * 0.55;
    let ax = fbm(Vec3::new(t * 2.2, 0.3, 1.7), seed ^ 0x51ed_270b, 3) * m * 1.6;
    let az = fbm(Vec3::new(t * 2.2, 4.1, -2.3), seed ^ 0x7f4a_7c15, 3) * m * 1.6;
    let d = Vec3::new(u.x - ax, 0.0, u.z - az).length() / width.max(1e-3);
    // In fondo nasce dalla bocca (niente fumo sotto), in cima si chiude piano.
    ((1.0 - d).min(t * 10.0).min((1.0 - t) * 6.0), width)
}

/// Densità del fumo procedurale nel punto `p` (1/m).
pub fn smoke_density_at(s: &SmokeCloud, p: Vec3) -> f32 {
    smoke_density_with(s, p, None)
}

/// Densità del fumo nel punto `p` (1/m); con la simulazione (`field`) la forma di massima è
/// quella simulata e il rumore aggiunge solo il dettaglio fine sul bordo.
pub fn smoke_density_with(s: &SmokeCloud, p: Vec3, field: Option<&SmokeField>) -> f32 {
    let field = field.filter(|_| s.mode == SmokeMode::Simulated);
    let (lo, hi) = s.bounds();
    // Coordinate del riquadro (anche ruotato attorno alla verticale).
    let half = s.half();
    let rel = s.to_local(p);
    let u = rel / half.max(Vec3::splat(1e-4));
    // Fuori dal riquadro (anche con i vortici) niente fumo.
    let box_edge = 1.0 - u.abs().max_element();
    if box_edge <= 0.0 {
        return 0.0;
    }
    // Quote di partenza e di arrivo, misurate dal fondo del riquadro.
    let y = p.y - lo.y;
    let size_y = hi.y - lo.y;
    let fw = (0.05 * size_y).max(0.005);
    let mut cut = 1.0;
    if s.bottom_height > 0.0 {
        cut *= smoothstep(s.bottom_height, s.bottom_height + fw, y);
    }
    if s.top_height > 0.0 {
        cut *= 1.0 - smoothstep(s.top_height - fw, s.top_height, y);
    }
    if cut <= 0.0 {
        return 0.0;
    }
    // Fumo a particelle: il campo ha già veli e filamenti; la densità lo segue in proporzione
    // (niente soglie né rumore, che lo farebbero pieno e gonfio), con il contrasto per i veli.
    if let Some(f) = field.filter(|_| s.sim.particles) {
        // Limite alto: un filo sottile ma concentrato (incenso) deve restare denso, non spianato.
        let v = (f.sample(p) / f.max.max(1e-6) * s.edge_keep(p)).clamp(0.0, 64.0);
        let k = if s.contrast.is_finite() { s.contrast.clamp(0.5, 4.0) } else { 1.0 };
        return s.density * v.powf(k) * smoothstep(0.0, 0.08, box_edge) * cut;
    }
    let l = s.detail_scale.max(0.002);
    let seed = s.seed;
    let q = rel / l + Vec3::new(17.0, -3.0, 9.0) * (seed % 97) as f32;
    let t = (u.y + 1.0) * 0.5;
    // Turbolenza: il pennacchio e soprattutto l'incenso nascono lisci e si rompono salendo.
    let laminar = match (field, s.shape) {
        (Some(_), _) => 0.5,
        (None, SmokeShape::Plume) => 0.25 + 0.75 * t.powf(0.8),
        (None, SmokeShape::Wisp) => 0.08 + 0.92 * t.powf(1.6),
        _ => 1.0,
    };
    let turb = s.turbulence * laminar;
    // Bordi sfumati del fumo simulato (1 lontano dalle pareti, 0 sulla parete): nella fascia
    // vortici, sbuffi e filamenti si fanno più forti, così il fumo si sfilaccia in modo frattale
    // invece di svanire in una sfumatura uniforme.
    let ek = if field.is_some() { s.edge_keep(p) } else { 1.0 };
    let erode = 1.0 + 2.5 * (1.0 - ek);
    // Uscita veloce dove la forma di massima è lontana (oltre il massimo spostamento dei vortici).
    let w = warp(q, seed);
    let disp = w * (turb * 0.45 * l * (1.0 + 1.5 * (1.0 - ek)));
    let up = (rel + disp) / half.max(Vec3::splat(1e-4));
    let tp = ((up.y + 1.0) * 0.5).clamp(0.0, 1.0);
    // Forma e larghezza del pezzo (frazione del riquadro: il filo d'incenso è sottile).
    // Con la simulazione: la sua densità rispetto al punto più denso (4 % = bordo, 40 % o più =
    // pieno), letta nel punto mosso dai vortici fini.
    // Curva schiacciata (radice): il fumo diluito che sale resta visibile, come nel fumo vero.
    let sim = field.map(|f| (f.sample(p + s.rotation() * disp * 0.5) / f.max.max(1e-6) * ek).max(0.0).powf(0.55));
    let (base, width) = match (sim, s.shape) {
        (Some(v), _) => ((v * 2.5 - 0.25).min(1.2), 0.3),
        (None, SmokeShape::Cloud) => (1.0 - up.length(), 1.0),
        (None, SmokeShape::Bank) => {
            let a = up.abs();
            (1.0 - (a.x.powi(6) + a.y.powi(6) + a.z.powi(6)).powf(1.0 / 6.0), 1.0)
        }
        (None, SmokeShape::Plume) => column(s, up, tp, false),
        (None, SmokeShape::Wisp) => column(s, up, tp, true),
    };
    // Spessore del bordo in unità della forma: dettagli più grandi = bordo più frastagliato; mai
    // tanto da spezzare un pezzo sottile.
    let feature = (half.x.min(half.z) * width).max(1e-4);
    let rel_l = (l / feature).clamp(0.02, 0.4);
    if base < -1.2 * rel_l - 0.1 {
        return 0.0;
    }
    let qd = q + w * turb * 0.6;
    let oct = s.octaves.clamp(1, 10);
    let gain = s.roughness.clamp(0.2, 0.85);
    // Bordo: sbuffi a cavolfiore e rumore frattale.
    let puffs = billow_fbm(qd * 0.8, seed ^ 0x3c6e_f372, oct, gain);
    let grain = fbm_gain(qd, seed, oct, gain);
    let bump = (s.billows * 0.9 * puffs + (1.0 - s.billows) * 0.5 * grain) * rel_l * 1.4 * erode;
    // Filamenti: creste sottili del rumore, che portano il fumo anche poco fuori dal bordo.
    let ridge = (1.0 - fbm_gain(qd * 2.3 + Vec3::splat(4.7), seed ^ 0x9e37_79b9, oct.min(6), gain).abs() * 2.2)
        .max(0.0)
        .powi(4);
    let edge = base + bump + s.wisps.max(0.5 * (1.0 - ek)) * ridge * rel_l * 0.5 * erode;
    let soft = 0.02 + 0.45 * s.softness * rel_l.max(0.15);
    let mut cov = smoothstep(0.0, soft, edge);
    if cov <= 0.0 {
        return 0.0;
    }
    // Vicino al bordo la nube si sfilaccia in filamenti (il cuore resta pieno).
    let rim = 1.0 - smoothstep(0.0, (4.0 * soft + 0.25 * rel_l).min(0.45), edge);
    cov *= 1.0 - s.wisps * rim * (1.0 - (ridge * 1.6).min(1.0));
    // Interno: variazione lenta (media ≈ 1).
    let v = fbm(qd * 0.45 + Vec3::splat(-6.1), seed ^ 0x2545_f491, 3);
    let inner = (1.0 - s.variation) + s.variation * 2.0 * smoothstep(-0.55, 0.65, v);
    // Pennacchio e incenso si diradano salendo.
    // (Con la simulazione la dissolvenza è già nel moto; la densità segue quella simulata.)
    let fade = match (sim, s.shape) {
        (Some(v), _) => (v * 2.0).min(1.0),
        (None, SmokeShape::Plume | SmokeShape::Wisp) => (1.0 - s.fade * t.powf(0.8)).max(0.0).powf(1.5),
        _ => 1.0,
    };
    // Mai un taglio netto sulle facce del riquadro.
    let walls = smoothstep(0.0, 0.08, box_edge);
    // Contrasto: la densità relativa (0–1 circa) elevata a una potenza; con le parti piene a 1
    // i grumi restano densi e le parti diradate si fanno più trasparenti.
    let shape = cov * inner * fade * walls * cut;
    let k = if s.contrast.is_finite() { s.contrast.clamp(0.5, 4.0) } else { 1.0 };
    let shape = if (k - 1.0).abs() < 1e-3 {
        shape
    } else {
        shape.clamp(0.0, 2.0).powf(k)
    };
    s.density * shape
}

/// Qualità: (celle sul lato più lungo, celle al massimo).
const SMOKE_QUALITY: [(usize, f32); 4] = [(128, 3.0e6), (192, 7.0e6), (288, 14.0e6), (400, 24.0e6)];

type Cache = Mutex<HashMap<String, Arc<BakedFog>>>;

fn cache() -> &'static Cache {
    static C: OnceLock<Cache> = OnceLock::new();
    C.get_or_init(Default::default)
}

fn active(scene: &Scene) -> impl Iterator<Item = SmokeCloud> + '_ {
    // Un fumo simulato "solo anteprima" vive nella viewport: il render non lo calcola.
    scene
        .smokes
        .iter()
        .filter(|s| s.enabled && s.density > 0.0 && !(s.preview_only && s.mode == SmokeMode::Simulated))
        .map(|s| s.sanitized())
}

fn key(s: &SmokeCloud, geo: &RenderGeometry) -> String {
    let k = SmokeCloud {
        name: String::new(),
        n: 0,
        preview_only: false,
        ..s.clone()
    };
    // Con le collisioni contano gli oggetti (qualsiasi cambio di geometria: semplice e sicuro).
    format!("{k:?}|{}", if s.collisions { geo.id } else { 0 })
}

#[cfg(test)]
type SimCache = Mutex<HashMap<String, Arc<SmokeField>>>;

#[cfg(test)]
fn sim_cache() -> &'static SimCache {
    static C: OnceLock<SimCache> = OnceLock::new();
    C.get_or_init(Default::default)
}

/// Parametri della simulazione di una nube sulla griglia `spec`.
pub(crate) fn sim_params(s: &SmokeCloud, spec: GridSpec) -> SmokeSimParams {
    let m = &s.sim;
    SmokeSimParams {
        spec,
        source: s.sim_source(),
        source_radius: m.source_radius,
        emission: m.emission,
        temperature: m.temperature,
        buoyancy: m.buoyancy,
        weight: m.weight,
        exit_speed: m.exit_speed,
        cooling: m.cooling,
        dissipation: m.dissipation,
        vorticity: m.vorticity,
        turbulence: m.turbulence,
        turbulence_scale: m.turbulence_scale,
        wind: m.wind,
        time: m.time,
        seed: s.seed,
        edge_fade: m.edge_fade,
        box_center: s.center,
        box_yaw: s.rotation_deg.to_radians(),
        box_half: s.half(),
        particles: m.particles,
        patchiness: m.patchiness,
        emit_stop: m.emit_stop,
    }
}

/// Simulazione della nube fino al suo istante, senza precalcolo (per le prove).
#[cfg(test)]
fn simulated_field(s: &SmokeCloud, geo: &RenderGeometry, p0: f32, p1: f32) -> Option<Arc<SmokeField>> {
    let (lo, hi) = s.bounds();
    let k = format!(
        "{:?}|{:?}|{:?}|{}|{}|{}",
        s.sim,
        lo,
        hi,
        s.rotation_deg,
        s.seed,
        if s.collisions { geo.id } else { 0 }
    );
    if let Some(f) = sim_cache().lock().unwrap().get(&k) {
        return Some(f.clone());
    }
    let cells = s.sim_cells();
    let spec = GridSpec::covering(lo, hi, cells);
    let solid = (s.collisions && !geo.accel.is_empty()).then(|| {
        let shell = voxelize(geo, &spec);
        let inside = interior_cells(geo, &spec, &shell);
        shell.iter().zip(&inside).map(|(a, b)| *a || *b).collect::<Vec<bool>>()
    });
    let params = sim_params(s, spec);
    let f = simulate_smoke(
        &params,
        solid.as_deref(),
        &|x| progress::update(p0 + (p1 - p0) * x),
        &progress::cancelled,
    )?;
    let max = f.density.iter().copied().fold(0.0f32, f32::max);
    let full = f.density.iter().filter(|&&v| v > 0.05).count();
    tracing::info!(
        "fumo simulato «{}»: {:?} celle, {} passi in {:.0} ms (massimo {:.2}, {} celle con fumo)",
        s.name,
        f.spec.res,
        f.steps,
        f.ms,
        max,
        full
    );
    if std::env::var_os("FORGE_SMOKE_AXIS").is_some() {
        let src = s.sim_source();
        let line: Vec<String> = (0..24)
            .map(|k| {
                let y = lo.y + (hi.y - lo.y) * k as f32 / 24.0;
                let mut best = 0.0f32;
                for dx in -6..=6 {
                    for dz in -6..=6 {
                        let q = Vec3::new(src.x + dx as f32 * 0.002, y, src.z + dz as f32 * 0.002);
                        best = best.max(f.sample(q));
                    }
                }
                format!("{:.2}:{:.1}", y, best / f.max.max(1e-6))
            })
            .collect();
        tracing::info!("asse del fumo (quota:densità/rif): {}", line.join(" "));
    }
    let f = Arc::new(f);
    let mut c = sim_cache().lock().unwrap();
    if c.len() > 8 {
        c.clear();
    }
    c.insert(k, f.clone());
    Some(f)
}

/// Numero di nubi di fumo accese.
pub fn smoke_count(scene: &Scene) -> usize {
    active(scene).count()
}

/// Nubi di fumo accese già calcolate?
pub fn smokes_ready(scene: &Scene, geo: &RenderGeometry) -> bool {
    let c = cache().lock().unwrap();
    active(scene).all(|s| c.contains_key(&key(&s, geo)))
}

/// Una nube calcolata (dalla cache o calcolata qui, con l'avanzamento sulla barra); None se il
/// calcolo è stato annullato.
/// Anteprima della simulazione per la viewport: lo stesso fumo simulato su una griglia grossolana
/// (veloce, circa mezzo secondo), con un fotogramma ogni [`PREVIEW_FRAMES`]-esimo del tempo
/// simulato. Ogni fotogramma è una nuvola di punti dove c'è fumo (più punti dove è denso): mostra
/// dove va il fumo e come si muove senza fare il render. Non è il fumo del render (manca il
/// dettaglio fine e la griglia è più rada), solo il suo andamento.
#[derive(Clone, Debug, Default)]
pub struct SmokePreview {
    /// (istante in secondi, punti: posizione e densità relativa 0–1).
    pub frames: Vec<(f32, Vec<(Vec3, f32)>)>,
    /// Lato della cella dell'anteprima (m).
    pub cell: f32,
}

impl SmokePreview {
    /// Il fotogramma più vicino all'istante `t`.
    pub fn at(&self, t: f32) -> Option<&[(Vec3, f32)]> {
        let i = self.frames.partition_point(|f| f.0 < t).min(self.frames.len().checked_sub(1)?);
        Some(&self.frames[i].1)
    }
}

/// Fotogrammi dell'anteprima nel tempo simulato.
pub const PREVIEW_FRAMES: usize = 48;
/// Celle al massimo nella griglia dell'anteprima.
const PREVIEW_CELLS: f32 = 36_000.0;
/// Punti al massimo per fotogramma.
const PREVIEW_POINTS: usize = 5000;

/// Calcola l'anteprima (None se annullata o se il fumo non è simulato).
/// Con `scene` e oggetti animati il fumo li sente come nel render: bordi in movimento e aria
/// spostata (`geo` è allora la geometria senza di loro).
pub fn smoke_preview(
    s: &SmokeCloud,
    scene: Option<&forge_scene::Scene>,
    geo: &RenderGeometry,
    cancelled: &dyn Fn() -> bool,
) -> Option<SmokePreview> {
    let s = s.sanitized();
    if s.mode != SmokeMode::Simulated {
        return None;
    }
    let (lo, hi) = s.bounds();
    let ext = (hi - lo).max(Vec3::splat(1e-3));
    let cell = (ext.x * ext.y * ext.z / PREVIEW_CELLS).cbrt().max(ext.max_element() / 96.0);
    let spec = GridSpec::covering(lo, hi, ((ext.max_element() / cell).ceil() as usize).max(8));
    let solid = (s.collisions && !geo.accel.is_empty()).then(|| {
        let shell = voxelize(geo, &spec);
        let inside = interior_cells(geo, &spec, &shell);
        shell.iter().zip(&inside).map(|(a, b)| *a || *b).collect::<Vec<bool>>()
    });
    let m = &s.sim;
    let params = SmokeSimParams {
        spec,
        source: s.sim_source(),
        source_radius: m.source_radius,
        emission: m.emission,
        temperature: m.temperature,
        buoyancy: m.buoyancy,
        weight: m.weight,
        exit_speed: m.exit_speed,
        cooling: m.cooling,
        dissipation: m.dissipation,
        vorticity: m.vorticity,
        turbulence: m.turbulence,
        turbulence_scale: m.turbulence_scale,
        wind: m.wind,
        time: m.time,
        seed: s.seed,
        edge_fade: m.edge_fade,
        box_center: s.center,
        box_yaw: s.rotation_deg.to_radians(),
        box_half: s.half(),
        // L'anteprima mostra il moto del fumo sulla griglia: le particelle servono al render.
        particles: false,
        patchiness: 0.0,
        emit_stop: m.emit_stop,
    };
    let moving = scene.and_then(|sc| crate::smoke_bake::preview_with_movers(sc, &s, &params, solid.as_deref(), PREVIEW_FRAMES, cancelled));
    let raw: Vec<(f32, Vec<f32>)> = match moving {
        Some(frames) => frames?,
        None => {
            let mut raw = Vec::new();
            simulate_smoke_frames(
                &params,
                solid.as_deref(),
                m.time / PREVIEW_FRAMES as f32,
                &mut |t, rho| raw.push((t, rho.to_vec())),
                cancelled,
            )?;
            raw
        }
    };
    // Riferimento comune a tutti i fotogrammi: il 90° percentile dell'ultimo (come il render).
    let reference = raw
        .last()
        .map(|(_, r)| {
            let mut v: Vec<f32> = r.iter().copied().filter(|&x| x > 1e-4).collect();
            if v.is_empty() {
                return 1.0;
            }
            let k = ((v.len() as f32 * 0.9) as usize).min(v.len() - 1);
            *v.select_nth_unstable_by(k, |a, b| a.total_cmp(b)).1
        })
        .unwrap_or(1.0)
        .max(1e-4);
    let h = spec.cell;
    let hash = |i: usize, k: u32| {
        let mut x = (i as u32).wrapping_mul(0x9E37_79B9) ^ k.wrapping_mul(0x85EB_CA6B) ^ s.seed;
        x ^= x >> 16;
        x = x.wrapping_mul(0x7FEB_352D);
        x ^= x >> 15;
        (x & 0xFFFF) as f32 / 65535.0
    };
    let frames = raw
        .into_iter()
        .map(|(t, rho)| {
            let mut pts = Vec::new();
            for (i, &r) in rho.iter().enumerate() {
                let d = (r / reference).min(1.5);
                if d < 0.04 {
                    continue;
                }
                let (x, y, z) = spec.coords(i);
                let c = spec.center(x, y, z);
                let d = d * s.edge_keep(c);
                if d < 0.04 {
                    continue;
                }
                // Uno o più punti per cella, sparsi dentro la cella (sempre negli stessi posti).
                let n = (d * 3.0).ceil() as u32;
                for k in 0..n {
                    let j = Vec3::new(hash(i, k * 3), hash(i, k * 3 + 1), hash(i, k * 3 + 2)) - Vec3::splat(0.5);
                    pts.push((c + j * h, d.min(1.0)));
                }
            }
            if pts.len() > PREVIEW_POINTS {
                let step = pts.len() as f32 / PREVIEW_POINTS as f32;
                pts = (0..PREVIEW_POINTS).map(|k| pts[(k as f32 * step) as usize]).collect();
            }
            (t, pts)
        })
        .collect();
    Some(SmokePreview { frames, cell: h })
}

fn bake_one(scene: &Scene, s: &SmokeCloud, geo: &RenderGeometry) -> Option<Arc<BakedFog>> {
    let k = key(s, geo);
    if let Some(b) = cache().lock().unwrap().get(&k) {
        return Some(b.clone());
    }
    let (lo, hi) = s.bounds();
    let ext = hi - lo;
    let (side, max_cells) = SMOKE_QUALITY[(s.quality.clamp(1, 4) - 1) as usize];
    let mut longest = side;
    loop {
        let cell = ext.max_element() / longest as f32;
        if (ext.x / cell).ceil() * (ext.y / cell).ceil() * (ext.z / cell).ceil() <= max_cells || longest <= 16 {
            break;
        }
        longest = (longest as f32 * 0.92) as usize;
    }
    let spec = GridSpec::covering(lo, hi, longest);
    progress::update(0.0);
    // Simulazione (se richiesta): la prima parte della barra.
    let simulated = s.mode == SmokeMode::Simulated;
    let frame = if simulated {
        Some(crate::smoke_bake::frame_ref(scene, s, geo, 0.0, 0.6)?)
    } else {
        None
    };
    // La griglia finale di questo fotogramma può essere già pronta: si legge e basta.
    let albedo = Vec3::from(s.color).clamp(Vec3::ZERO, Vec3::splat(0.999));
    let look = look_key(s, longest);
    if let Some(fog) = frame.as_ref().and_then(|f| f.load_render(look, albedo, s.anisotropy)) {
        tracing::debug!("fumo «{}»: griglia finale letta dal precalcolo", s.name);
        return Some(remember(k, fog));
    }
    let field = match &frame {
        Some(f) => Some(f.field()?),
        None => None,
    };
    let p0 = if simulated { 0.6 } else { 0.0 };
    let inside = if s.collisions && !geo.accel.is_empty() {
        let shell = voxelize(geo, &spec);
        progress::update(p0 + 0.1 * (1.0 - p0));
        interior_cells(geo, &spec, &shell)
    } else {
        Vec::new()
    };
    let p0 = p0 + 0.15 * (1.0 - p0);
    progress::update(p0);
    let done = std::sync::atomic::AtomicUsize::new(0);
    let n = spec.len().max(1);
    let sc = s.clone();
    let fog = BakedFog::from_fn(lo, hi, longest, albedo, s.anisotropy, |p| {
        let i = done.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if i.is_multiple_of(32_768) {
            progress::update(p0 + (1.0 - p0) * i as f32 / n as f32);
        }
        if !inside.is_empty() {
            if let Some((x, y, z)) = spec.cell_of(p) {
                if inside[spec.index(x, y, z)] {
                    return 0.0;
                }
            }
        }
        smoke_density_with(&sc, p, field.as_deref())
    });
    if progress::cancelled() {
        return None;
    }
    tracing::info!("fumo «{}»: {:?} celle in {:.0} ms", s.name, fog.spec.res, fog.bake_ms);
    if let Some(f) = &frame {
        f.save_render(look, &fog);
    }
    Some(remember(k, fog))
}

fn remember(k: String, fog: BakedFog) -> Arc<BakedFog> {
    let b = Arc::new(fog);
    let mut c = cache().lock().unwrap();
    if c.len() > 16 {
        c.clear();
    }
    c.insert(k, b.clone());
    b
}

/// Impronta di quello che decide la griglia finale oltre al fotogramma simulato: la sua risoluzione
/// e l'aspetto del fumo (l'istante no, sta già nel fotogramma).
fn look_key(s: &SmokeCloud, longest: usize) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut k = SmokeCloud {
        name: String::new(),
        n: 0,
        preview_only: false,
        ..s.clone()
    };
    k.sim.time = 0.0;
    k.sim.emit_stop = 0.0;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    format!("{k:?}|{longest}").hash(&mut h);
    h.finish()
}

/// Calcola le nubi accese, una alla volta sulla barra degli effetti: `first` è il numero della
/// prima su `total` effetti. Si ferma se il calcolo viene annullato.
pub fn bake_smokes(scene: &Scene, geo: &RenderGeometry, first: usize, total: usize) {
    for (k, s) in active(scene).enumerate() {
        if progress::cancelled() {
            return;
        }
        progress::begin(progress::Phase::Smoke, first + k, total);
        bake_one(scene, &s, geo);
    }
}

/// Nubi di fumo accese per il renderer (dalla cache, o calcolate qui).
pub fn smoke_fogs(scene: &Scene, geo: &RenderGeometry) -> Vec<Arc<BakedFog>> {
    active(scene).filter_map(|s| bake_one(scene, &s, geo)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Anteprima del fumo caldo simulato: fotogrammi nel tempo, con punti che salgono dalla
    /// sorgente (più in alto alla fine che all'inizio), calcolata in fretta.
    #[test]
    fn preview_shows_hot_smoke_rising_over_time() {
        let preset = forge_scene::SMOKE_PRESETS.iter().find(|p| p.name == "Fumo caldo simulato").unwrap();
        let mut s = (preset.make)();
        s.center = Vec3::new(0.0, s.size.y * 0.5, 0.0);
        let geo = RenderGeometry::build(&Scene::default());
        let t0 = std::time::Instant::now();
        let p = smoke_preview(&s, None, &geo, &|| false).expect("anteprima");
        let ms = t0.elapsed().as_millis();
        assert!(p.frames.len() >= PREVIEW_FRAMES / 2, "{} fotogrammi", p.frames.len());
        let top = |pts: &[(Vec3, f32)]| pts.iter().map(|q| q.0.y).fold(f32::MIN, f32::max);
        let early = &p.frames[p.frames.len() / 6].1;
        let late = &p.frames.last().unwrap().1;
        assert!(!late.is_empty() && late.len() <= PREVIEW_POINTS);
        assert!(top(late) > top(early) + 0.05, "il fumo non sale: {} → {}", top(early), top(late));
        assert!(ms < 5000, "anteprima lenta: {ms} ms");
        // Fotogramma per istante: il primo all'inizio, l'ultimo alla fine.
        assert!(std::ptr::eq(p.at(0.0).unwrap(), p.frames[0].1.as_slice()));
        assert!(std::ptr::eq(p.at(1e9).unwrap(), late.as_slice()));
    }

    /// Fumo simulato "solo anteprima": il render non lo calcola (e non tocca la chiave della cache,
    /// così riaccendendo il calcolo vero si riusa quello già fatto).
    #[test]
    fn preview_only_smoke_is_skipped_by_the_render() {
        let preset = forge_scene::SMOKE_PRESETS.iter().find(|p| p.name == "Fumo caldo simulato").unwrap();
        let mut scene = Scene::default();
        let mut s = (preset.make)();
        s.preview_only = true;
        scene.smokes.push(s.clone());
        let geo = RenderGeometry::build(&scene);
        let t0 = std::time::Instant::now();
        assert!(smoke_fogs(&scene, &geo).is_empty());
        assert!(t0.elapsed().as_millis() < 50, "niente simulazione");
        let on = SmokeCloud {
            preview_only: false,
            ..s.clone()
        };
        assert_eq!(key(&s, &geo), key(&on, &geo));
    }

    /// La stessa scena ricostruita dà la stessa impronta della geometria (il fumo e la nebbia già
    /// calcolati si riusano al render), una scena diversa no.
    #[test]
    fn same_scene_same_geometry_key() {
        let mut scene = Scene::default();
        let m = scene.add_mesh(forge_scene::primitives::uv_sphere(0.1, 16, 8));
        scene.add_node(None, forge_scene::ComponentId("s".into()), "S", forge_math::Mat4::IDENTITY, Some(m));
        let (a, b) = (RenderGeometry::build(&scene), RenderGeometry::build(&scene));
        assert_eq!(a.id, b.id);
        scene.nodes[0].user.translation += Vec3::new(0.01, 0.0, 0.0);
        assert_ne!(RenderGeometry::build(&scene).id, a.id);
    }

    /// Contrasto: con la stessa nube, alzandolo le parti diradate perdono densità molto più del
    /// cuore (più rilievo), e a 1 non cambia niente.
    #[test]
    fn contrast_thins_the_wisps_more_than_the_core() {
        let base = SmokeCloud {
            center: Vec3::new(0.0, 0.3, 0.0),
            ..Default::default()
        };
        let pts: Vec<Vec3> = (0..4000)
            .map(|i| {
                let a = i as f32 * 0.618_034;
                let r = (i as f32 / 4000.0).sqrt() * 0.3;
                base.center
                    + Vec3::new(
                        r * (a * std::f32::consts::TAU).cos(),
                        ((i * 7919) % 1000) as f32 / 1000.0 * 0.5 - 0.25,
                        r * (a * std::f32::consts::TAU).sin(),
                    )
            })
            .collect();
        let field = |k: f32| -> Vec<f32> {
            let s = SmokeCloud {
                contrast: k,
                ..base.clone()
            };
            pts.iter().map(|&p| smoke_density_at(&s, p)).collect()
        };
        let (one, high) = (field(1.0), field(3.0));
        assert_eq!(one, field(1.0));
        let core = |d: &[f32]| d.iter().copied().fold(0.0f32, f32::max);
        let thin = |d: &[f32]| {
            let m = core(&one);
            let idx: Vec<usize> = (0..one.len()).filter(|&i| one[i] > 0.0 && one[i] < m * 0.3).collect();
            idx.iter().map(|&i| d[i]).sum::<f32>() / idx.len().max(1) as f32
        };
        let (rc, rt) = (core(&high) / core(&one), thin(&high) / thin(&one));
        eprintln!("cuore ×{rc:.2}, parti diradate ×{rt:.3}");
        assert!(rt < 0.2, "le parti diradate si fanno trasparenti: ×{rt}");
        assert!(rc > rt * 4.0, "il cuore resta denso: ×{rc}");
    }

    /// Vapore da tazza: sale sopra la metà del riquadro in filamenti (riempie poco del riquadro)
    /// e non si ammassa sotto il soffitto (bordi sfumati).
    #[test]
    fn cup_steam_rises_in_thin_wisps() {
        let preset = forge_scene::SMOKE_PRESETS.iter().find(|p| p.name == "Vapore da tazza").unwrap();
        let mut s = (preset.make)();
        s.center = Vec3::new(0.0, s.size.y * 0.5, 0.0);
        s.sim.quality = 1;
        let geo = RenderGeometry::build(&Scene::default());
        let f = simulated_field(&s, &geo, 0.0, 1.0).expect("simulazione");
        let ref_d = f.max.max(1e-6);
        let ry = f.spec.res[1];
        let (mut full, mut high, mut ceiling) = (0usize, 0.0f32, 0.0f32);
        for i in 0..f.spec.len() {
            let (_, y, _) = f.spec.coords(i);
            let d = f.density[i] / ref_d;
            if d > 0.1 {
                full += 1;
            }
            if y > ry / 2 {
                high += d;
            }
            if y + 1 == ry {
                ceiling += d;
            }
        }
        let frac = full as f32 / f.spec.len() as f32;
        eprintln!(
            "celle con vapore {:.1} %, sopra la metà {high:.1}, sotto il soffitto {ceiling:.3}",
            frac * 100.0
        );
        assert!(high > 1.0, "il vapore sale sopra la metà");
        assert!(frac < 0.35, "filamenti, non un blocco: {frac}");
        assert!(ceiling < high * 0.02, "niente ammasso sotto il soffitto");
    }

    /// Nube sospesa: piena al centro, vuota fuori dal riquadro e sugli angoli, con il bordo
    /// irregolare (sbuffi), e spostando il riquadro la nube si sposta identica.
    #[test]
    fn smoke_cloud_has_a_core_and_follows_its_box() {
        let s = SmokeCloud {
            center: Vec3::new(0.0, 1.0, 0.0),
            size: Vec3::splat(1.0),
            ..Default::default()
        };
        assert!(smoke_density_at(&s, s.center) > 5.0, "cuore");
        assert_eq!(smoke_density_at(&s, Vec3::new(0.0, 2.0, 0.0)), 0.0, "fuori");
        assert_eq!(smoke_density_at(&s, Vec3::new(0.49, 1.49, 0.49)), 0.0, "angolo");
        // Bordo irregolare: sulla sfera di raggio 0,4 la densità varia molto.
        let vals: Vec<f32> = (0..64)
            .map(|i| {
                let a = i as f32 / 64.0 * std::f32::consts::TAU;
                smoke_density_at(&s, s.center + Vec3::new(a.cos(), 0.2, a.sin()) * 0.4)
            })
            .collect();
        let (lo, hi) = vals.iter().fold((f32::MAX, 0.0f32), |(a, b), v| (a.min(*v), b.max(*v)));
        assert!(hi - lo > 5.0, "bordo uniforme: {lo}..{hi}");
        let moved = SmokeCloud {
            center: s.center + Vec3::new(3.0, 0.5, -2.0),
            ..s.clone()
        };
        for p in [Vec3::ZERO, Vec3::new(0.3, -0.2, 0.1), Vec3::new(-0.35, 0.3, 0.2)] {
            let a = smoke_density_at(&s, s.center + p);
            let b = smoke_density_at(&moved, moved.center + p);
            assert!((a - b).abs() < 1e-3, "{a} vs {b}");
        }
    }

    /// Pennacchio: stretto e denso in basso, largo in alto; quote di partenza e di arrivo tagliano.
    #[test]
    fn plume_widens_and_height_limits_cut() {
        let s = SmokeCloud {
            shape: SmokeShape::Plume,
            center: Vec3::new(0.0, 1.0, 0.0),
            size: Vec3::new(1.0, 2.0, 1.0),
            meander: 0.0,
            turbulence: 0.0,
            billows: 0.0,
            wisps: 0.0,
            variation: 0.0,
            fade: 0.0,
            ..Default::default()
        };
        let at = |y: f32, x: f32| smoke_density_at(&s, Vec3::new(x, y, 0.0));
        assert!(at(0.2, 0.0) > 1.0, "asse in basso");
        assert_eq!(at(0.2, 0.4), 0.0, "in basso è stretto");
        assert!(at(1.6, 0.25) > 1.0, "in alto è largo");
        let cut = SmokeCloud {
            bottom_height: 0.5,
            top_height: 1.2,
            ..s.clone()
        };
        let cut_at = |y: f32| smoke_density_at(&cut, Vec3::new(0.0, y, 0.0));
        assert_eq!(cut_at(0.3), 0.0, "sotto la partenza");
        assert!(cut_at(0.9) > 1.0, "nella fascia");
        assert_eq!(cut_at(1.5), 0.0, "sopra l'arrivo");
    }
}
