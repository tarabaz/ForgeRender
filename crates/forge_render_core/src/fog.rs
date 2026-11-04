//! Nebbie volumetriche nella scena compilata: voxelizzazione degli oggetti
//! (collisioni) dalla BVH e cache dei volumi calcolati.
//!
//! Il calcolo di una nebbia richiede da qualche secondo a qualche minuto: viene rifatto
//! solo quando cambiano i suoi parametri o gli oggetti dentro la sua zona.

use crate::RenderGeometry;
use forge_math::Vec3;
use forge_scene::{FogVolume, Scene};
/// Avanzamento del calcolo della nebbia (per la barra nell'interfaccia).
pub use forge_volume::progress;
use forge_volume::{BakedFog, GridSpec, SimState};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// Celle della griglia attraversate da una superficie della scena: raggi lungo i
/// tre assi per ogni riga di celle, ogni intersezione marca la cella che la contiene.
/// Il guscio ottenuto separa interno ed esterno degli oggetti (connessione per facce).
pub fn voxelize(geo: &RenderGeometry, spec: &GridSpec) -> Vec<bool> {
    use rayon::prelude::*;
    let mut solid = vec![false; spec.len()];
    if geo.accel.is_empty() {
        return solid;
    }
    let max = spec.max();
    for axis in 0..3 {
        let (a1, a2) = ((axis + 1) % 3, (axis + 2) % 3);
        let rows: Vec<(usize, usize)> = (0..spec.res[a2]).flat_map(|j| (0..spec.res[a1]).map(move |i| (i, j))).collect();
        let hits: Vec<Vec<usize>> = rows
            .par_iter()
            .map(|&(i, j)| {
                let mut o = spec.min;
                o[a1] += (i as f32 + 0.5) * spec.cell;
                o[a2] += (j as f32 + 0.5) * spec.cell;
                o[axis] = spec.min[axis] - spec.cell;
                let mut d = Vec3::ZERO;
                d[axis] = 1.0;
                let t_end = max[axis] - o[axis] + spec.cell;
                let mut cells = Vec::new();
                let mut t = 0.0f32;
                let step = spec.cell * 1e-3;
                while let Some(h) = geo.accel.intersect(o, d, t, t_end) {
                    if let Some((x, y, z)) = spec.cell_of(o + d * h.t) {
                        cells.push(spec.index(x, y, z));
                    }
                    t = h.t + step;
                }
                cells
            })
            .collect();
        for c in hits.into_iter().flatten() {
            solid[c] = true;
        }
    }
    solid
}

/// Celle *interne* agli oggetti (il guscio `shell` di [`voxelize`] escluso): le zone di
/// celle libere non collegate al bordo della griglia sono chiuse da superfici; ognuna si
/// classifica con 6 raggi lungo gli assi da una sua cella: è interna se la prima superficie
/// incontrata mostra il retro in almeno 4 direzioni. Così l'aria di una stanza chiusa
/// (pareti viste dal davanti) resta aria, il pieno di un oggetto no.
/// Serve alla foschia (niente foschia dentro i pezzi) e a una sua futura simulazione.
pub fn interior_cells(geo: &RenderGeometry, spec: &GridSpec, shell: &[bool]) -> Vec<bool> {
    let n = spec.len();
    let mut inside = vec![false; n];
    if geo.accel.is_empty() || shell.len() != n {
        return inside;
    }
    let mut seen = vec![false; n];
    let [rx, ry, rz] = spec.res;
    let on_border = |x: usize, y: usize, z: usize| x == 0 || y == 0 || z == 0 || x + 1 == rx || y + 1 == ry || z + 1 == rz;
    let mut stack = Vec::new();
    let mut comp = Vec::new();
    for start in 0..n {
        if seen[start] || shell[start] {
            continue;
        }
        // Zona connessa (6 vicini) di celle libere.
        comp.clear();
        let mut open = false;
        seen[start] = true;
        stack.push(start);
        while let Some(i) = stack.pop() {
            comp.push(i);
            let (x, y, z) = spec.coords(i);
            open |= on_border(x, y, z);
            let mut visit = |x: usize, y: usize, z: usize| {
                let j = spec.index(x, y, z);
                if !seen[j] && !shell[j] {
                    seen[j] = true;
                    stack.push(j);
                }
            };
            if x > 0 {
                visit(x - 1, y, z);
            }
            if x + 1 < rx {
                visit(x + 1, y, z);
            }
            if y > 0 {
                visit(x, y - 1, z);
            }
            if y + 1 < ry {
                visit(x, y + 1, z);
            }
            if z > 0 {
                visit(x, y, z - 1);
            }
            if z + 1 < rz {
                visit(x, y, z + 1);
            }
        }
        if open {
            continue;
        }
        let (x, y, z) = spec.coords(comp[comp.len() / 2]);
        let p = spec.center(x, y, z);
        let mut back = 0;
        for axis in 0..3 {
            for s in [-1.0f32, 1.0] {
                let mut d = Vec3::ZERO;
                d[axis] = s;
                if let Some(h) = geo.accel.intersect(p, d, 0.0, f32::INFINITY) {
                    if geo.geometric_normals[h.prim as usize].dot(d) > 0.0 {
                        back += 1;
                    }
                }
            }
        }
        if back >= 4 {
            for &i in &comp {
                inside[i] = true;
            }
        }
    }
    inside
}

type Cache<T> = Mutex<HashMap<(String, u64), Arc<T>>>;

/// Nebbie pronte (simulazione + dettaglio), per tutti i parametri.
fn cache() -> &'static Cache<BakedFog> {
    static C: OnceLock<Cache<BakedFog>> = OnceLock::new();
    C.get_or_init(Default::default)
}

/// Simulazioni, per i soli parametri che cambiano il moto: cambiare densità, colore,
/// anisotropia o scala del dettaglio rifà solo il dettaglio (frazioni di secondo).
fn sim_cache() -> &'static Cache<SimState> {
    static C: OnceLock<Cache<SimState>> = OnceLock::new();
    C.get_or_init(Default::default)
}

/// Superficie su cui si appoggia la nebbia sotto la sua zona: la più alta tra quelle che
/// stanno sotto il fondo della zona, cercata con raggi verso il basso su una griglia 3×3
/// attraverso la zona. `None` se la zona arriva già sulle superfici (o sotto non c'è nulla).
///
/// Non si usa il punto più basso della scena: in una scena con un bancone alto un metro
/// la zona si allungherebbe fino al pavimento e le celle diventerebbero grossolane.
pub fn ground_under(f: &FogVolume, geo: &RenderGeometry) -> Option<f32> {
    if geo.accel.is_empty() {
        return None;
    }
    let f = f.sanitized();
    let (lo, hi) = f.bounds();
    let mut ground: Option<f32> = None;
    for i in 0..3 {
        for j in 0..3 {
            let x = lo.x + (hi.x - lo.x) * (0.1 + 0.4 * i as f32);
            let z = lo.z + (hi.z - lo.z) * (0.1 + 0.4 * j as f32);
            let o = Vec3::new(x, f.emitter.y.max(lo.y), z);
            let Some(h) = geo.accel.intersect(o, -Vec3::Y, 0.0, f32::INFINITY) else {
                continue;
            };
            let y = o.y - h.t;
            if y < lo.y {
                ground = Some(ground.map_or(y, |g: f32| g.max(y)));
            }
        }
    }
    ground
}

/// Ostacoli "visti" da una nebbia: il suolo sotto la zona e le celle occupate dagli
/// oggetti sulle sue due griglie, con un'impronta del contenuto. La simulazione si rifà
/// solo se l'impronta cambia: spostare un oggetto lontano dalla nebbia non la tocca.
struct Obstacles {
    ground: Option<f32>,
    specs: (GridSpec, GridSpec),
    sim: Vec<bool>,
    fine: Vec<bool>,
    hash: u64,
}

impl Obstacles {
    /// Celle solide per la griglia richiesta (quelle già calcolate, se è una delle due).
    fn solid(&self, geo: &RenderGeometry, spec: &GridSpec) -> Vec<bool> {
        if *spec == self.specs.0 {
            self.sim.clone()
        } else if *spec == self.specs.1 {
            self.fine.clone()
        } else {
            voxelize(geo, spec)
        }
    }
}

type ObstacleCache = Mutex<HashMap<(u64, String), Arc<Obstacles>>>;

fn obstacle_cache() -> &'static ObstacleCache {
    static C: OnceLock<ObstacleCache> = OnceLock::new();
    C.get_or_init(Default::default)
}

fn obstacles(f: &FogVolume, geo: &RenderGeometry) -> Arc<Obstacles> {
    let f = f.sanitized();
    // Solo i parametri che decidono zona e griglie.
    let zk = format!(
        "{:?} {:?} {:?} {} {} {}",
        f.emitter, f.size, f.zone_offset, f.resolution, f.sim_cells, f.collisions
    );
    let k = (geo.id, zk);
    if let Some(o) = obstacle_cache().lock().unwrap().get(&k) {
        return o.clone();
    }
    let o = if f.collisions {
        let ground = ground_under(&f, geo);
        let specs = forge_volume::grid_specs(&f, ground);
        let sim = voxelize(geo, &specs.0);
        let fine = voxelize(geo, &specs.1);
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        ground.map(f32::to_bits).hash(&mut h);
        sim.hash(&mut h);
        fine.hash(&mut h);
        // 0 è riservato a "nessuna collisione".
        let hash = h.finish().max(1);
        Obstacles {
            ground,
            specs,
            sim,
            fine,
            hash,
        }
    } else {
        let specs = forge_volume::grid_specs(&f, None);
        Obstacles {
            ground: None,
            specs,
            sim: Vec::new(),
            fine: Vec::new(),
            hash: 0,
        }
    };
    let o = Arc::new(o);
    let mut c = obstacle_cache().lock().unwrap();
    if c.len() > 8 {
        c.clear();
    }
    c.insert(k, o.clone());
    o
}

fn key(f: &FogVolume, geo: &RenderGeometry) -> (String, u64) {
    let f = f.sanitized();
    // Il nome non cambia la nebbia.
    let k = format!(
        "{:?}",
        FogVolume {
            name: String::new(),
            ..f.clone()
        }
    );
    (k, obstacles(&f, geo).hash)
}

/// Nebbie attive della scena già calcolate (senza calcolarne di nuove)?
pub fn fogs_ready(scene: &Scene, geo: &RenderGeometry) -> bool {
    let fogs_done = {
        let c = cache().lock().unwrap();
        scene.fogs.iter().filter(|f| f.enabled).all(|f| c.contains_key(&key(f, geo)))
    };
    fogs_done
        && crate::smoke::smokes_ready(scene, geo)
        && match haze_params(scene, geo) {
            Some(p) => haze_cache().lock().unwrap().contains_key(&p.key),
            None => true,
        }
}

/// Nebbie attive della scena, calcolate se non sono già in cache. Gli effetti si calcolano
/// uno alla volta (ognuno usa già tutti i core), sulla stessa barra che riparte da zero:
/// prima le nebbie di ghiaccio secco, poi il fumo volumetrico, poi la foschia d'ambiente (fumo e
/// foschia restano in cache per [`crate::smoke::smoke_fogs`] e [`haze_fog`]). Restituisce le
/// nebbie di ghiaccio secco.
pub fn bake_fogs(scene: &Scene, geo: &RenderGeometry) -> Vec<Arc<BakedFog>> {
    use forge_volume::progress::{self, Phase};
    progress::start_job();
    let mut out = Vec::new();
    let fogs: Vec<&FogVolume> = scene.fogs.iter().filter(|f| f.enabled).collect();
    let smokes = crate::smoke::smoke_count(scene);
    let total = fogs.len() + smokes + haze_on(scene) as usize;
    for (n, f) in fogs.iter().enumerate() {
        progress::begin(Phase::Obstacles, n + 1, total);
        let k = key(f, geo);
        if let Some(b) = cache().lock().unwrap().get(&k) {
            out.push(b.clone());
            continue;
        }
        let obs = obstacles(f, geo);
        progress::set(Phase::Simulation, 0.0);
        let solid = |spec: &GridSpec| obs.solid(geo, spec);
        let ground = obs.ground;
        let sk = (forge_volume::sim_key(f), obs.hash);
        let cached_sim = sim_cache().lock().unwrap().get(&sk).cloned();
        let sim = match cached_sim {
            Some(s) => s,
            None => {
                let s = Arc::new(forge_volume::simulate_fog(f, Some(&solid), ground));
                // Annullato (dall'utente o perché la nebbia è cambiata): niente cache, si esce.
                if progress::cancelled() {
                    break;
                }
                let mut c = sim_cache().lock().unwrap();
                if c.len() > 8 {
                    c.clear();
                }
                c.insert(sk, s.clone());
                s
            }
        };
        let b = Arc::new(forge_volume::bake_detail(f, &sim, Some(&solid), ground));
        if progress::cancelled() {
            break;
        }
        let mut c = cache().lock().unwrap();
        if c.len() > 16 {
            c.clear();
        }
        c.insert(k, b.clone());
        out.push(b);
    }
    if !progress::cancelled() {
        crate::smoke::bake_smokes(scene, geo, fogs.len() + 1, total);
    }
    if haze_on(scene) && !progress::cancelled() {
        progress::begin(Phase::Haze, total, total);
        haze_fog(scene, geo);
    }
    progress::clear();
    out
}

/// Lunghezza del fascio visibile di una spot: quella impostata, oppure il raggio d'azione, oppure 3 m.
pub fn beam_length(l: &forge_scene::Light) -> f32 {
    if l.beam_length > 0.0 {
        l.beam_length
    } else if l.range > 0.0 {
        l.range
    } else {
        3.0
    }
}

/// Densità della foschia del fascio nel punto `p` (1/m): dentro il cono della spot, con il bordo
/// sfumato tra l'angolo interno e quello esterno e la fine sfumata nell'ultimo quinto.
pub fn beam_density_at(l: &forge_scene::Light, p: Vec3) -> f32 {
    let forge_scene::LightKind::Spot {
        inner_angle_deg,
        outer_angle_deg,
    } = l.kind
    else {
        return 0.0;
    };
    let len = beam_length(l);
    let dir = l.direction();
    let v = p - l.position;
    let along = v.dot(dir);
    if along <= 0.0 || along > len {
        return 0.0;
    }
    let outer = (outer_angle_deg.clamp(0.1, 179.0) * 0.5).to_radians();
    let inner = (inner_angle_deg.clamp(0.0, outer_angle_deg) * 0.5).to_radians().min(outer - 1e-3);
    let angle = (v - dir * along).length().atan2(along);
    let smooth = |e0: f32, e1: f32, x: f32| {
        let t = ((x - e0) / (e1 - e0).max(1e-6)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    let edge = 1.0 - smooth(inner, outer, angle);
    let end = 1.0 - smooth(0.8 * len, len, along);
    l.beam_density * edge * end
}

/// Riquadro che contiene il cono del fascio.
fn beam_bounds(l: &forge_scene::Light) -> (Vec3, Vec3) {
    let outer_half = match l.kind {
        forge_scene::LightKind::Spot { outer_angle_deg, .. } => (outer_angle_deg.clamp(0.1, 179.0) * 0.5).to_radians(),
        _ => 0.0,
    };
    let len = beam_length(l);
    let dir = l.direction();
    let base = l.position + dir * len;
    let radius = len * outer_half.min(1.4).tan();
    let e = Vec3::new(
        (1.0 - dir.x * dir.x).max(0.0).sqrt(),
        (1.0 - dir.y * dir.y).max(0.0).sqrt(),
        (1.0 - dir.z * dir.z).max(0.0).sqrt(),
    ) * radius;
    ((base - e).min(l.position), (base + e).max(l.position))
}

fn beam_cache() -> &'static Mutex<HashMap<String, Arc<BakedFog>>> {
    static C: OnceLock<Mutex<HashMap<String, Arc<BakedFog>>>> = OnceLock::new();
    C.get_or_init(Default::default)
}

/// Fasci visibili delle spot accese con `beam_density` > 0: una foschia a forma di cono,
/// illuminata come le altre nebbie (dalla spot stessa e dalle altre luci, con le ombre).
pub fn beam_fogs(scene: &Scene) -> Vec<Arc<BakedFog>> {
    let mut out = Vec::new();
    for l in &scene.lights {
        if !l.enabled || l.beam_density <= 0.0 || !matches!(l.kind, forge_scene::LightKind::Spot { .. }) {
            continue;
        }
        let key = format!(
            "{:?}|{:?}|{:?}|{}|{}|{}",
            l.position,
            l.target,
            l.kind,
            l.beam_density,
            beam_length(l),
            l.range
        );
        if let Some(b) = beam_cache().lock().unwrap().get(&key) {
            out.push(b.clone());
            continue;
        }
        let (min, max) = beam_bounds(l);
        // Celle: abbastanza per il diametro del fascio (bordo morbido) senza superare ~8 milioni.
        let ext = max - min;
        let outer_half = match l.kind {
            forge_scene::LightKind::Spot { outer_angle_deg, .. } => (outer_angle_deg * 0.5).to_radians(),
            _ => 0.5,
        };
        let diameter = 2.0 * beam_length(l) * outer_half.min(1.4).tan();
        let mut cell = (diameter / 24.0).min(beam_length(l) / 48.0).max(ext.max_element() / 256.0);
        while (ext.x / cell).ceil() * (ext.y / cell).ceil() * (ext.z / cell).ceil() > 8.0e6 {
            cell *= 1.25;
        }
        let longest = ((ext.max_element() / cell).ceil() as usize).clamp(8, 256);
        let lc = l.clone();
        let mut fog = BakedFog::from_fn(min, max, longest, Vec3::ONE, 0.3, move |p| beam_density_at(&lc, p));
        // Fascio da studio: lo vede la camera (diffusione singola), non illumina la scena.
        fog.camera_only = true;
        let b = Arc::new(fog);
        tracing::info!("fascio volumetrico «{}»: {:?} celle in {:.0} ms", l.name, b.spec.res, b.bake_ms);
        let mut c = beam_cache().lock().unwrap();
        if c.len() > 16 {
            c.clear();
        }
        c.insert(key, b.clone());
        out.push(b);
    }
    out
}

/// Densità della foschia d'ambiente nel punto `p` (1/m), secondo il tipo: rumore frattale a
/// dominio distorto (riccioli, non macchie) con ottave, ruvidità e forza dei vortici scelte.
pub fn haze_density_at(h: &forge_scene::Haze, p: Vec3, floor: f32) -> f32 {
    use forge_scene::HazeKind;
    let top = top_factor(h, p, floor) * bottom_factor(h, p, floor);
    if top <= 0.0 {
        return 0.0;
    }
    // Altezza di partenza: strati, diradamento e fumo a terra si misurano da lì.
    let floor = floor + h.bottom_height.max(0.0);
    let d = match h.kind {
        HazeKind::Ambient => ambient_density(h, p, floor),
        HazeKind::GasClouds => gas_cloud_density(h, p, floor),
        HazeKind::ToxicFloor => toxic_floor_density(h, p, floor),
        HazeKind::Layers => layer_density(h, p, floor),
        HazeKind::Billows => billow_density(h, p, floor),
    };
    if d <= 0.0 {
        return 0.0;
    }
    d * clump_factor(h, p) * top
}

/// Addensamenti: un rumore largo (3,5 volte i vortici) rende alcune zone più dense e altre più
/// tenui; la media resta circa 1.
/// Oltre 1 gli ammassi diventano più rari e più concentrati (fino a 2).
fn clump_factor(h: &forge_scene::Haze, p: Vec3) -> f32 {
    let c = h.clumping.clamp(0.0, 2.0);
    if c <= 0.0 {
        return 1.0;
    }
    let q = p / (h.swirl_scale.max(0.005) * 3.5) + Vec3::new(11.3, -4.1, 7.7);
    let n = forge_volume::noise::fbm(q, h.seed ^ 0x3c6e_f372, 3);
    let m = smoothstep(-0.25, 0.25, n);
    let k = c.min(1.0);
    let extra = (c - 1.0).max(0.0);
    // Esponente più alto = ammassi più rari; il guadagno compensa per tenere la media vicina.
    let s = 1.5 + 3.0 * extra;
    (1.0 - k) + k * 2.5 * (1.0 + 1.6 * extra) * m.powf(s)
}

/// Altezza di partenza: niente foschia sotto `bottom_height` dal pavimento, con l'inizio sfumato
/// e (se ci sono sbuffi) qualche sbuffo che pende sotto.
fn bottom_factor(h: &forge_scene::Haze, p: Vec3, floor: f32) -> f32 {
    let bottom = h.bottom_height;
    if bottom <= 0.0 {
        return 1.0;
    }
    let y = p.y - floor;
    let w = (bottom * 0.1).clamp(0.02, 0.25);
    let edge = smoothstep(bottom, bottom + w, y);
    let puffs = h.puffs.clamp(0.0, 1.0);
    if y >= bottom + w || puffs <= 0.0 {
        return edge;
    }
    let s = h.swirl_scale.max(0.005);
    let q = Vec3::new(p.x / s, (bottom - y) / (s * 2.0), p.z / s) * 1.3 + Vec3::new(8.1, 0.0, -2.9);
    let n = forge_volume::noise::fbm(q, h.seed ^ 0x85eb_ca6b, 3);
    let blob = smoothstep(0.32 - 0.3 * puffs, 0.4, n);
    let fade = (-(bottom - y).max(0.0) / (0.5 * s + 0.2 * bottom)).exp();
    edge.max(blob * fade)
}

/// Altezza massima: piena fino al 90 % di `top_height`, sfuma fino all'altezza massima; sopra
/// restano solo gli sbuffi, macchie sparse che salgono e si spengono salendo.
fn top_factor(h: &forge_scene::Haze, p: Vec3, floor: f32) -> f32 {
    let top = h.top_height;
    if top <= 0.0 {
        return 1.0;
    }
    let y = p.y - floor;
    let edge = 1.0 - smoothstep(top * 0.9, top, y);
    let puffs = h.puffs.clamp(0.0, 1.0);
    if y <= top * 0.9 || puffs <= 0.0 {
        return edge;
    }
    let s = h.swirl_scale.max(0.005);
    let q = Vec3::new(p.x / s, (y - top) / (s * 2.0), p.z / s) * 1.3 + Vec3::new(-3.7, 0.0, 5.3);
    let n = forge_volume::noise::fbm(q, h.seed ^ 0x9e37_79b9, 3);
    let blob = smoothstep(0.32 - 0.3 * puffs, 0.4, n);
    let fade = (-(y - top).max(0.0) / (0.5 * s + 0.3 * top)).exp();
    edge.max(blob * fade)
}

/// Frattale della foschia nel punto (già diviso per la grandezza dei vortici): ottave e
/// ruvidità dalla foschia.
fn haze_fbm(h: &forge_scene::Haze, q: Vec3, seed: u32) -> f32 {
    forge_volume::noise::fbm_gain(q, seed, h.octaves.clamp(1, 10), h.roughness.clamp(0.2, 0.85))
}

/// Distorsione del dominio: spostare il punto con altro rumore fa i vortici (ampiezza ≈ 1 per
/// unità di "forza dei vortici").
fn haze_warp(q: Vec3, s: u32) -> Vec3 {
    use forge_volume::noise::fbm;
    Vec3::new(
        fbm(q + Vec3::new(5.2, 1.3, 9.1), s ^ 0x68e3_1da4, 3),
        fbm(q + Vec3::new(-7.7, 3.9, 2.4), s ^ 0x1b56_c4e9, 3),
        fbm(q + Vec3::new(1.7, -9.2, 6.3), s ^ 0x2f8a_7d11, 3),
    ) * 2.5
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Aria velata con banchi e riccioli (anche la nebbia bassa, che si dirada salendo).
fn ambient_density(h: &forge_scene::Haze, p: Vec3, floor: f32) -> f32 {
    let base = h.density_at(p.y, floor);
    let v = h.variation.clamp(0.0, 1.0);
    if v <= 0.0 || base <= 0.0 {
        return base;
    }
    let q = p / h.swirl_scale.max(0.005);
    let n = haze_fbm(h, q + haze_warp(q, h.seed) * h.warp, h.seed);
    // Banchi: da quasi vuoto (n basso) a circa il doppio della base; media ≈ la base.
    let shape = ((n * 2.2 + 0.5).clamp(0.0, 1.0)).powf(1.6) * 2.3;
    base * ((1.0 - v) + v * shape)
}

/// Nuvole di gas: aria quasi limpida (3 % della densità) e ammassi densi dove il rumore
/// supera una soglia; `variation` decide quanta aria coprono (0 = pochi ammassi, 1 = metà).
fn gas_cloud_density(h: &forge_scene::Haze, p: Vec3, floor: f32) -> f32 {
    let base = h.density_at(p.y, floor);
    if base <= 0.0 {
        return 0.0;
    }
    let q = p / h.swirl_scale.max(0.005);
    let n = haze_fbm(h, q + haze_warp(q, h.seed) * h.warp, h.seed);
    let threshold = 0.22 - 0.3 * h.variation.clamp(0.0, 1.0);
    base * (0.03 + 0.97 * smoothstep(threshold, threshold + 0.15, n))
}

/// Fumo tossico a terra: uno strato spesso `falloff_height` sopra il punto più basso, che sfuma
/// subito sopra; dentro, filamenti allungati (creste del rumore, stirato lungo x).
/// `variation` = quanto il fumo è raccolto in filamenti invece che steso uniforme.
fn toxic_floor_density(h: &forge_scene::Haze, p: Vec3, floor: f32) -> f32 {
    let thick = h.falloff_height.max(0.01);
    let y = p.y - floor;
    let profile = if y <= thick { 1.0 } else { (-(y - thick) / (0.5 * thick)).exp() };
    let d = h.density.max(0.0) * profile;
    if d <= 1e-6 {
        return 0.0;
    }
    let sc = h.swirl_scale.max(0.005);
    let q = Vec3::new(p.x / (sc * 4.0), p.y / sc, p.z / sc);
    let n = haze_fbm(h, q + haze_warp(q, h.seed) * h.warp, h.seed);
    let ridge = (1.0 - (n * 3.0).abs()).max(0.0).powi(3);
    let v = h.variation.clamp(0.0, 1.0);
    d * ((1.0 - v) * 0.3 + v * ridge * 1.6)
}

/// Strati sospesi: veli orizzontali a distanza `falloff_height` l'uno dall'altro, ondulati dal
/// rumore e bucati dove il rumore è basso.
fn layer_density(h: &forge_scene::Haze, p: Vec3, floor: f32) -> f32 {
    let d = h.density.max(0.0);
    if d <= 0.0 {
        return 0.0;
    }
    let spacing = h.falloff_height.max(0.05);
    let q = p / h.swirl_scale.max(0.005);
    let wave = haze_fbm(h, q * 0.5 + haze_warp(q * 0.5, h.seed) * h.warp * 0.5, h.seed ^ 0x51ed_270b);
    let phase = (p.y - floor) / spacing + wave * 1.5;
    let sheet = (0.5 + 0.5 * (std::f32::consts::TAU * phase).cos()).powi(6);
    let n = haze_fbm(h, q + haze_warp(q, h.seed) * h.warp, h.seed);
    let holes = (n * 1.6 + 0.5).clamp(0.0, 1.0);
    let v = h.variation.clamp(0.0, 1.0);
    d * (0.06 + sheet * 3.0 * ((1.0 - v) + v * holes))
}

/// Riccioli di vapore: distorsione applicata due volte (vortici dentro i vortici) e contrasto
/// alto, così i riccioli si vedono netti.
fn billow_density(h: &forge_scene::Haze, p: Vec3, floor: f32) -> f32 {
    let base = h.density_at(p.y, floor);
    if base <= 0.0 {
        return 0.0;
    }
    let q = p / h.swirl_scale.max(0.005);
    let w1 = haze_warp(q, h.seed);
    let w2 = haze_warp(q + w1 * h.warp + Vec3::new(3.1, 7.4, 1.9), h.seed ^ 0x7f4a_7c15);
    let n = haze_fbm(h, q + w2 * h.warp, h.seed);
    let v = h.variation.clamp(0.0, 1.0);
    let shape = smoothstep(-0.12, 0.28, n).powi(2) * 2.6;
    base * ((1.0 - v) * 0.5 + v * shape)
}

fn haze_cache() -> &'static Mutex<HashMap<String, Arc<BakedFog>>> {
    static C: OnceLock<Mutex<HashMap<String, Arc<BakedFog>>>> = OnceLock::new();
    C.get_or_init(Default::default)
}

/// Qualità della foschia: (celle per vortice, celle minime per lato, celle massime per lato,
/// celle al massimo).
const HAZE_QUALITY: [(f32, f32, f32, f32); 4] = [
    (6.0, 96.0, 128.0, 2.0e6),
    (10.0, 160.0, 224.0, 6.0e6),
    (14.0, 256.0, 352.0, 14.0e6),
    (18.0, 384.0, 512.0, 24.0e6),
];

/// La foschia d'ambiente è accesa?
pub fn haze_on(scene: &Scene) -> bool {
    scene.haze.enabled && scene.haze.density > 0.0
}

/// Riquadro e chiave di cache della foschia (None se è spenta).
struct HazeParams {
    min: Vec3,
    max: Vec3,
    /// Quota del pavimento (da cui si misurano strati, diradamento e altezza massima).
    floor: f32,
    key: String,
}

/// Quota del pavimento sotto la scena: la più bassa delle superfici rivolte in alto colpite da 5×5
/// raggi verso il basso nel riquadro (il piano d'appoggio, non il fondo del suo spessore). Solo se
/// almeno metà dei raggi trova una superficie (un pavimento copre tutto, un oggetto no);
/// altrimenti `fallback`.
fn haze_floor(geo: &RenderGeometry, min: Vec3, max: Vec3, fallback: f32) -> f32 {
    if geo.accel.is_empty() {
        return fallback;
    }
    let mut floor = f32::INFINITY;
    let mut found = 0;
    for i in 0..5 {
        for j in 0..5 {
            let x = min.x + (max.x - min.x) * (i as f32 + 0.5) / 5.0;
            let z = min.z + (max.z - min.z) * (j as f32 + 0.5) / 5.0;
            let o = Vec3::new(x, max.y + 0.01, z);
            if let Some(h) = geo.accel.intersect(o, Vec3::NEG_Y, 0.0, f32::INFINITY) {
                // Solo superfici rivolte in alto (il piano su cui si cammina).
                if geo.geometric_normals[h.prim as usize].y > 0.5 {
                    floor = floor.min(o.y - h.t);
                    found += 1;
                }
            }
        }
    }
    if found >= 13 {
        floor
    } else {
        fallback
    }
}

fn haze_params(scene: &Scene, geo: &RenderGeometry) -> Option<HazeParams> {
    let h = &scene.haze;
    if !haze_on(scene) {
        return None;
    }
    let mut b = scene.bounds();
    if b.is_empty() {
        b.grow_point(Vec3::ZERO);
    }
    b.grow_point(scene.render_camera().position);
    for l in &scene.lights {
        if l.enabled && !matches!(l.kind, forge_scene::LightKind::Directional) {
            b.grow_point(l.position);
        }
    }
    // Pavimenti enormi: la foschia resta entro 20 m dal centro della scena.
    let center = (b.min + b.max) * 0.5;
    let half = ((b.max - b.min) * 0.5 * 1.05 + Vec3::splat(0.05)).min(Vec3::splat(20.0));
    let (mut min, mut max) = (center - half, center + half);
    let floor = haze_floor(geo, min, max, min.y);
    // Sotto il pavimento (o sotto l'altezza di partenza, salvo gli sbuffi che pendono) la foschia
    // non serve: celle più fitte dove c'è.
    let bottom = h.bottom_height.max(0.0);
    let below = if bottom > 0.0 {
        h.swirl_scale.max(0.005) * 2.0 * h.puffs.clamp(0.0, 1.0) + 0.05
    } else {
        0.02
    };
    min.y = min.y.max(floor + bottom - below);
    // Fumo a terra: il riquadro si ferma poco sopra lo strato (celle fitte dove serve).
    if h.kind == forge_scene::HazeKind::ToxicFloor {
        max.y = max.y.min(floor + bottom + 0.05 + h.falloff_height.max(0.01) * 5.0);
    }
    // Altezza massima: sopra ci sono solo gli sbuffi, che si spengono in poco.
    if h.top_height > 0.0 {
        max.y = max.y.min(floor + 0.05 + h.top_height * 2.0 + h.swirl_scale.max(0.005) * 2.0);
    }
    max.y = max.y.max(min.y + 0.02);
    let key = format!("{h:?}|{min:?}|{max:?}|{}", geo.id);
    Some(HazeParams { min, max, floor, key })
}

/// Foschia d'ambiente pronta (dalla cache, o calcolata qui). L'avanzamento va sulla barra
/// della nebbia, se c'è un calcolo in corso.
pub fn haze_fog(scene: &Scene, geo: &RenderGeometry) -> Option<Arc<BakedFog>> {
    use forge_volume::progress;
    let HazeParams { min, max, floor, key } = haze_params(scene, geo)?;
    if let Some(f) = haze_cache().lock().unwrap().get(&key) {
        return Some(f.clone());
    }
    let h = &scene.haze;
    let ext = max - min;
    // Qualità 1–4: celle per vortice, celle minime e massime per lato, tetto di celle (con 4:
    // 24 milioni di celle, 48 MB, entro la memoria che la GPU dà a un buffer).
    let (per_swirl, min_side, max_side, max_cells) = HAZE_QUALITY[(h.quality.clamp(1, 4) - 1) as usize];
    let mut cell = h.swirl_scale.max(0.005) / per_swirl;
    if h.falloff_height > 0.0 {
        cell = cell.min(h.falloff_height / (per_swirl * 0.66));
    }
    // La griglia segue anche la grandezza della scena: mai meno di `min_side` celle per lato.
    cell = cell.min(ext.max_element() / min_side).max(ext.max_element() / max_side);
    while (ext.x / cell).ceil() * (ext.y / cell).ceil() * (ext.z / cell).ceil() > max_cells {
        cell *= 1.1;
    }
    let longest = ((ext.max_element() / cell).ceil() as usize).clamp(4, max_side as usize);
    // Niente foschia dentro gli oggetti: solo le celle interne, non il guscio (che è in parte
    // aria: azzerarlo lascerebbe un alone pulito attorno ai pezzi piccoli). I tratti dentro
    // vetri e liquidi li esclude esattamente il path tracer.
    let t0 = std::time::Instant::now();
    let spec = GridSpec::covering(min, max, longest);
    progress::update(0.0);
    let shell = voxelize(geo, &spec);
    progress::update(0.2);
    let inside = interior_cells(geo, &spec, &shell);
    progress::update(0.3);
    let n_inside = inside.iter().filter(|&&b| b).count();
    let mask_ms = t0.elapsed().as_secs_f64() * 1e3;
    let hz = h.clone();
    let albedo = Vec3::from(h.color).clamp(Vec3::ZERO, Vec3::splat(0.999));
    let done = std::sync::atomic::AtomicUsize::new(0);
    let n = spec.len().max(1);
    let fog = BakedFog::from_fn(min, max, longest, albedo, h.anisotropy.clamp(-0.95, 0.95), move |p| {
        let i = done.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if i.is_multiple_of(32_768) {
            progress::update(0.3 + 0.7 * i as f32 / n as f32);
        }
        match spec.cell_of(p) {
            Some((x, y, z)) if inside[spec.index(x, y, z)] => 0.0,
            _ => haze_density_at(&hz, p, floor),
        }
    });
    // Annullato durante il calcolo: niente cache (si rifà all'avvio del render).
    if progress::cancelled() {
        return None;
    }
    let mut fog = fog;
    // Solo raggi delle luci: luce delle lampade aggiunta, senza velo (vedi BakedFog::additive).
    fog.additive = h.rays_only;
    fog.camera_only = h.rays_only;
    let f = Arc::new(fog);
    tracing::info!(
        "foschia d'ambiente: {:?} celle in {:.0} ms (+ {:.0} ms per trovare {} celle dentro gli oggetti)",
        f.spec.res,
        f.bake_ms,
        mask_ms,
        n_inside
    );
    let mut c = haze_cache().lock().unwrap();
    if c.len() > 8 {
        c.clear();
    }
    c.insert(key, f.clone());
    Some(f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::Mat4;
    use forge_scene::{primitives, ComponentId};

    /// La foschia vorticosa ha banchi e vuoti, ma in media resta vicina alla densità scelta.
    #[test]
    fn haze_swirls_keep_the_mean_density() {
        let h = forge_scene::Haze {
            enabled: true,
            density: 0.2,
            variation: 0.8,
            ..Default::default()
        };
        let (mut sum, mut lo, mut hi, n) = (0.0, f32::MAX, 0.0f32, 4000);
        for i in 0..n {
            let p = Vec3::new((i % 20) as f32 * 0.13, ((i / 20) % 20) as f32 * 0.11, (i / 400) as f32 * 0.17);
            let d = haze_density_at(&h, p, 0.0);
            sum += d;
            lo = lo.min(d);
            hi = hi.max(d);
        }
        let mean = sum / n as f32;
        assert!((mean / 0.2 - 1.0).abs() < 0.35, "media {mean}");
        assert!(lo < 0.1 && hi > 0.3, "niente vortici: {lo}..{hi}");
        let flat = forge_scene::Haze { variation: 0.0, ..h };
        assert_eq!(haze_density_at(&flat, Vec3::ONE, 0.0), 0.2);
    }

    /// Fascia sospesa: con altezza di partenza 2,2 m e massima 2,8 m (senza sbuffi) la foschia c'è
    /// solo in mezzo; anche il fumo a terra si alza con l'altezza di partenza.
    #[test]
    fn haze_band_between_start_and_top_height() {
        let h = forge_scene::Haze {
            enabled: true,
            density: 0.2,
            variation: 0.0,
            bottom_height: 2.2,
            top_height: 2.8,
            puffs: 0.0,
            ..Default::default()
        };
        let at = |y: f32| haze_density_at(&h, Vec3::new(0.3, y, -0.2), 0.0);
        assert_eq!(at(1.0), 0.0, "sotto la fascia");
        assert_eq!(at(2.1), 0.0, "appena sotto");
        assert!(at(2.5) > 0.1, "dentro la fascia: {}", at(2.5));
        assert_eq!(at(3.0), 0.0, "sopra la fascia");
        let toxic = forge_scene::Haze {
            bottom_height: 1.0,
            top_height: 0.0,
            puffs: 0.0,
            ..forge_scene::Haze::of_kind(forge_scene::HazeKind::ToxicFloor)
        };
        let t = |y: f32| haze_density_at(&toxic, Vec3::new(0.0, y, 0.0), 0.0);
        assert_eq!(t(0.03), 0.0, "il fumo a terra non è più a terra");
        let lifted: f32 = (0..20)
            .map(|i| haze_density_at(&toxic, Vec3::new(i as f32 * 0.07, 1.05, 0.1), 0.0))
            .sum();
        assert!(lifted > 0.0, "strato spostato a 1 m");
    }

    /// Bancone a y = 0 sopra un pavimento a y = −1: la nebbia sopra il bancone si appoggia
    /// al bancone, non al pavimento (altrimenti la zona si allunga e le celle ingrossano).
    #[test]
    fn fog_rests_on_the_surface_below_not_on_the_lowest_point() {
        let mut s = Scene::default();
        for (name, size, y) in [("bancone", 2.0, 0.0), ("pavimento", 10.0, -1.0)] {
            let m = s.add_mesh(primitives::plane(size, size));
            s.add_node(None, ComponentId(name.into()), name, Mat4::from_translation(Vec3::Y * y), Some(m));
        }
        let geo = RenderGeometry::build(&s);
        let high = FogVolume {
            emitter: Vec3::new(0.0, 0.5, 0.0),
            size: Vec3::splat(0.4),
            ..Default::default()
        };
        let g = ground_under(&high, &geo).expect("superficie sotto la zona");
        assert!(g.abs() < 1e-4, "{g}");
        // La zona arriva già sotto il bancone: non si estende.
        let low = FogVolume {
            emitter: Vec3::new(0.0, 0.1, 0.0),
            ..high
        };
        assert_eq!(ground_under(&low, &geo), None);
    }

    /// Spostare un oggetto fuori dalla zona della nebbia non cambia la chiave (niente nuova
    /// simulazione); spostarne uno dentro sì.
    #[test]
    fn haze_stays_out_of_solid_objects() {
        let mut s = Scene::default();
        let m = s.add_mesh(primitives::cuboid(Vec3::splat(0.5)));
        s.add_node(None, ComponentId("cubo".into()), "cubo", Mat4::IDENTITY, Some(m));
        s.haze = forge_scene::Haze {
            enabled: true,
            variation: 0.0,
            ..Default::default()
        };
        let geo = RenderGeometry::build(&s);
        let f = haze_fog(&s, &geo).expect("foschia");
        assert_eq!(f.density(Vec3::ZERO), 0.0, "dentro il cubo");
        let (lo, _) = f.bounds();
        let outside = Vec3::new(lo.x + f.spec.cell, 0.0, 0.0);
        assert!(f.density(outside) > 0.0, "fuori dal cubo");
    }

    #[test]
    fn only_obstacles_inside_the_zone_invalidate_the_fog() {
        let build = |far_x: f32, near_x: f32| {
            let mut s = Scene::default();
            let floor = s.add_mesh(primitives::plane(4.0, 4.0));
            s.add_node(None, ComponentId("tavolo".into()), "tavolo", Mat4::IDENTITY, Some(floor));
            for (name, x) in [("lontano", far_x), ("vicino", near_x)] {
                let m = s.add_mesh(primitives::cuboid(Vec3::splat(0.1)));
                s.add_node(
                    None,
                    ComponentId(name.into()),
                    name,
                    Mat4::from_translation(Vec3::new(x, 0.05, 0.0)),
                    Some(m),
                );
            }
            RenderGeometry::build(&s)
        };
        let fog = FogVolume {
            emitter: Vec3::new(0.0, 0.3, 0.0),
            size: Vec3::new(0.6, 0.5, 0.6),
            resolution: 48,
            ..Default::default()
        };
        let a = build(1.5, 0.15);
        let moved_far = build(1.7, 0.15);
        let moved_near = build(1.5, 0.05);
        assert_ne!(a.id, moved_far.id);
        assert_eq!(key(&fog, &a), key(&fog, &moved_far), "oggetto fuori dalla zona");
        assert_ne!(key(&fog, &a), key(&fog, &moved_near), "oggetto dentro la zona");
        // Senza collisioni la geometria non conta mai.
        let free = FogVolume { collisions: false, ..fog };
        assert_eq!(key(&free, &a), key(&free, &moved_near));
    }
}
