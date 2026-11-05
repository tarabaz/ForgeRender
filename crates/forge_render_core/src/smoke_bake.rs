//! Precalcolo del fumo simulato sulla timeline: la simulazione gira una volta su tutta la durata e
//! lascia nella cartella temporanea un file per fotogramma e, una volta al secondo, lo stato completo
//! da cui può ripartire. Spostare il cursore legge il file del fotogramma, non ricalcola niente.
//!
//! Ogni fotogramma ha un'impronta cumulativa di tutto quello che lo influenza fino a lì: oggetti
//! fermi, oggetti in movimento quando sono nel riquadro del fumo, emissione accesa o spenta. Se
//! cambia qualcosa si cerca il primo fotogramma con l'impronta diversa e si riparte dall'ultimo
//! stato salvato prima di lui: quello che viene prima resta identico al bit. Una sfera che entra
//! nel riquadro al secondo 6 non tocca i primi 6 secondi, e un fumo che piace non cambia forma.
//!
//! Gli oggetti animati sono voxelizzati una volta nel loro spazio e riportati nella griglia a ogni
//! passo con la loro matrice, da cui viene anche la velocità delle celle (differenza finita della
//! matrice): la simulazione li usa come bordi in movimento (vedi `forge_volume::smoke_sim`).
//!
//! Cartella: `%TEMP%/ForgeRender/fumo/<impronta dei parametri>/`, con `manifest.txt`, `f_<k>.bin`
//! (fumo sparso a 8 bit) e `c_<k>.bin` (stato). Restano le ultime cartelle usate.

use crate::fog::{interior_cells, voxelize};
use crate::RenderGeometry;
use forge_math::{Aabb, Mat4, Vec3};
use forge_scene::{Prop, Scene, SmokeCloud, Target, Timeline};
use forge_volume::smoke_sim::{simulate_bake, BakeHooks, BakeRun, SimState, SmokeField, SolidFrame};
use forge_volume::{progress, GridSpec};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Cartelle di precalcolo tenute.
const KEEP_DIRS: usize = 6;
/// Limite della densità nei file: oltre il render la taglia comunque.
const RANGE: f32 = 64.0;

/// Fotogrammi simulati in questo processo (per le prove).
pub static SIMULATED_FRAMES: AtomicUsize = AtomicUsize::new(0);

fn root() -> PathBuf {
    match std::env::var_os("FORGE_SMOKE_CACHE") {
        Some(p) => PathBuf::from(p),
        None => std::env::temp_dir().join("ForgeRender").join("fumo"),
    }
}

/// Nodi con una mesh che si muovono con la timeline (con i loro discendenti).
fn moving_nodes(scene: &Scene) -> Vec<usize> {
    let mut out = Vec::new();
    for tr in &scene.timeline.tracks {
        let Target::Object(id) = &tr.target else { continue };
        if !tr.is_animated() || !tr.prop.moves_geometry() || tr.prop == Prop::Visible {
            continue;
        }
        let Some(i) = scene.find(id) else { continue };
        let mut stack = vec![i];
        while let Some(n) = stack.pop() {
            if scene.nodes[n].mesh.is_some() && !out.contains(&n) {
                out.push(n);
            }
            stack.extend(&scene.nodes[n].children);
        }
    }
    out
}

/// Parametri che decidono tutto il fumo: cambiandoli si riparte da zero. Tempo e fine
/// dell'emissione non ci sono: l'emissione sta nell'impronta dei fotogrammi.
fn params_key(s: &SmokeCloud, fps: f32, start: f32) -> u64 {
    let mut sim = s.sim.clone();
    sim.time = 0.0;
    sim.emit_stop = 0.0;
    let (lo, hi) = s.bounds();
    let mut h = DefaultHasher::new();
    format!("{sim:?}|{lo:?}|{hi:?}|{}|{}|{}|{fps}|{start}", s.rotation_deg, s.seed, s.collisions).hash(&mut h);
    h.finish()
}

/// Impronta di un oggetto in una posizione. Gli oggetti di un fotogramma si combinano con lo XOR,
/// senza ordine: conta solo chi c'è e dove, non se è animato (un oggetto fermo che diventa animato
/// ma resta lì non cambia niente).
fn node_digest(id: &str, m: &Mat4) -> u64 {
    let mut h = DefaultHasher::new();
    id.hash(&mut h);
    for v in m.to_cols_array() {
        v.to_bits().hash(&mut h);
    }
    h.finish()
}

fn touches(b: &Aabb, box_: &Aabb) -> bool {
    b.min.cmple(box_.max).all() && b.max.cmpge(box_.min).all()
}

/// Timeline pronta per essere applicata a una copia della scena.
fn fresh_timeline(scene: &Scene) -> Timeline {
    let mut tl = scene.timeline.clone();
    for t in &mut tl.tracks {
        t.last = None;
        t.wanted = None;
    }
    tl
}

/// Impronta cumulativa di ogni fotogramma (1..=n, in posizione k − 1).
fn frame_hashes(scene: &Scene, s: &SmokeCloud, movers: &[usize], dt: f32, start: f32, n: usize) -> Vec<u64> {
    let (lo, hi) = s.bounds();
    let box_ = Aabb { min: lo, max: hi };
    // Solo gli oggetti dentro il riquadro: quelli fuori il fumo non li sente.
    let mut fixed = 0u64;
    if s.collisions {
        for (i, node) in scene.nodes.iter().enumerate() {
            let Some(m) = node.mesh else { continue };
            if movers.contains(&i) || !scene.is_visible(i) || scene.is_effect_node(i) {
                continue;
            }
            let xf = scene.world_transform(i);
            if touches(&scene.meshes[m].bounds().transformed(&xf), &box_) {
                fixed ^= node_digest(&node.id.0, &xf);
            }
        }
    }
    let bounds: Vec<Aabb> = movers
        .iter()
        .map(|&i| scene.meshes[scene.nodes[i].mesh.unwrap()].bounds())
        .collect();
    let mut sc = scene.clone();
    let mut tl = fresh_timeline(scene);
    let mut h = DefaultHasher::new();
    let mut out = Vec::with_capacity(n);
    for k in 0..n {
        let time = k as f32 * dt;
        (s.sim.emit_stop <= 0.0 || time < s.sim.emit_stop).hash(&mut h);
        let mut digest = fixed;
        if !movers.is_empty() {
            tl.time = start + time;
            tl.apply(&mut sc);
            for (m, &i) in movers.iter().enumerate() {
                let xf = sc.world_transform(i);
                if touches(&bounds[m].transformed(&xf), &box_) {
                    digest ^= node_digest(&sc.nodes[i].id.0, &xf);
                }
            }
        }
        digest.hash(&mut h);
        out.push(h.finish());
    }
    out
}

fn read_manifest(dir: &Path) -> Vec<u64> {
    std::fs::read_to_string(dir.join("manifest.txt"))
        .map(|t| t.lines().filter_map(|l| u64::from_str_radix(l.trim(), 16).ok()).collect())
        .unwrap_or_default()
}

fn write_manifest(dir: &Path, hashes: &[u64]) {
    let text: String = hashes.iter().map(|h| format!("{h:016x}\n")).collect();
    let tmp = dir.join("manifest.tmp");
    if std::fs::write(&tmp, text).is_ok() {
        let _ = std::fs::rename(&tmp, dir.join("manifest.txt"));
    }
}

fn frame_path(dir: &Path, k: usize) -> PathBuf {
    dir.join(format!("f_{k:05}.bin"))
}

fn state_path(dir: &Path, k: usize) -> PathBuf {
    dir.join(format!("c_{k:05}.bin"))
}

/// Fumo di un fotogramma: celle con fumo, densità a 8 bit (radice: più precisione dove è rado).
fn write_frame(path: &Path, f: &SmokeField) -> std::io::Result<()> {
    let scale = (f.max.max(1e-6)) * RANGE;
    let mut cells: Vec<u32> = Vec::new();
    let mut vals: Vec<u8> = Vec::new();
    for (i, &d) in f.density.iter().enumerate() {
        let q = ((d / scale).clamp(0.0, 1.0).sqrt() * 255.0).round() as u8;
        if q > 0 {
            cells.push(i as u32);
            vals.push(q);
        }
    }
    let mut w = std::io::BufWriter::new(std::fs::File::create(path)?);
    let head: [u32; 3] = [f.spec.res[0] as u32, f.spec.res[1] as u32, f.spec.res[2] as u32];
    w.write_all(bytemuck::cast_slice(&head))?;
    let floats = [f.spec.cell, f.spec.min.x, f.spec.min.y, f.spec.min.z, f.max];
    w.write_all(bytemuck::cast_slice(&floats))?;
    w.write_all(bytemuck::cast_slice(&[cells.len() as u32]))?;
    w.write_all(bytemuck::cast_slice(&cells))?;
    w.write_all(&vals)?;
    w.flush()
}

fn read_frame(path: &Path) -> Option<SmokeField> {
    let b = std::fs::read(path).ok()?;
    let u32_at = |o: usize| b.get(o..o + 4).map(|s| u32::from_le_bytes(s.try_into().unwrap()));
    let f32_at = |o: usize| b.get(o..o + 4).map(|s| f32::from_le_bytes(s.try_into().unwrap()));
    let res = [u32_at(0)? as usize, u32_at(4)? as usize, u32_at(8)? as usize];
    let (cell, min, max) = (f32_at(12)?, Vec3::new(f32_at(16)?, f32_at(20)?, f32_at(24)?), f32_at(28)?);
    let count = u32_at(32)? as usize;
    let spec = GridSpec { min, res, cell };
    let idx = b.get(36..36 + count * 4)?;
    let vals = b.get(36 + count * 4..36 + count * 5)?;
    let scale = max.max(1e-6) * RANGE;
    let mut density = vec![0.0f32; spec.len()];
    for (c, &q) in idx.as_chunks::<4>().0.iter().zip(vals) {
        let i = u32::from_le_bytes(*c) as usize;
        let v = q as f32 / 255.0;
        *density.get_mut(i)? = v * v * scale;
    }
    Some(SmokeField {
        spec,
        density,
        max,
        steps: 0,
        ms: 0.0,
    })
}

fn write_state(path: &Path, st: &SimState) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    {
        let mut w = std::io::BufWriter::new(std::fs::File::create(&tmp)?);
        let heads = [st.rho.len() as u32, st.parts.len() as u32];
        w.write_all(bytemuck::cast_slice(&heads))?;
        w.write_all(bytemuck::cast_slice(&[st.part_debt]))?;
        for a in [&st.u, &st.v, &st.w, &st.rho, &st.temp, &st.pres] {
            w.write_all(bytemuck::cast_slice(a))?;
        }
        w.write_all(bytemuck::cast_slice(&st.parts))?;
        w.flush()?;
    }
    std::fs::rename(tmp, path)
}

fn read_state(path: &Path) -> Option<SimState> {
    let b = std::fs::read(path).ok()?;
    let word = |o: usize| b.get(o..o + 4).map(|s| u32::from_le_bytes(s.try_into().unwrap()));
    let (n, parts) = (word(0)? as usize, word(4)? as usize);
    let part_debt = f32::from_bits(word(8)?);
    let mut o = 12;
    let mut take = |len: usize| -> Option<Vec<f32>> {
        let s = b.get(o..o + len * 4)?;
        o += len * 4;
        Some(s.as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes(*c)).collect())
    };
    let (u, v, w, rho, temp, pres) = (take(n)?, take(n)?, take(n)?, take(n)?, take(n)?, take(n)?);
    let flat = take(parts * 4)?;
    Some(SimState {
        u,
        v,
        w,
        rho,
        temp,
        pres,
        parts: flat.as_chunks::<4>().0.to_vec(),
        part_debt,
    })
}

/// Un oggetto animato voxelizzato una volta, nel punto in cui è nella scena.
struct Mover {
    node: usize,
    /// Da mondo (all'istante della voxelizzazione)...
    start: Mat4,
    spec: GridSpec,
    mask: Vec<bool>,
    local: Aabb,
}

fn solid_cells(geo: &RenderGeometry, spec: &GridSpec) -> Vec<bool> {
    let shell = voxelize(geo, spec);
    let inside = interior_cells(geo, spec, &shell);
    shell.iter().zip(&inside).map(|(a, b)| *a || *b).collect()
}

fn build_mover(scene: &Scene, node: usize, cell: f32) -> Option<Mover> {
    let mut only = scene.clone();
    for (i, n) in only.nodes.iter_mut().enumerate() {
        if i != node && n.mesh.is_some() {
            n.visible = false;
        }
    }
    let local = scene.meshes[scene.nodes[node].mesh?].bounds();
    let xf = scene.world_transform(node);
    let world = local.transformed(&xf);
    let margin = Vec3::splat(cell * 2.0);
    let ext = world.max - world.min + margin * 2.0;
    let longest = ((ext.max_element() / (cell * 0.75)).ceil() as usize).clamp(8, 256);
    let spec = GridSpec::covering(world.min - margin, world.max + margin, longest);
    let geo = RenderGeometry::build(&only);
    let mask = solid_cells(&geo, &spec);
    Some(Mover {
        node,
        start: xf,
        spec,
        mask,
        local,
    })
}

/// Segue la simulazione: oggetti di ogni istante, fotogrammi e stati su file.
struct Writer {
    dir: PathBuf,
    spec: GridSpec,
    base: Vec<bool>,
    movers: Vec<Mover>,
    scene: Scene,
    timeline: Timeline,
    start: f32,
    air: bool,
    was_moving: bool,
    last: Vec<Mat4>,
    hashes: Vec<u64>,
    every: usize,
    errors: usize,
}

impl Writer {
    fn at(&mut self, time: f32) -> Vec<Mat4> {
        self.timeline.time = self.start + time;
        self.timeline.apply(&mut self.scene);
        self.movers.iter().map(|m| self.scene.world_transform(m.node)).collect()
    }
}

/// Tempo per la velocità delle celle (differenza finita, sempre uguale: riprendendo da uno stato
/// il primo passo vede la stessa velocità del calcolo fatto di fila).
const VELOCITY_DT: f32 = 1.0 / 240.0;

impl BakeHooks for Writer {
    fn solid(&mut self, time: f32) -> Option<SolidFrame> {
        if self.movers.is_empty() {
            return None;
        }
        let before = self.at((time - VELOCITY_DT).max(0.0));
        let now = self.at(time);
        if now == self.last && !self.was_moving {
            return None;
        }
        let n = self.spec.len();
        let s = self.spec;
        let moving = now != before;
        let mut mask = self.base.clone();
        let mut velocity = if self.air && moving { vec![[0.0f32; 3]; n] } else { Vec::new() };
        let (mut zl, mut zh) = ([usize::MAX; 3], [0usize; 3]);
        for (m, mover) in self.movers.iter().enumerate() {
            let b = mover.local.transformed(&now[m]);
            let cell_of = |v: f32, o: f32, r: usize| (((v - o) / s.cell).floor().max(0.0) as usize).min(r - 1);
            if b.max.cmplt(s.min).any() || b.min.cmpgt(s.max()).any() {
                continue;
            }
            let lo = [
                cell_of(b.min.x, s.min.x, s.res[0]),
                cell_of(b.min.y, s.min.y, s.res[1]),
                cell_of(b.min.z, s.min.z, s.res[2]),
            ];
            let hi = [
                cell_of(b.max.x, s.min.x, s.res[0]),
                cell_of(b.max.y, s.min.y, s.res[1]),
                cell_of(b.max.z, s.min.z, s.res[2]),
            ];
            for a in 0..3 {
                zl[a] = zl[a].min(lo[a]);
                zh[a] = zh[a].max(hi[a]);
            }
            let to_start = mover.start * now[m].inverse();
            let to_before = before[m] * now[m].inverse();
            for z in lo[2]..=hi[2] {
                for y in lo[1]..=hi[1] {
                    for x in lo[0]..=hi[0] {
                        let c = s.center(x, y, z);
                        let p = to_start.transform_point3(c);
                        let Some((a, b2, d)) = mover.spec.cell_of(p) else { continue };
                        if !mover.mask[mover.spec.index(a, b2, d)] {
                            continue;
                        }
                        let i = s.index(x, y, z);
                        mask[i] = true;
                        if !velocity.is_empty() {
                            let v = (c - to_before.transform_point3(c)) / VELOCITY_DT;
                            velocity[i] = v.to_array();
                        }
                    }
                }
            }
        }
        self.was_moving = moving;
        self.last = now;
        Some(SolidFrame {
            mask,
            velocity,
            zone: (zl[0] != usize::MAX).then_some((zl, zh)),
        })
    }

    fn frame(&mut self, k: usize, field: &SmokeField) {
        SIMULATED_FRAMES.fetch_add(1, Ordering::Relaxed);
        if let Err(e) = write_frame(&frame_path(&self.dir, k), field) {
            self.errors += 1;
            tracing::warn!("fotogramma del fumo non salvato: {e}");
        }
    }

    fn wants_state(&self, k: usize) -> bool {
        k.is_multiple_of(self.every)
    }

    fn state(&mut self, k: usize, state: SimState) {
        if let Err(e) = write_state(&state_path(&self.dir, k), &state) {
            self.errors += 1;
            tracing::warn!("stato del fumo non salvato: {e}");
            return;
        }
        // Fin qui i fotogrammi sono buoni anche se il calcolo si interrompe.
        write_manifest(&self.dir, &self.hashes[..k.min(self.hashes.len())]);
    }
}

/// Toglie le cartelle di precalcolo più vecchie.
fn clean_old(current: &Path) {
    let Ok(rd) = std::fs::read_dir(root()) else { return };
    let mut dirs: Vec<(std::time::SystemTime, PathBuf)> = rd
        .flatten()
        .filter(|e| e.path().is_dir() && e.path() != current)
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .collect();
    dirs.sort_by_key(|d| std::cmp::Reverse(d.0));
    for (_, p) in dirs.into_iter().skip(KEEP_DIRS.saturating_sub(1)) {
        let _ = std::fs::remove_dir_all(p);
    }
}

/// Stato salvato più recente entro il fotogramma `limit`.
fn best_state(dir: &Path, limit: usize) -> usize {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.strip_prefix("c_")?.strip_suffix(".bin")?.parse::<usize>().ok()
        })
        .filter(|&k| k <= limit)
        .max()
        .unwrap_or(0)
}

/// Fumo all'istante della nube, dal precalcolo (fatto qui se manca, o ripreso dal primo fotogramma
/// cambiato). `p0..p1` è la parte della barra. None se il calcolo è stato annullato.
pub fn frame_field(scene: &Scene, s: &SmokeCloud, geo: &RenderGeometry, p0: f32, p1: f32) -> Option<Arc<SmokeField>> {
    let tl = &scene.timeline;
    let fps = tl.fps.clamp(1.0, 240.0);
    let dt = 1.0 / fps;
    let target = Target::Smoke(s.n);
    let start = tl.start_of(&target);
    let horizon = (tl.duration - start).max(s.sim.time).max(dt);
    let n = ((horizon / dt).ceil() as usize).max(1);
    let k = ((s.sim.time / dt).round() as usize).clamp(1, n);
    let movers = if s.collisions { moving_nodes(scene) } else { Vec::new() };
    let dir = root().join(format!("{:016x}", params_key(s, fps, start)));
    let hashes = frame_hashes(scene, s, &movers, dt, start, n);
    let done = read_manifest(&dir);
    let valid = done.iter().zip(&hashes).take_while(|(a, b)| a == b).count();
    if valid < k.max(n) && !bake(scene, s, geo, &dir, &movers, &hashes, valid, dt, start, p0, p1) {
        return None;
    }
    read_frame(&frame_path(&dir, k)).map(Arc::new)
}

#[allow(clippy::too_many_arguments)]
fn bake(
    scene: &Scene,
    s: &SmokeCloud,
    geo: &RenderGeometry,
    dir: &Path,
    movers: &[usize],
    hashes: &[u64],
    valid: usize,
    dt: f32,
    start: f32,
    p0: f32,
    p1: f32,
) -> bool {
    if std::fs::create_dir_all(dir).is_err() {
        tracing::warn!("cartella del precalcolo del fumo non disponibile: {}", dir.display());
        return false;
    }
    clean_old(dir);
    // Da dove si riparte: l'ultimo stato prima del primo fotogramma cambiato.
    let first = best_state(dir, valid);
    let state = (first > 0).then(|| read_state(&state_path(dir, first))).flatten();
    let first = if state.is_some() { first } else { 0 };
    tracing::info!(
        "fumo «{}»: precalcolo dei fotogrammi da {} a {} (i primi {} sono già buoni)",
        s.name,
        first + 1,
        hashes.len(),
        first
    );
    let spec = GridSpec::covering(s.bounds().0, s.bounds().1, s.sim_cells());
    let mut sc = scene.clone();
    let mut base: Option<Vec<bool>> = None;
    if s.collisions {
        let static_geo = if movers.is_empty() {
            None
        } else {
            for &m in movers {
                sc.nodes[m].visible = false;
            }
            Some(RenderGeometry::build(&sc))
        };
        let g = static_geo.as_ref().unwrap_or(geo);
        if !g.accel.is_empty() {
            base = Some(solid_cells(g, &spec));
        }
    }
    let cell = s.sim_cell();
    let built: Vec<Mover> = movers.iter().filter_map(|&m| build_mover(scene, m, cell)).collect();
    let mut w = Writer {
        dir: dir.to_path_buf(),
        spec,
        base: base.clone().unwrap_or_else(|| vec![false; spec.len()]),
        movers: built,
        scene: scene.clone(),
        timeline: fresh_timeline(scene),
        start,
        air: s.sim.air_push,
        was_moving: false,
        last: Vec::new(),
        hashes: hashes.to_vec(),
        every: ((1.0 / dt).round() as usize).max(1),
        errors: 0,
    };
    let params = crate::smoke::sim_params(s, spec);
    let last = hashes.len();
    let run = BakeRun {
        frame_dt: dt,
        first,
        last,
        state,
        hooks: &mut w,
    };
    let ok = simulate_bake(
        &params,
        base.as_deref(),
        run,
        &|x| progress::update(p0 + (p1 - p0) * x),
        &progress::cancelled,
    );
    if ok && w.errors == 0 {
        write_manifest(dir, hashes);
    }
    ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_scene::{Interp, Key, SmokeMode};

    fn scene() -> (Scene, SmokeCloud, Target) {
        let mut sc = Scene::test_scene();
        let s = SmokeCloud {
            mode: SmokeMode::Simulated,
            center: Vec3::new(0.0, 2.0, 0.0),
            size: Vec3::splat(1.0),
            sim: forge_scene::SmokeSim {
                quality: 1,
                time: 1.5,
                ..Default::default()
            },
            ..Default::default()
        };
        sc.smokes.push(s.clone());
        sc.timeline.fps = 10.0;
        sc.timeline.duration = 1.5;
        let id = sc.nodes.iter().find(|n| n.mesh.is_some()).unwrap().id.clone();
        (sc, s, Target::Object(id))
    }

    fn key(t: f32, v: f32) -> Key {
        Key {
            t,
            v,
            interp: Interp::Linear,
        }
    }

    /// La seconda volta non si simula niente; se un oggetto entra nel riquadro solo alla fine si
    /// rifà solo da lì, e i fotogrammi prima restano identici.
    #[test]
    fn only_what_changed_is_computed_again() {
        let dir = std::env::temp_dir().join(format!("forge_fumo_prova_{}", std::process::id()));
        std::env::set_var("FORGE_SMOKE_CACHE", &dir);
        let (mut sc, s, target) = scene();
        let geo = RenderGeometry::build(&sc);
        let done = || SIMULATED_FRAMES.load(Ordering::Relaxed);

        let a = done();
        let first = frame_field(&sc, &s, &geo, 0.0, 1.0).expect("fumo");
        assert_eq!(done() - a, 15, "tutti i fotogrammi");
        assert!(first.density.iter().any(|&d| d > 0.0), "c'è fumo");

        let a = done();
        frame_field(&sc, &s, &geo, 0.0, 1.0).expect("fumo");
        assert_eq!(done() - a, 0, "dal file, niente calcolo");

        // Un oggetto resta fuori dal riquadro fino a 1,2 s, poi ci entra.
        let mut tl = std::mem::take(&mut sc.timeline);
        tl.add_element(&sc, target.clone());
        let i = tl.track(&target, Prop::PosY).unwrap();
        tl.tracks[i].keys = vec![key(0.0, 0.0), key(1.2, 0.0), key(1.5, 3.0)];
        sc.timeline = tl;
        let early = |k: usize| std::fs::read(frame_path(&dir.join(format!("{:016x}", params_key(&s, 10.0, 0.0))), k)).unwrap();
        let before = early(5);
        let a = done();
        frame_field(&sc, &s, &geo, 0.0, 1.0).expect("fumo");
        let again = done() - a;
        assert!(again > 0 && again <= 15 - 10, "solo dall'ultimo stato prima dell'ingresso: {again}");
        assert_eq!(early(5), before, "il prima resta identico");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn frames_keep_the_smoke_in_a_file() {
        let spec = GridSpec::covering(Vec3::ZERO, Vec3::ONE, 8);
        let mut density = vec![0.0; spec.len()];
        density[17] = 2.0;
        density[200] = 0.5;
        let f = SmokeField {
            spec,
            density,
            max: 1.0,
            steps: 0,
            ms: 0.0,
        };
        let p = std::env::temp_dir().join(format!("forge_f_{}.bin", std::process::id()));
        write_frame(&p, &f).unwrap();
        let g = read_frame(&p).unwrap();
        let _ = std::fs::remove_file(&p);
        assert_eq!(g.spec, spec);
        assert!((g.density[17] - 2.0).abs() < 0.2 && (g.density[200] - 0.5).abs() < 0.2);
        assert_eq!(g.density[0], 0.0);
    }
}
