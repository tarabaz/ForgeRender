//! Stato dell'applicazione e orchestrazione: scena, progetto, sessione di
//! rendering, viewport. La UI vera e propria è nei moduli di `ui`.

use crate::viewport::ViewportRenderer;
use eframe::egui;
use forge_import::ImportOptions;
use forge_project::ProjectMeta;
use forge_render_core::{RenderGeometry, RenderScene, RenderSession, RenderSettings, SessionState, ToneMapping};
use forge_render_cpu::CpuBackend;
use forge_scene::{MaterialId, OrbitController, Scene};
use forge_telemetry::{LogBuffer, ResourceMonitor, SystemInfo};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Viewport,
    Render,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InspectorTab {
    Materials,
    Lights,
    Environment,
    Camera,
    Render,
}

/// Cosa è stato usato per avviare il rendering corrente (per capire quando ripartire).
struct JobInfo {
    geometry_rev: u64,
    shading_rev: u64,
    key: (u32, u32, u32, u32, u32, u64, bool),
    threads: usize,
    target: u32,
}

/// Compilazione della geometria in corso su un thread separato.
struct PendingGeometry {
    rev: u64,
    rx: Receiver<Arc<RenderGeometry>>,
    then_render: bool,
}

/// Opzioni da riga di comando utili a test automatici della GUI.
#[derive(Clone, Debug, Default)]
pub struct StartupOptions {
    pub open: Option<PathBuf>,
    pub safe_mode: bool,
    pub auto_render: bool,
    pub screenshot: Option<PathBuf>,
    pub screenshot_after_samples: u32,
}

pub struct ForgeApp {
    pub scene: Scene,
    pub meta: ProjectMeta,
    pub settings: RenderSettings,
    pub orbit: OrbitController,
    pub selection: BTreeSet<usize>,
    pub selected_light: Option<usize>,
    pub selected_material: MaterialId,
    pub view: View,
    pub tab: InspectorTab,
    pub auto_restart: bool,
    pub import_opts: ImportOptions,
    pub unsaved: bool,
    pub error: Option<String>,
    pub info: Option<String>,
    pub rule_pattern: String,
    pub collapsed: BTreeSet<usize>,
    pub custom_samples: u32,
    pub preview: crate::ui::central::MaterialPreview,

    /// Revisione della geometria (import, visibilità): invalida la BVH.
    pub geometry_rev: u64,
    /// Revisione di materiali, luci, ambiente, camera: invalida l'accumulo.
    pub shading_rev: u64,
    /// Revisione di materiali e assegnazioni (colori della viewport).
    pub appearance_rev: u64,

    geometry: Option<(u64, Arc<RenderGeometry>)>,
    pending_geometry: Option<PendingGeometry>,
    job: Option<JobInfo>,
    pub session: RenderSession,

    pub viewport: Option<ViewportRenderer>,
    viewport_uploaded: Option<(u64, u64, BTreeSet<usize>)>,
    pub render_texture: Option<egui::TextureHandle>,
    render_texture_key: Option<(u64, ToneMapping, u32)>,

    pub logs: LogBuffer,
    pub system: SystemInfo,
    pub monitor: ResourceMonitor,
    pub safe_mode: bool,
    startup: StartupOptions,
    screenshot_requested: bool,
    screenshot_done: Arc<std::sync::atomic::AtomicBool>,
    frames: u64,
}

impl ForgeApp {
    pub fn new(cc: &eframe::CreationContext<'_>, logs: LogBuffer, system: SystemInfo, startup: StartupOptions) -> Self {
        crate::ui::style::apply(&cc.egui_ctx);
        let viewport = cc.wgpu_render_state.as_ref().map(ViewportRenderer::new);
        if viewport.is_none() {
            tracing::warn!("modalità sicura: viewport 3D GPU non disponibile, il rendering CPU resta utilizzabile");
        }
        let scene = Scene::test_scene();
        let orbit = OrbitController::from_camera(&scene.camera);
        let mut app = Self {
            scene,
            meta: ProjectMeta {
                test_scene: true,
                ..Default::default()
            },
            settings: RenderSettings::default(),
            orbit,
            selection: BTreeSet::new(),
            selected_light: None,
            selected_material: MaterialId::new("default"),
            view: View::Viewport,
            tab: InspectorTab::Materials,
            auto_restart: true,
            import_opts: ImportOptions::default(),
            unsaved: false,
            error: None,
            info: None,
            rule_pattern: String::new(),
            collapsed: BTreeSet::new(),
            custom_samples: 2048,
            preview: Default::default(),
            geometry_rev: 1,
            shading_rev: 1,
            appearance_rev: 1,
            geometry: None,
            pending_geometry: None,
            job: None,
            session: RenderSession::new(),
            viewport,
            viewport_uploaded: None,
            render_texture: None,
            render_texture_key: None,
            logs,
            system,
            monitor: ResourceMonitor::new(),
            safe_mode: startup.safe_mode,
            startup: startup.clone(),
            screenshot_requested: false,
            screenshot_done: Default::default(),
            frames: 0,
        };
        if let Some(p) = &startup.open {
            app.open_any(p);
        }
        if startup.auto_render {
            app.start_render();
        }
        app
    }

    // ------------------------------------------------------------------
    // Revisioni e invalidazione
    // ------------------------------------------------------------------

    /// Materiali, luci, ambiente o camera modificati.
    pub fn shading_changed(&mut self) {
        self.shading_rev += 1;
        self.unsaved = true;
    }

    /// Materiali o assegnazioni modificati (cambiano anche i colori della viewport).
    pub fn appearance_changed(&mut self) {
        self.appearance_rev += 1;
        self.shading_changed();
    }

    /// Geometria, visibilità o struttura modificate.
    pub fn geometry_changed(&mut self) {
        self.geometry_rev += 1;
        self.appearance_rev += 1;
        self.shading_rev += 1;
        self.unsaved = true;
    }

    /// Camera spostata dall'utente (orbita/pan/zoom).
    pub fn camera_changed(&mut self) {
        self.scene.camera = self.orbit.camera();
        self.shading_rev += 1;
    }

    pub fn sync_orbit_from_camera(&mut self) {
        self.orbit = OrbitController::from_camera(&self.scene.camera);
    }

    fn current_geometry(&self) -> Option<Arc<RenderGeometry>> {
        self.geometry
            .as_ref()
            .filter(|(r, _)| *r == self.geometry_rev)
            .map(|(_, g)| g.clone())
    }

    /// Geometria compilata sincrona (per il picking); costruisce la BVH se serve.
    pub fn geometry_now(&mut self) -> Arc<RenderGeometry> {
        if let Some(g) = self.current_geometry() {
            return g;
        }
        let g = Arc::new(RenderGeometry::build(&self.scene));
        self.geometry = Some((self.geometry_rev, g.clone()));
        g
    }

    /// Avvia la costruzione asincrona della BVH (non blocca l'interfaccia).
    fn request_geometry(&mut self, then_render: bool) {
        if self.pending_geometry.as_ref().is_some_and(|p| p.rev == self.geometry_rev) {
            if then_render {
                if let Some(p) = &mut self.pending_geometry {
                    p.then_render = true;
                }
            }
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let scene = self.scene.clone();
        let rev = self.geometry_rev;
        std::thread::Builder::new()
            .name("forge-geometry".into())
            .spawn(move || {
                let _ = tx.send(Arc::new(RenderGeometry::build(&scene)));
            })
            .expect("thread di compilazione");
        self.pending_geometry = Some(PendingGeometry { rev, rx, then_render });
    }

    pub fn is_preparing(&self) -> bool {
        self.pending_geometry.is_some()
    }

    fn poll_geometry(&mut self) {
        let Some(p) = &self.pending_geometry else { return };
        match p.rx.try_recv() {
            Ok(g) => {
                let (rev, then_render) = (p.rev, p.then_render);
                self.pending_geometry = None;
                self.geometry = Some((rev, g));
                if then_render {
                    self.start_render();
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(_) => {
                self.pending_geometry = None;
                self.error = Some("compilazione della geometria fallita".into());
            }
        }
    }

    // ------------------------------------------------------------------
    // Rendering
    // ------------------------------------------------------------------

    pub fn start_render(&mut self) {
        let Some(geometry) = self.current_geometry() else {
            self.request_geometry(true);
            self.view = View::Render;
            return;
        };
        let backend = match CpuBackend::from_settings(&self.settings) {
            Ok(b) => b,
            Err(e) => {
                self.error = Some(format!("impossibile creare il backend CPU: {e}"));
                return;
            }
        };
        let rs = Arc::new(RenderScene::compile_with_geometry(&self.scene, geometry));
        self.session.start(rs, self.settings.clone(), Box::new(backend));
        self.job = Some(JobInfo {
            geometry_rev: self.geometry_rev,
            shading_rev: self.shading_rev,
            key: self.settings.accumulation_key(),
            threads: self.settings.effective_threads(),
            target: self.settings.target_samples,
        });
        self.view = View::Render;
    }

    pub fn render_state(&self) -> SessionState {
        self.session.progress().state
    }

    /// Vero se il render corrente non corrisponde più alla scena/impostazioni.
    pub fn render_is_stale(&self) -> bool {
        match &self.job {
            None => false,
            Some(j) => {
                j.geometry_rev != self.geometry_rev
                    || j.shading_rev != self.shading_rev
                    || j.key != self.settings.accumulation_key()
                    || j.threads != self.settings.effective_threads()
            }
        }
    }

    fn update_render_job(&mut self) {
        let Some(job) = &mut self.job else { return };
        if job.target != self.settings.target_samples {
            job.target = self.settings.target_samples;
            self.session.set_target(job.target);
        }
        let state = self.session.progress().state;
        if self.auto_restart && state.is_active() && self.render_is_stale() && !self.is_preparing() {
            self.start_render();
        }
    }

    pub fn stop_render(&mut self) {
        self.session.cancel();
    }

    pub fn toggle_pause(&mut self) {
        match self.render_state() {
            SessionState::Rendering => self.session.pause(),
            SessionState::Paused | SessionState::Cancelled => self.session.resume(),
            _ => self.start_render(),
        }
    }

    /// Aggiorna la texture del render se è arrivato un nuovo frame o se è cambiato il tone mapping.
    pub fn render_texture(&mut self, ctx: &egui::Context) -> Option<&egui::TextureHandle> {
        let version = self.session.frame_version();
        let key = (version, self.settings.tone_mapping, self.settings.exposure_ev.to_bits());
        if self.render_texture_key != Some(key) {
            if let Some(frame) = self.session.latest_frame() {
                let rgba = forge_render_core::tonemap_to_rgba8(&frame.linear, self.settings.tone_mapping, self.settings.exposure_ev);
                let img = egui::ColorImage::from_rgba_unmultiplied([frame.width as usize, frame.height as usize], &rgba);
                match &mut self.render_texture {
                    Some(t) => t.set(img, egui::TextureOptions::LINEAR),
                    None => self.render_texture = Some(ctx.load_texture("render", img, egui::TextureOptions::LINEAR)),
                }
                self.render_texture_key = Some(key);
            }
        }
        self.render_texture.as_ref()
    }

    pub fn export_png_dialog(&mut self) {
        let Some(frame) = self.session.latest_frame() else {
            self.error = Some("nessuna immagine da esportare: avvia prima un rendering".into());
            return;
        };
        let default_name = format!("render_{}spp.png", frame.samples);
        let Some(path) = rfd::FileDialog::new()
            .add_filter("PNG", &["png"])
            .set_file_name(default_name)
            .save_file()
        else {
            return;
        };
        let path = with_extension(path, "png");
        match forge_image_io::save_frame_png(&path, &frame, self.settings.tone_mapping, self.settings.exposure_ev) {
            Ok(()) => {
                tracing::info!("immagine esportata: {} ({} campioni)", path.display(), frame.samples);
                self.info = Some(format!("Immagine esportata ({} campioni):\n{}", frame.samples, path.display()));
            }
            Err(e) => self.error = Some(format!("esportazione fallita: {e:#}")),
        }
    }

    // ------------------------------------------------------------------
    // Scena e progetto
    // ------------------------------------------------------------------

    fn replace_scene(&mut self, scene: Scene, meta: ProjectMeta, settings: Option<RenderSettings>) {
        self.session.cancel();
        self.job = None;
        self.scene = scene;
        self.meta = meta;
        if let Some(s) = settings {
            self.settings = s;
        }
        self.selection.clear();
        self.selected_light = None;
        self.collapsed.clear();
        self.sync_orbit_from_camera();
        self.geometry_changed();
        self.unsaved = false;
        self.view = View::Viewport;
    }

    pub fn new_test_scene(&mut self) {
        self.replace_scene(
            Scene::test_scene(),
            ProjectMeta {
                test_scene: true,
                ..Default::default()
            },
            None,
        );
    }

    pub fn new_empty_scene(&mut self) {
        let mut s = Scene::default();
        s.lights = forge_scene::light::preset("studio", forge_math::Vec3::ZERO, 1.0).unwrap_or_default();
        self.replace_scene(s, ProjectMeta::default(), None);
    }

    pub fn import_dialog(&mut self) {
        let files = rfd::FileDialog::new()
            .add_filter("Mesh (OBJ, STL)", &forge_import::SUPPORTED_EXTENSIONS)
            .pick_files();
        for f in files.unwrap_or_default() {
            self.import_path(&f);
        }
    }

    pub fn import_path(&mut self, path: &Path) {
        // Se lo stesso file era già stato importato, lo si sostituisce (stessi id stabili).
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
        if let Some(old) = self.scene.find(&forge_scene::ComponentId(file_name)) {
            self.scene.remove_subtree(old);
            self.selection.clear();
        }
        let was_empty = self.scene.nodes.is_empty();
        match forge_import::import_file(&mut self.scene, path, &self.import_opts) {
            Ok(r) => {
                let project_dir = self.meta.path.as_deref().and_then(Path::parent);
                self.meta.sources.retain(|s| s.absolute_path.file_name() != path.file_name());
                self.meta
                    .sources
                    .push(forge_project::source_ref(project_dir, path, self.import_opts.clone()));
                if was_empty && self.meta.test_scene {
                    self.meta.test_scene = false;
                }
                self.geometry_changed();
                self.selection = BTreeSet::from([r.root]);
                let b = self.scene.bounds_of(r.root);
                if was_empty || self.scene.lights.is_empty() {
                    self.scene.lights = forge_scene::light::preset("studio", b.center(), b.radius()).unwrap_or_default();
                }
                self.orbit.frame(&b);
                self.camera_changed();
                let mut msg = format!(
                    "{}: {} componenti, {} triangoli ({} ms)",
                    path.display(),
                    r.components,
                    r.triangles,
                    r.duration.as_millis()
                );
                if !r.warnings.is_empty() {
                    msg.push_str(&format!("\nAvvisi:\n- {}", r.warnings.join("\n- ")));
                    self.info = Some(msg);
                }
            }
            Err(e) => self.error = Some(format!("importazione di {} fallita: {e}", path.display())),
        }
    }

    pub fn open_project_dialog(&mut self) {
        if let Some(p) = rfd::FileDialog::new()
            .add_filter("Progetto ForgeRender", &[forge_project::EXTENSION])
            .pick_file()
        {
            self.open_project(&p);
        }
    }

    /// Apre un progetto o importa una mesh in base all'estensione.
    pub fn open_any(&mut self, path: &Path) {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        if ext == forge_project::EXTENSION {
            self.open_project(path);
        } else {
            self.import_path(path);
        }
    }

    pub fn open_project(&mut self, path: &Path) {
        match forge_project::load(path) {
            Ok(l) => {
                let report = l.report;
                self.replace_scene(l.scene, l.meta, Some(l.settings));
                let mut notes = Vec::new();
                if !report.missing_sources.is_empty() {
                    notes.push(format!(
                        "File sorgente non trovati:\n- {}",
                        report
                            .missing_sources
                            .iter()
                            .map(|p| p.display().to_string())
                            .collect::<Vec<_>>()
                            .join("\n- ")
                    ));
                }
                if !report.missing_components.is_empty() {
                    notes.push(format!(
                        "{} componenti con materiale assegnato non sono più presenti (rinominati o rimossi). Le assegnazioni sono conservate:\n- {}",
                        report.missing_components.len(),
                        report.missing_components.iter().take(20).map(|c| c.0.clone()).collect::<Vec<_>>().join("\n- ")
                    ));
                }
                notes.extend(report.warnings);
                if !notes.is_empty() {
                    self.info = Some(notes.join("\n\n"));
                }
            }
            Err(e) => self.error = Some(format!("apertura del progetto fallita: {e:#}")),
        }
    }

    pub fn save_project(&mut self, save_as: bool) {
        let path = match (&self.meta.path, save_as) {
            (Some(p), false) => p.clone(),
            _ => {
                let Some(p) = rfd::FileDialog::new()
                    .add_filter("Progetto ForgeRender", &[forge_project::EXTENSION])
                    .set_file_name("progetto.frproject")
                    .save_file()
                else {
                    return;
                };
                with_extension(p, forge_project::EXTENSION)
            }
        };
        match forge_project::save(&path, &self.scene, &self.meta, &self.settings) {
            Ok(()) => {
                self.meta.path = Some(path);
                self.unsaved = false;
            }
            Err(e) => self.error = Some(format!("salvataggio fallito: {e:#}")),
        }
    }

    pub fn frame_all(&mut self) {
        let b = self.scene.bounds();
        self.orbit.frame(&b);
        self.camera_changed();
    }

    pub fn frame_selection(&mut self) {
        let mut b = forge_math::Aabb::EMPTY;
        for &s in &self.selection {
            b.grow(&self.scene.bounds_of(s));
        }
        if b.is_empty() {
            return self.frame_all();
        }
        self.orbit.frame(&b);
        self.camera_changed();
    }

    /// Prepara la viewport (upload geometria se cambiata) e restituisce la texture.
    pub fn viewport_texture(&mut self, frame: &eframe::Frame, size: [u32; 2]) -> Option<egui::TextureId> {
        let rs = frame.wgpu_render_state()?;
        let vp = self.viewport.as_mut()?;
        // La camera non richiede un nuovo upload: contano geometria, materiali e selezione.
        let key = (self.geometry_rev, self.appearance_rev, self.selection.clone());
        if self.viewport_uploaded.as_ref() != Some(&key) {
            vp.upload_scene(rs, &self.scene, &self.selection);
            self.viewport_uploaded = Some(key);
        }
        let radius = self.scene.bounds().radius().max(1e-3);
        vp.render(rs, &self.scene.camera, radius, size, forge_math::Vec3::splat(0.045))
    }
}

impl ForgeApp {
    /// Chiamata a ogni frame prima del disegno dell'interfaccia.
    pub fn tick(&mut self, ctx: &egui::Context) {
        self.frames += 1;
        self.poll_geometry();
        self.update_render_job();
        self.monitor.tick();
        let state = self.render_state();
        if state == SessionState::Rendering || self.is_preparing() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
        self.handle_screenshot(ctx);
        let title = format!(
            "ForgeRender — {}{}",
            self.meta
                .path
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "senza titolo".into()),
            if self.unsaved { " *" } else { "" }
        );
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
    }

    /// Screenshot automatico per i test della GUI (`--screenshot`).
    fn handle_screenshot(&mut self, ctx: &egui::Context) {
        let Some(path) = self.startup.screenshot.clone() else { return };
        if self.screenshot_done.load(std::sync::atomic::Ordering::Acquire) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        let ready = self.frames > 30
            && (self.startup.screenshot_after_samples == 0 || self.session.progress().samples >= self.startup.screenshot_after_samples);
        if ready && !self.screenshot_requested {
            self.screenshot_requested = true;
            let done = self.screenshot_done.clone();
            let ctx2 = ctx.clone();
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::ScreenshotCallback::new(move |img| {
                let [w, h] = img.size;
                let rgba: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
                match forge_image_io::save_png(&path, w as u32, h as u32, &rgba) {
                    Ok(()) => tracing::info!("screenshot salvato in {}", path.display()),
                    Err(e) => tracing::error!("screenshot fallito: {e:#}"),
                }
                done.store(true, std::sync::atomic::Ordering::Release);
                ctx2.request_repaint();
            })));
        }
        ctx.request_repaint();
    }
}

fn with_extension(p: PathBuf, ext: &str) -> PathBuf {
    if p.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case(ext)) {
        p
    } else {
        p.with_extension(ext)
    }
}
