//! Viste multiple automatiche, turntable (con video MP4) e passaggi di render.
//!
//! Il prodotto gira su se stesso attorno all'asse verticale che passa per il suo centro, come su
//! un piatto girevole: camera, luci, ambiente e fondale restano fermi (girando la camera si
//! vedrebbe il retro del fondale). La vista dall'alto sposta invece la camera.
//!
//! Contro lo sfarfallio tra fotogrammi vicini: stessi campioni per pixel in ogni fotogramma
//! (stesso seed: il rumore residuo resta fermo sullo schermo invece di ribollire), niente
//! campionamento adattivo (si fermerebbe in punti diversi a ogni fotogramma) e denoiser solo
//! sull'immagine finale, a numero di campioni fisso.
//!
//! Alla fine (anche se si interrompe) scena, camera e impostazioni tornano come prima.

use crate::app::{ForgeApp, View};
use crate::i18n::{tf, tk, tr};
use forge_math::{Mat4, Quat, Vec3};
use forge_render_core::{DenoiseMode, RenderScene, RenderSettings, SessionState, ToneMapping};
use forge_render_cpu::passes::{group_color, render_passes, NO_GROUP};
use forge_scene::{Camera, UserTransform};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeqKind {
    Views,
    Turntable,
}

/// Viste predefinite: (parte del nome del file, etichetta, angolo da cui si guarda in gradi —
/// 0 = l'inquadratura attuale, 90 = da destra —, vista dall'alto).
pub const VIEWS: [(&str, &str, f32, bool); 7] = [
    ("frontale", tk("Frontale"), 0.0, false),
    ("tre-quarti-destra", tk("Tre quarti destra"), 45.0, false),
    ("destra", tk("Destra"), 90.0, false),
    ("posteriore", tk("Posteriore"), 180.0, false),
    ("sinistra", tk("Sinistra"), 270.0, false),
    ("tre-quarti-sinistra", tk("Tre quarti sinistra"), 315.0, false),
    ("alto", tk("Dall'alto"), 0.0, true),
];

/// Passaggi di render da salvare accanto all'immagine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct PassSet {
    pub albedo: bool,
    pub normals: bool,
    pub depth: bool,
    pub mask: bool,
    pub ids: bool,
}

impl PassSet {
    pub fn any(&self) -> bool {
        self.albedo || self.normals || self.depth || self.mask || self.ids
    }
}

#[derive(Clone, Debug)]
pub struct SequenceOptions {
    pub kind: SeqKind,
    pub views: [bool; 7],
    /// Gradi di rotazione tra un fotogramma e il successivo.
    pub step_deg: f32,
    pub fps: u32,
    /// Giri ripetuti nel video (lo stesso giro, senza rifare i render).
    pub turns: u32,
    /// Senso orario visto dall'alto (altrimenti antiorario).
    pub clockwise: bool,
    /// Inquadratura adattata perché il prodotto stia sempre tutto dentro.
    pub fit_frame: bool,
    pub samples: u32,
    pub make_video: bool,
    pub passes: PassSet,
    pub out_dir: Option<PathBuf>,
    pub name: String,
}

impl Default for SequenceOptions {
    fn default() -> Self {
        Self {
            kind: SeqKind::Turntable,
            views: [true; 7],
            step_deg: 2.0,
            fps: 30,
            turns: 1,
            clockwise: false,
            fit_frame: true,
            samples: 256,
            make_video: true,
            passes: PassSet::default(),
            out_dir: None,
            name: "prodotto".into(),
        }
    }
}

/// Fotogrammi di un giro completo con questo passo (il giro si chiude: l'ultimo non ripete il
/// primo, così il video gira in loop senza scatti).
pub fn turntable_frames(step_deg: f32) -> u32 {
    (360.0 / step_deg.max(0.1)).round().clamp(2.0, 3600.0) as u32
}

/// Passo effettivo: 360° divisi esattamente nei fotogrammi del giro.
pub fn effective_step(step_deg: f32) -> f32 {
    360.0 / turntable_frames(step_deg) as f32
}

/// Passo che dà un giro di `seconds` secondi a `fps` fotogrammi al secondo.
pub fn step_for_duration(seconds: f32, fps: u32) -> f32 {
    let frames = (seconds.max(0.1) * fps.max(1) as f32).round().clamp(2.0, 3600.0);
    360.0 / frames
}

/// Durata in secondi di un giro.
pub fn turn_seconds(step_deg: f32, fps: u32) -> f32 {
    turntable_frames(step_deg) as f32 / fps.max(1) as f32
}

/// Un fotogramma da produrre.
#[derive(Clone, Debug, PartialEq)]
pub struct FramePlan {
    /// Nome del file senza estensione.
    pub file: String,
    pub label: String,
    /// Rotazione del prodotto attorno all'asse verticale, in gradi (positiva = antioraria
    /// vista dall'alto).
    pub spin_deg: f32,
    pub top: bool,
}

pub fn plan(o: &SequenceOptions) -> Vec<FramePlan> {
    let name = safe_name(&o.name);
    match o.kind {
        SeqKind::Views => VIEWS
            .iter()
            .zip(o.views)
            .filter(|(_, on)| *on)
            .map(|((file, label, angle, top), _)| FramePlan {
                file: format!("{name}_{file}"),
                label: tr(label).to_string(),
                // Guardare da destra = ruotare il prodotto di -90° (il suo lato destro verso la camera).
                spin_deg: -angle,
                top: *top,
            })
            .collect(),
        SeqKind::Turntable => {
            let n = turntable_frames(o.step_deg);
            let step = 360.0 / n as f32;
            let sign = if o.clockwise { -1.0 } else { 1.0 };
            (0..n)
                .map(|k| FramePlan {
                    file: format!("{name}_{:04}", k + 1),
                    label: tf("Fotogramma {} di {}", &[format!("{}", k + 1), format!("{n}")]),
                    spin_deg: sign * step * k as f32,
                    top: false,
                })
                .collect()
        }
    }
}

/// Nome utilizzabile come file (lettere, cifre, - e _).
pub fn safe_name(s: &str) -> String {
    let s: String = s
        .trim()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    if s.is_empty() {
        "render".into()
    } else {
        s
    }
}

/// Rotazione di `deg` gradi attorno all'asse verticale che passa per `center`.
pub fn spin_matrix(center: Vec3, deg: f32) -> Mat4 {
    Mat4::from_translation(center) * Mat4::from_quat(Quat::from_rotation_y(deg.to_radians())) * Mat4::from_translation(-center)
}

/// La camera vista dall'alto: sopra `center` alla stessa distanza, con il davanti del prodotto
/// (il lato verso la camera attuale) in basso nell'immagine.
pub fn top_camera(cam: &Camera, center: Vec3) -> Camera {
    let f = cam.forward();
    let mut up = Vec3::new(f.x, 0.0, f.z);
    up = if up.length_squared() > 1e-8 { up.normalize() } else { -Vec3::Z };
    let dist = (cam.position - cam.target).length().max(1e-3);
    Camera {
        position: center + Vec3::Y * dist,
        target: center,
        up,
        ..cam.clone()
    }
}

/// La stessa camera (direzione e obiettivo) avvicinata o allontanata perché tutti i punti stiano
/// dentro l'inquadratura con un margine (`fill` = frazione del lato più stretto, es. 0,85),
/// puntata al centro dei punti. Ortografica: cambia l'altezza dell'inquadratura.
pub fn fit_camera(cam: &Camera, aspect: f32, points: &[Vec3], fill: f32) -> Camera {
    if points.is_empty() {
        return cam.clone();
    }
    let (mut lo, mut hi) = (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY));
    for p in points {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    let target = (lo + hi) * 0.5;
    let mut c = cam.clone();
    let old_dist = (cam.position - cam.target).length().max(1e-4);
    let (r, u, f) = cam.basis();
    let (sx, sy) = (2.0 * cam.lens.shift_x, 2.0 * cam.lens.shift_y);
    let aspect = aspect.max(1e-3);
    if cam.lens.ortho {
        // Mezza altezza che contiene tutti i punti (proiezione parallela: non dipende dalla distanza).
        let mut half = 1e-4f32;
        for p in points {
            let q = *p - target;
            half = half.max(((q.dot(r) / aspect) / (fill - sx.abs()).max(0.05)).abs());
            half = half.max((q.dot(u) / (fill - sy.abs()).max(0.05)).abs());
        }
        c.target = target;
        c.position = target - f * old_dist;
        c.lens.ortho_height = 2.0 * half;
        return c;
    }
    let tan_v = (cam.vfov_deg.to_radians() * 0.5).tan().max(1e-4);
    let fits = |d: f32| {
        let pos = target - f * d;
        points.iter().all(|p| {
            let q = *p - pos;
            let z = q.dot(f);
            if z <= 1e-4 {
                return false;
            }
            let x = q.dot(r) / (z * tan_v * aspect) - sx;
            let y = q.dot(u) / (z * tan_v) - sy;
            x.abs() <= fill && y.abs() <= fill
        })
    };
    let radius = points.iter().map(|p| (*p - target).length()).fold(0.0f32, f32::max).max(1e-4);
    let mut hi_d = radius * 2.0;
    while !fits(hi_d) && hi_d < radius * 1e5 {
        hi_d *= 2.0;
    }
    let mut lo_d = radius * 1e-3;
    for _ in 0..40 {
        let mid = 0.5 * (lo_d + hi_d);
        if fits(mid) {
            hi_d = mid;
        } else {
            lo_d = mid;
        }
    }
    c.target = target;
    c.position = target - f * hi_d;
    // Il piano di fuoco si sposta con la camera (stessa parte del prodotto a fuoco).
    c.lens.focus_distance = (cam.lens.focus_distance + hi_d - old_dist).max(0.01);
    c
}

/// Punti ruotati di `deg` attorno all'asse verticale per `center`.
fn spun(points: &[Vec3], center: Vec3, deg: f32) -> Vec<Vec3> {
    let m = spin_matrix(center, deg);
    points.iter().map(|p| m.transform_point3(*p)).collect()
}

/// Camere dei fotogrammi: per le viste ognuna inquadrata sul suo lato, per il turntable una
/// sola che contiene il prodotto in tutte le posizioni del giro (l'inquadratura non salta).
pub fn plan_cameras(
    plan: &[FramePlan],
    cam: &Camera,
    aspect: f32,
    points: &[Vec3],
    center: Vec3,
    fit: bool,
    turntable: bool,
) -> Vec<Camera> {
    const FILL: f32 = 0.85;
    if turntable {
        let c = if fit {
            let all: Vec<Vec3> = plan.iter().flat_map(|f| spun(points, center, f.spin_deg)).collect();
            fit_camera(cam, aspect, &all, FILL)
        } else {
            cam.clone()
        };
        return vec![c; plan.len()];
    }
    plan.iter()
        .map(|f| {
            let base = if f.top { top_camera(cam, center) } else { cam.clone() };
            if fit {
                fit_camera(&base, aspect, &spun(points, center, f.spin_deg), FILL)
            } else {
                base
            }
        })
        .collect()
}

/// Gruppo (componente) di ogni nodo per i passaggi ID e maschera: il pezzo (il nodo sopra le
/// mesh "(mesh)" degli import), [`NO_GROUP`] per la scenografia.
pub fn node_groups(app: &ForgeApp) -> Vec<u32> {
    let s = &app.scene;
    (0..s.nodes.len())
        .map(|i| {
            if crate::presets::is_scenery_node(s, i) {
                return NO_GROUP;
            }
            match s.nodes[i].parent {
                Some(p) if s.nodes[i].name.ends_with("(mesh)") => p as u32,
                _ => i as u32,
            }
        })
        .collect()
}

/// Calcola e scrive i passaggi richiesti: `<base>_albedo.png`, `_normali.png`,
/// `_profondita.png` (16 bit, bianco = vicino) ed `_profondita.exr` (metri), `_maschera.png`,
/// `_id.png`. Gira su un thread a parte (CPU, pochi campioni: sono quasi senza rumore).
pub fn write_passes(scene: &RenderScene, settings: &RenderSettings, groups: &[u32], set: PassSet, base: &Path) -> anyhow::Result<()> {
    use anyhow::Context;
    let cancel = AtomicBool::new(false);
    let (w, h) = (settings.width, settings.height);
    let path = |suffix: &str| {
        let mut name = base.file_name().map(|n| n.to_os_string()).unwrap_or_default();
        name.push(suffix);
        base.with_file_name(name)
    };
    let save16 = |p: PathBuf, rgb: Vec<u16>| -> anyhow::Result<()> {
        let img = image::ImageBuffer::<image::Rgb<u16>, _>::from_raw(w, h, rgb).context("dimensioni")?;
        forge_image_io::write_atomic(&p, |tmp| Ok(img.save_with_format(tmp, image::ImageFormat::Png)?))
    };
    if set.albedo || set.normals {
        let g = forge_render_cpu::render_guides(scene, settings, forge_render_cpu::GUIDE_SAMPLES, &cancel).context("guide")?;
        if set.albedo {
            save16(
                path("_albedo.png"),
                forge_render_core::tonemap_to_rgb16(&g.albedo, ToneMapping::Standard, 0.0),
            )?;
        }
        if set.normals {
            // Normali nel mondo, x/y/z da [-1, 1] a [0, 1], valori lineari.
            let rgb = g
                .normal
                .iter()
                .flat_map(|n| n.map(|c| ((c * 0.5 + 0.5).clamp(0.0, 1.0) * 65535.0).round() as u16))
                .collect();
            save16(path("_normali.png"), rgb)?;
        }
    }
    if set.depth || set.mask || set.ids {
        let p = render_passes(scene, settings, groups, 8, &cancel).context("passaggi")?;
        if set.depth {
            let (mut near, mut far) = (f32::INFINITY, 0.0f32);
            for d in p.depth.iter().filter(|d| d.is_finite()) {
                near = near.min(*d);
                far = far.max(*d);
            }
            let span = (far - near).max(1e-6);
            let rgb = p
                .depth
                .iter()
                .flat_map(|d| {
                    let v = if d.is_finite() { (1.0 - (d - near) / span) * 65535.0 } else { 0.0 };
                    [v.round() as u16; 3]
                })
                .collect();
            save16(path("_profondita.png"), rgb)?;
            let data: Vec<f32> = p.depth.iter().flat_map(|d| [if d.is_finite() { *d } else { 0.0 }; 3]).collect();
            let img = image::ImageBuffer::<image::Rgb<f32>, _>::from_raw(w, h, data).context("dimensioni")?;
            forge_image_io::write_atomic(&path("_profondita.exr"), |tmp| {
                Ok(img.save_with_format(tmp, image::ImageFormat::OpenExr)?)
            })?;
        }
        if set.mask {
            let l: Vec<u8> = p.mask.iter().map(|m| (m * 255.0).round() as u8).collect();
            let img = image::GrayImage::from_raw(w, h, l).context("dimensioni")?;
            forge_image_io::write_atomic(
                &path("_maschera.png"),
                |tmp| Ok(img.save_with_format(tmp, image::ImageFormat::Png)?),
            )?;
        }
        if set.ids {
            let rgb: Vec<u8> = p.group.iter().flat_map(|g| group_color(*g)).collect();
            let img = image::RgbImage::from_raw(w, h, rgb).context("dimensioni")?;
            forge_image_io::write_atomic(&path("_id.png"), |tmp| Ok(img.save_with_format(tmp, image::ImageFormat::Png)?))?;
        }
    }
    Ok(())
}

/// Stato da rimettere com'era alla fine della sequenza.
struct Saved {
    users: Vec<(usize, UserTransform)>,
    camera: Camera,
    active: Option<(usize, Camera)>,
    settings: RenderSettings,
    unsaved: bool,
    view: View,
    auto_restart: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// Da preparare il fotogramma `index`.
    Apply,
    /// Render del fotogramma `index` in corso.
    Rendering,
    /// Fotogrammi finiti: si aspettano i passaggi e il video.
    Finishing,
}

pub struct Run {
    plan: Vec<FramePlan>,
    cameras: Vec<Camera>,
    index: usize,
    phase: Phase,
    frames_dir: PathBuf,
    video_path: Option<PathBuf>,
    fps: u32,
    turns: u32,
    passes: PassSet,
    groups: Vec<u32>,
    center: Vec3,
    saved: Saved,
    written: Vec<PathBuf>,
    workers: Vec<JoinHandle<anyhow::Result<()>>>,
    video: Option<JoinHandle<anyhow::Result<&'static str>>>,
    started: Instant,
    frame_started: Instant,
    frame_times: Vec<Duration>,
    /// Render completato ma ancora senza immagine ripulita: da quando.
    waiting_denoise: Option<Instant>,
    errors: Vec<String>,
    cancelled: bool,
}

impl Run {
    pub fn total(&self) -> usize {
        self.plan.len()
    }

    pub fn done(&self) -> usize {
        self.written.len()
    }

    pub fn current_label(&self) -> &str {
        self.plan.get(self.index).map(|f| f.label.as_str()).unwrap_or("")
    }

    pub fn is_finishing(&self) -> bool {
        self.phase == Phase::Finishing
    }

    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    /// Tempo che manca, dalla media dei fotogrammi già fatti.
    pub fn remaining(&self) -> Option<Duration> {
        if self.frame_times.is_empty() {
            return None;
        }
        let avg = self.frame_times.iter().sum::<Duration>() / self.frame_times.len() as u32;
        let left = self.plan.len().saturating_sub(self.written.len()) as u32;
        Some(avg * left)
    }
}

/// Stato della finestra "Viste e turntable".
pub struct SequenceUi {
    pub open: bool,
    pub opts: SequenceOptions,
    pub run: Option<Run>,
    /// Finestra "Passaggi di render" aperta.
    pub passes_open: bool,
    pub export_passes: PassSet,
    pub export_beauty: bool,
    pub passes_job: Option<JoinHandle<anyhow::Result<PathBuf>>>,
    /// Prova automatica: 0 da avviare, 1 in corso, 2 finita.
    auto: u8,
}

impl Default for SequenceUi {
    fn default() -> Self {
        let all = PassSet {
            albedo: true,
            normals: true,
            depth: true,
            mask: true,
            ids: true,
        };
        Self {
            open: false,
            opts: SequenceOptions::default(),
            run: None,
            passes_open: false,
            export_passes: all,
            export_beauty: true,
            passes_job: None,
            auto: 0,
        }
    }
}

impl ForgeApp {
    /// Avvia la sequenza con le opzioni della finestra, salvando nella cartella scelta.
    pub fn start_sequence(&mut self) {
        if self.sequence.run.is_some() {
            return;
        }
        let o = self.sequence.opts.clone();
        let Some(dir) = o.out_dir.clone() else {
            self.error = Some(tr("scegli prima la cartella in cui salvare").into());
            return;
        };
        if !crate::presets::has_product(&self.scene) {
            self.error = Some(tr("nella scena non c'è un prodotto da riprendere").into());
            return;
        }
        let plan = plan(&o);
        if plan.is_empty() {
            self.error = Some(tr("scegli almeno una vista").into());
            return;
        }
        let turntable = o.kind == SeqKind::Turntable;
        let name = safe_name(&o.name);
        let frames_dir = if turntable {
            dir.join(format!("{name}_fotogrammi"))
        } else {
            dir.clone()
        };
        if let Err(e) = std::fs::create_dir_all(&frames_dir) {
            self.error = Some(tf(
                "impossibile creare la cartella {}: {e}",
                &[frames_dir.display().to_string(), e.to_string()],
            ));
            return;
        }
        let b = crate::presets::product_bounds(&self.scene);
        let center = b.center();
        let points = crate::presets::product_points(&self.scene, 20_000);
        let cameras = plan_cameras(
            &plan,
            self.scene.render_camera(),
            self.settings.aspect(),
            &points,
            center,
            o.fit_frame,
            turntable,
        );
        // Si gira il prodotto, cioè le sue radici (fondale, pavimento e pareti restano fermi).
        let users = crate::presets::product_roots(&self.scene)
            .into_iter()
            .map(|r| (r, self.scene.nodes[r].user))
            .collect();
        let saved = Saved {
            users,
            camera: self.scene.camera.clone(),
            active: self
                .scene
                .active_camera
                .and_then(|i| self.scene.render_cameras.get(i).map(|c| (i, c.camera.clone()))),
            settings: self.settings.clone(),
            unsaved: self.unsaved,
            view: self.view,
            auto_restart: self.auto_restart,
        };
        // Campioni fissi, niente adattivo, denoiser solo alla fine: fotogrammi coerenti tra loro.
        self.settings.target_samples = o.samples.max(1);
        self.settings.noise_threshold = 0.0;
        if self.settings.denoise.mode != DenoiseMode::Off {
            self.settings.denoise.mode = DenoiseMode::Final;
        }
        self.auto_restart = false;
        let video_path = (turntable && o.make_video).then(|| dir.join(format!("{name}.mp4")));
        tracing::info!(
            "sequenza avviata: {} fotogrammi, {} campioni, cartella {}",
            plan.len(),
            o.samples,
            frames_dir.display()
        );
        self.sequence.run = Some(Run {
            plan,
            cameras,
            index: 0,
            phase: Phase::Apply,
            frames_dir,
            video_path,
            fps: o.fps.max(1),
            turns: o.turns.max(1),
            passes: o.passes,
            groups: node_groups(self),
            center,
            saved,
            written: Vec::new(),
            workers: Vec::new(),
            video: None,
            started: Instant::now(),
            frame_started: Instant::now(),
            frame_times: Vec::new(),
            waiting_denoise: None,
            errors: Vec::new(),
            cancelled: false,
        });
    }

    /// Interrompe la sequenza: i fotogrammi già salvati restano, la scena torna com'era.
    pub fn cancel_sequence(&mut self) {
        let Some(run) = &mut self.sequence.run else { return };
        if run.phase == Phase::Finishing {
            return;
        }
        run.cancelled = true;
        self.session.cancel();
        self.finish_sequence();
    }

    /// Prova automatica (`--sequenza <cartella>`): parte quando la scena è pronta e, finita la
    /// sequenza, chiude il programma.
    pub fn auto_sequence(&mut self, ctx: &eframe::egui::Context) {
        let Some(dir) = self.startup.sequence.clone() else { return };
        let ready = self.frames > 30 && !self.is_preparing() && (self.gpu_ready().is_some() || self.safe_mode);
        match self.sequence.auto {
            0 if ready => {
                let o = &mut self.sequence.opts;
                o.out_dir = Some(dir);
                if self.startup.sequence_views {
                    o.kind = SeqKind::Views;
                }
                if let Some(s) = self.startup.sequence_step {
                    o.step_deg = s;
                }
                if let Some(n) = self.startup.sequence_samples {
                    o.samples = n;
                }
                if self.startup.sequence_passes {
                    o.passes = self.sequence.export_passes;
                }
                self.sequence.auto = 1;
                self.start_sequence();
            }
            1 if self.sequence.run.is_none() => {
                self.sequence.auto = 2;
                if let Some(e) = &self.error {
                    tracing::error!("sequenza automatica: {e}");
                }
                self.exit_confirmed = true;
                ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Close);
            }
            _ => {}
        }
    }

    pub fn sequence_running(&self) -> bool {
        self.sequence.run.is_some()
    }

    /// Un passo della sequenza (a ogni frame dell'interfaccia).
    pub fn tick_sequence(&mut self) {
        self.poll_passes_export();
        let Some(phase) = self.sequence.run.as_ref().map(|r| r.phase) else {
            return;
        };
        match phase {
            Phase::Apply => self.apply_sequence_frame(),
            Phase::Rendering => self.poll_sequence_frame(),
            Phase::Finishing => self.poll_sequence_end(),
        }
    }

    fn apply_sequence_frame(&mut self) {
        let Some(run) = &self.sequence.run else { return };
        let k = run.index;
        let spin = spin_matrix(run.center, run.plan[k].spin_deg);
        let cam = run.cameras[k].clone();
        let users = run.saved.users.clone();
        for (i, u) in users {
            self.scene.nodes[i].user = u;
            if run.plan[k].spin_deg != 0.0 {
                let m = spin * self.scene.user_matrix(i);
                let pivot = self.scene.nodes[i].pivot;
                let frame = self.scene.frame_of(i);
                self.scene.nodes[i].user = UserTransform::from_matrix(m, pivot, frame);
            }
        }
        match self.scene.active_camera {
            Some(i) => self.scene.render_cameras[i].camera = cam.clone(),
            None => self.scene.camera = cam.clone(),
        }
        self.scene.camera = cam;
        self.sync_orbit_from_camera();
        self.geometry_changed();
        self.camera_rev_bump();
        self.view = View::Render;
        self.start_render();
        if let Some(run) = &mut self.sequence.run {
            run.phase = Phase::Rendering;
            run.frame_started = Instant::now();
            run.waiting_denoise = None;
        }
    }

    fn poll_sequence_frame(&mut self) {
        if self.is_preparing() {
            return;
        }
        let Some(job) = self.job_id() else {
            // La geometria è arrivata ma il render non è partito (es. nebbia): si riprova.
            self.start_render();
            return;
        };
        let p = self.session.progress();
        if p.job_id != job {
            return;
        }
        match p.state {
            SessionState::Completed => {}
            SessionState::Failed(e) => {
                if let Some(run) = &mut self.sequence.run {
                    run.errors.push(e);
                    run.cancelled = true;
                }
                return self.finish_sequence();
            }
            SessionState::Cancelled => {
                // Fermato dalla barra (Stop): vale come interruzione della sequenza.
                if let Some(run) = &mut self.sequence.run {
                    run.cancelled = true;
                }
                return self.finish_sequence();
            }
            _ => return,
        }
        let Some(frame) = self.session.latest_frame() else { return };
        if frame.job_id != job || frame.samples < self.settings.target_samples {
            return;
        }
        // Con il denoiser si aspetta l'immagine ripulita (al massimo 30 s, poi quella grezza).
        let want_denoise = self.settings.denoise.mode != DenoiseMode::Off && self.denoiser_info.is_ok();
        if want_denoise && frame.denoised_samples != frame.samples {
            let run = self.sequence.run.as_mut().expect("sequenza");
            let since = *run.waiting_denoise.get_or_insert_with(Instant::now);
            if since.elapsed() < Duration::from_secs(30) {
                return;
            }
        }
        let (tone, ev, strength) = (
            self.settings.tone_mapping,
            self.settings.exposure_ev,
            self.settings.denoise.strength,
        );
        let render_scene = self.last_render_scene_arc();
        let settings = self.settings.clone();
        let run = self.sequence.run.as_mut().expect("sequenza");
        let path = run.frames_dir.join(format!("{}.png", run.plan[run.index].file));
        match forge_image_io::save_frame_png(&path, &frame, tone, ev, strength) {
            Ok(()) => run.written.push(path.clone()),
            Err(e) => {
                run.errors.push(format!("{}: {e:#}", path.display()));
                run.cancelled = true;
                return self.finish_sequence();
            }
        }
        if run.passes.any() {
            if let Some(rs) = render_scene {
                let (groups, set, base) = (run.groups.clone(), run.passes, path.with_extension(""));
                run.workers
                    .push(std::thread::spawn(move || write_passes(&rs, &settings, &groups, set, &base)));
            }
        }
        run.frame_times.push(run.frame_started.elapsed());
        run.index += 1;
        if run.index >= run.plan.len() {
            self.finish_sequence();
        } else {
            run.phase = Phase::Apply;
        }
    }

    /// Rimette scena, camera e impostazioni come prima; poi (se c'è) parte il video.
    fn finish_sequence(&mut self) {
        let Some(run) = &mut self.sequence.run else { return };
        run.phase = Phase::Finishing;
        for (i, u) in &run.saved.users {
            self.scene.nodes[*i].user = *u;
        }
        self.scene.camera = run.saved.camera.clone();
        if let Some((i, c)) = &run.saved.active {
            if let Some(rc) = self.scene.render_cameras.get_mut(*i) {
                rc.camera = c.clone();
            }
        }
        self.settings = run.saved.settings.clone();
        self.auto_restart = run.saved.auto_restart;
        let (unsaved, view) = (run.saved.unsaved, run.saved.view);
        if !run.cancelled && !run.written.is_empty() {
            if let Some(out) = run.video_path.clone() {
                let (frames, fps, turns) = (run.written.clone(), run.fps, run.turns);
                run.video = Some(std::thread::spawn(move || crate::video::encode_mp4(&frames, &out, fps, turns)));
            }
        }
        self.sync_orbit_from_camera();
        self.geometry_changed();
        self.camera_rev_bump();
        self.unsaved = unsaved;
        self.view = view;
    }

    fn poll_sequence_end(&mut self) {
        let Some(run) = &mut self.sequence.run else { return };
        if run.workers.iter().any(|w| !w.is_finished()) || run.video.as_ref().is_some_and(|v| !v.is_finished()) {
            return;
        }
        let mut errors = std::mem::take(&mut run.errors);
        for w in run.workers.drain(..) {
            match w.join() {
                Ok(Ok(())) => {}
                Ok(Err(e)) => errors.push(format!("{e:#}")),
                Err(_) => errors.push(tr("calcolo dei passaggi interrotto").into()),
            }
        }
        let mut video_msg = None;
        if let Some(v) = run.video.take() {
            match v.join() {
                Ok(Ok(enc)) => {
                    let p = run.video_path.as_ref().map(|p| p.display().to_string()).unwrap_or_default();
                    video_msg = Some(tf("Video: {} ({})", &[p, enc.to_string()]));
                }
                Ok(Err(e)) => errors.push(tf("video non creato: {e:#}", &[format!("{e:#}")])),
                Err(_) => errors.push(tr("video non creato").into()),
            }
        }
        let run = self.sequence.run.take().expect("sequenza");
        let mut text = if run.cancelled {
            tf(
                "Sequenza interrotta: {} immagini di {} salvate in\n{}",
                &[
                    format!("{}", run.written.len()),
                    format!("{}", run.plan.len()),
                    run.frames_dir.display().to_string(),
                ],
            )
        } else {
            tf(
                "Sequenza completata: {} immagini in {} salvate in\n{}",
                &[
                    format!("{}", run.written.len()),
                    format_duration(run.started.elapsed()),
                    run.frames_dir.display().to_string(),
                ],
            )
        };
        if let Some(v) = video_msg {
            text = format!("{text}\n{v}");
        }
        tracing::info!("{}", text.replace('\n', " "));
        if errors.is_empty() {
            self.info = Some(text);
        } else {
            self.error = Some(format!("{text}\n\n{}", errors.join("\n")));
        }
    }

    /// Esporta i passaggi dell'ultimo render (finestra "Passaggi di render").
    pub fn export_passes(&mut self, base: PathBuf) {
        let Some(rs) = self.last_render_scene_arc() else {
            self.error = Some(tr("nessuna immagine: avvia prima un rendering").into());
            return;
        };
        let set = self.sequence.export_passes;
        let beauty = self.sequence.export_beauty;
        if beauty {
            if let Some(frame) = self.session.latest_frame() {
                let p = base.with_extension("png");
                if let Err(e) = forge_image_io::save_frame_png(
                    &p,
                    &frame,
                    self.settings.tone_mapping,
                    self.settings.exposure_ev,
                    self.settings.denoise.strength,
                ) {
                    self.error = Some(tf("esportazione fallita: {e:#}", &[format!("{e:#}")]));
                    return;
                }
            }
        }
        // Le dimensioni sono quelle del render (la scena compilata usa la sua camera).
        let mut settings = self.settings.clone();
        if let Some(f) = self.session.latest_frame() {
            settings.width = f.width;
            settings.height = f.height;
        }
        let groups = node_groups(self);
        let stem = base.with_extension("");
        self.sequence.passes_job = Some(std::thread::spawn(move || {
            write_passes(&rs, &settings, &groups, set, &stem)?;
            Ok(stem)
        }));
    }

    fn poll_passes_export(&mut self) {
        if !self.sequence.passes_job.as_ref().is_some_and(|j| j.is_finished()) {
            return;
        }
        match self.sequence.passes_job.take().expect("job").join() {
            Ok(Ok(stem)) => {
                self.info = Some(tf("Passaggi di render salvati accanto a\n{}", &[stem.display().to_string()]));
            }
            Ok(Err(e)) => self.error = Some(tf("passaggi non salvati: {e:#}", &[format!("{e:#}")])),
            Err(_) => self.error = Some(tr("passaggi non salvati").into()),
        }
    }
}

pub fn format_duration(d: Duration) -> String {
    let s = d.as_secs();
    if s >= 3600 {
        format!("{} h {:02} min", s / 3600, (s % 3600) / 60)
    } else if s >= 60 {
        format!("{} min {:02} s", s / 60, s % 60)
    } else {
        format!("{:.1} s", d.as_secs_f32())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turntable_timing() {
        assert_eq!(turntable_frames(2.0), 180);
        assert!((turn_seconds(2.0, 30) - 6.0).abs() < 1e-6);
        assert!((step_for_duration(12.0, 30) - 1.0).abs() < 1e-6);
        // Passi che non dividono 360: si arrotonda ai fotogrammi e il giro si chiude esatto.
        assert_eq!(turntable_frames(7.0), 51);
        assert!((effective_step(7.0) * 51.0 - 360.0).abs() < 1e-3);
    }

    #[test]
    fn turntable_closes_the_loop_without_repeating_the_first_frame() {
        let o = SequenceOptions {
            step_deg: 90.0,
            ..Default::default()
        };
        let p = plan(&o);
        let spins: Vec<f32> = p.iter().map(|f| f.spin_deg).collect();
        assert_eq!(spins, vec![0.0, 90.0, 180.0, 270.0]);
        assert_eq!(p[0].file, "prodotto_0001");
        let cw = plan(&SequenceOptions { clockwise: true, ..o });
        assert_eq!(cw[1].spin_deg, -90.0);
    }

    #[test]
    fn views_rotate_the_product_toward_the_camera() {
        let mut o = SequenceOptions {
            kind: SeqKind::Views,
            name: "sedia rossa".into(),
            ..Default::default()
        };
        o.views = [true, false, true, false, false, false, true];
        let p = plan(&o);
        assert_eq!(p.len(), 3);
        assert_eq!(p[0].file, "sedia-rossa_frontale");
        // Da destra: il lato +X del prodotto deve girarsi verso la camera (+Z).
        let m = spin_matrix(Vec3::ZERO, p[1].spin_deg);
        assert!((m.transform_point3(Vec3::X) - Vec3::Z).length() < 1e-5);
        assert!(p[2].top);
    }

    /// L'inquadratura adattata contiene tutti i punti con il margine, e per il turntable resta
    /// la stessa in ogni fotogramma e contiene il prodotto in ogni posizione del giro.
    #[test]
    fn fitted_camera_keeps_the_product_inside() {
        let cam = Camera {
            position: Vec3::new(0.0, 0.5, 3.0),
            target: Vec3::new(0.0, 0.2, 0.0),
            vfov_deg: 35.0,
            ..Default::default()
        };
        // Prodotto lungo e stretto: girandolo cambia molto l'ingombro.
        let pts: Vec<Vec3> = [-1.0f32, 1.0]
            .iter()
            .flat_map(|x| [0.0f32, 0.4].map(|y| [-0.1f32, 0.1].map(move |z| Vec3::new(*x, y, z))))
            .flatten()
            .collect();
        let aspect = 16.0 / 9.0;
        let check = |c: &Camera, pts: &[Vec3]| {
            let (r, u, f) = c.basis();
            let tv = (c.vfov_deg.to_radians() * 0.5).tan();
            let worst = pts
                .iter()
                .map(|p| {
                    let q = *p - c.position;
                    let z = q.dot(f);
                    (q.dot(r) / (z * tv * aspect)).abs().max((q.dot(u) / (z * tv)).abs())
                })
                .fold(0.0f32, f32::max);
            assert!(worst <= 0.85 + 1e-3 && worst > 0.8, "{worst}");
        };
        let o = SequenceOptions {
            step_deg: 30.0,
            ..Default::default()
        };
        let p = plan(&o);
        let cams = plan_cameras(&p, &cam, aspect, &pts, Vec3::new(0.0, 0.2, 0.0), true, true);
        assert!(cams.windows(2).all(|w| w[0] == w[1]), "una sola inquadratura per il giro");
        let all: Vec<Vec3> = p.iter().flat_map(|f| spun(&pts, Vec3::new(0.0, 0.2, 0.0), f.spin_deg)).collect();
        check(&cams[0], &all);
        // Direzione di vista invariata.
        assert!((cams[0].forward() - cam.forward()).length() < 1e-4);
        // Vista frontale da sola: più vicina (il prodotto di lato non serve contenerlo).
        let front = fit_camera(&cam, aspect, &pts, 0.85);
        check(&front, &pts);
        assert!((front.position - front.target).length() < (cams[0].position - cams[0].target).length());
    }

    #[test]
    fn top_camera_looks_down_with_the_front_at_the_bottom() {
        let cam = Camera {
            position: Vec3::new(0.0, 0.0, 3.0),
            target: Vec3::ZERO,
            ..Default::default()
        };
        let t = top_camera(&cam, Vec3::ZERO);
        assert!((t.forward() - (-Vec3::Y)).length() < 1e-5);
        // Il lato verso la camera originale (+Z) va in basso: l'alto dell'immagine è -Z.
        let (_, up, _) = t.basis();
        assert!((up - (-Vec3::Z)).length() < 1e-5, "{up:?}");
    }
}
