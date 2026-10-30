//! Stato dell'applicazione e orchestrazione: scena, progetto, sessione di
//! rendering, viewport. La UI vera e propria è nei moduli di `ui`.

use crate::i18n::{tf, tk, tr};
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

/// Nome, versione e commit della build, per il titolo della finestra
/// (es. "ForgeRender v0.1.0 (a1b2c3d)"; "dev" se compilato senza Git).
pub const APP_TITLE: &str = concat!("ForgeRender v", env!("CARGO_PKG_VERSION"), " (", env!("FORGE_GIT_SHA"), ")");

/// Tipi di luce che si possono aggiungere (pannello Luci, menu Oggetti → Luce).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NewLight {
    Area,
    Point,
    Spot,
    Directional,
    Portal,
}

impl NewLight {
    pub const ALL: [NewLight; 5] = [
        NewLight::Area,
        NewLight::Point,
        NewLight::Spot,
        NewLight::Directional,
        NewLight::Portal,
    ];

    pub fn label(self) -> &'static str {
        match self {
            NewLight::Area => tk("Area"),
            NewLight::Point => tk("Punto"),
            NewLight::Spot => tk("Spot"),
            NewLight::Directional => tk("Direzionale"),
            NewLight::Portal => tk("Portale (finestra)"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Viewport,
    Render,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InspectorTab {
    Object,
    Materials,
    Lights,
    Environment,
    Effects,
    Camera,
    Render,
}

/// Cosa è stato usato per avviare il rendering corrente (per capire quando ripartire).
struct JobInfo {
    backend: forge_render_core::BackendKind,
    geometry_rev: u64,
    shading_rev: u64,
    camera_rev: u64,
    key: (u32, u32, u32, u32, u32, u64, bool),
    threads: usize,
    target: u32,
    /// Soglia di rumore e campioni minimi inviati alla sessione.
    adaptive: (f32, u32),
}

/// Compilazione della geometria in corso su un thread separato.
struct PendingGeometry {
    rev: u64,
    rx: Receiver<Arc<RenderGeometry>>,
    then_render: bool,
}

/// Messaggi dalle operazioni in background.
pub enum TaskMsg {
    Progress(String),
    Model(Result<forge_import::frmesh::FrMesh, String>),
    Blend(Result<forge_import::blender::BlendExport, String>),
    Project(Result<Box<forge_project::LoadedProject>, String>),
}

/// Operazione lenta in corso su un altro thread (import CAD, apertura progetto).
pub struct Pending {
    pub label: String,
    pub message: String,
    path: PathBuf,
    opts: ImportOptions,
    rx: Receiver<TaskMsg>,
    pub started: std::time::Instant,
}

/// Opzioni da riga di comando utili a test automatici della GUI.
#[derive(Clone, Debug, Default)]
pub struct StartupOptions {
    pub open: Option<PathBuf>,
    pub safe_mode: bool,
    pub auto_render: bool,
    pub screenshot: Option<PathBuf>,
    pub screenshot_after_samples: u32,
    /// Apre la finestra della libreria.
    pub library: bool,
    /// Apre la finestra delle impostazioni.
    pub settings: bool,
    /// Modalità dell'interfaccia per questa sessione (non salvata): "semplice" o "avanzata".
    pub ui_mode: Option<crate::prefs::UiMode>,
    /// Vista isometrica all'avvio (test e screenshot).
    pub iso: bool,
    /// "Controllo IA" acceso all'avvio (`--controllo-ia`, prove automatiche).
    pub mcp: bool,
    /// Esegue "Prepara scena prodotto" all'avvio con l'aspetto indicato (test e screenshot).
    pub prepare: Option<crate::presets::Look>,
    /// Scheda dell'inspector: oggetto | materiali | luci | ambiente | camera | render.
    pub tab: Option<String>,
    /// Seleziona il componente con questo nome.
    pub select: Option<String>,
    /// Luce da selezionare all'avvio (per nome).
    pub light: Option<String>,
    /// Seleziona la nebbia con questo nome (prove automatiche).
    pub fog: Option<String>,
    /// Aggancia la camera all'oggetto con questo nome (prove automatiche).
    pub aim: Option<String>,
    /// Modalità del gizmo: move | rotate | scale.
    pub gizmo: Option<String>,
    /// Crea una camera di rendering: "lato" sposta la vista per mostrarla, "attraverso" la guarda.
    pub camera_test: Option<String>,
}

/// Stato di un componente salvato per annulla/ripeti (trasformazione e visibilità).
#[derive(Clone, Copy, Debug)]
pub struct NodeState {
    user: forge_scene::UserTransform,
    visible: bool,
    removed: bool,
}

/// Contenuto della vista Render.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderView {
    /// Immagine finale (ripulita secondo l'intensità impostata, se il denoiser è attivo).
    Final,
    Raw,
    Albedo,
    Normals,
}

impl RenderView {
    pub fn label(self) -> &'static str {
        match self {
            RenderView::Final => tr("Finale"),
            RenderView::Raw => tr("Grezza"),
            RenderView::Albedo => tr("Albedo"),
            RenderView::Normals => tr("Normali"),
        }
    }
}

pub struct ForgeApp {
    pub scene: Scene,
    pub meta: ProjectMeta,
    pub settings: RenderSettings,
    pub orbit: OrbitController,
    pub selection: BTreeSet<usize>,
    pub selected_light: Option<usize>,
    /// Nebbia selezionata (il gizmo sposta il suo emettitore).
    pub selected_fog: Option<usize>,
    /// Il gizmo della nebbia sposta la zona simulata invece dell'emettitore.
    pub fog_gizmo_zone: bool,
    pub selected_material: MaterialId,
    pub view: View,
    pub tab: InspectorTab,
    pub auto_restart: bool,
    pub import_opts: ImportOptions,
    pub unsaved: bool,
    /// Comando in attesa di conferma perché butterebbe via modifiche non salvate.
    pub confirm_discard: Option<crate::commands::Cmd>,
    /// L'utente ha già confermato l'uscita: la chiusura della finestra non si ferma più.
    pub exit_confirmed: bool,
    /// Avviso breve in basso nella finestra (testo, quando è comparso).
    pub toast: Option<(String, std::time::Instant)>,
    /// Finestra del manuale (Guida → Manuale, F1).
    pub manual: crate::ui::manual::ManualUi,
    pub error: Option<String>,
    pub info: Option<String>,
    pub rule_pattern: String,
    pub collapsed: BTreeSet<usize>,
    pub custom_samples: u32,
    pub preview: crate::ui::central::MaterialPreview,
    pub pending: Option<Pending>,
    pub gpu: Option<Result<forge_render_gpu::SharedGpu, String>>,
    gpu_warmup: Option<Receiver<Result<forge_render_gpu::SharedGpu, String>>>,
    pub import_queue: Vec<PathBuf>,
    pub show_import_options: bool,

    /// Revisione della geometria (import, visibilità): invalida la BVH.
    pub geometry_rev: u64,
    /// Revisione di materiali, luci, ambiente, camera: invalida l'accumulo.
    pub shading_rev: u64,
    /// Solo la camera del render è cambiata: si riusa la scena compilata (e quella in VRAM).
    camera_rev: u64,
    /// Ultima scena compilata per il render, con le revisioni da cui è nata.
    last_render_scene: Option<(u64, u64, Arc<RenderScene>)>,
    /// Revisione di materiali e assegnazioni (colori della viewport).
    pub appearance_rev: u64,
    /// Componente da selezionare all'avvio (anche dopo il caricamento del progetto).
    startup_select: Option<String>,
    startup_light: Option<String>,
    startup_fog: Option<String>,
    startup_aim: Option<String>,
    /// Impronta dei parametri di displacement: se cambia va ricostruita la geometria.
    displacement_sig: u64,

    geometry: Option<(u64, Arc<RenderGeometry>)>,
    pending_geometry: Option<PendingGeometry>,
    /// Calcolo delle nebbie in corso (in background); al termine parte il render.
    pending_fog: Option<Receiver<()>>,
    job: Option<JobInfo>,
    pub session: RenderSession,

    pub viewport: Option<ViewportRenderer>,
    viewport_uploaded: Option<(u64, u64, BTreeSet<usize>)>,
    pub render_texture: Option<egui::TextureHandle>,
    render_texture_key: Option<(u64, ToneMapping, u32, u32, RenderView)>,
    /// Cosa mostrare del render: finale, grezzo o i buffer guida del denoiser.
    pub render_view: RenderView,
    /// Open Image Denoise caricato (versione) o motivo dell'assenza.
    pub denoiser_info: Result<String, String>,
    denoise_sent: Option<forge_render_core::DenoiseSettings>,
    pub gizmo: crate::ui::gizmo::Gizmo,
    pub library: crate::ui::library::LibraryUi,
    /// Impostazioni dell'app (menu Impostazioni) e finestra aperta.
    pub prefs: crate::prefs::Preferences,
    pub prefs_open: bool,
    /// Errore di lettura o salvataggio delle impostazioni, mostrato nella finestra.
    pub prefs_error: Option<String>,
    /// Lingue disponibili (comprese e file `lang/Lan_*.json`) e file di lingua non validi.
    pub languages: Vec<std::sync::Arc<crate::i18n::Language>>,
    pub language_errors: Vec<String>,
    /// Schermata di benvenuto aperta e sua immagine.
    pub splash_open: bool,
    pub splash_texture: Option<egui::TextureHandle>,
    pub logo_texture: Option<egui::TextureHandle>,
    /// Modalità Semplice: aspetto e qualità scelti, valori prima dell'ultimo preset.
    pub simple_look: crate::presets::Look,
    pub simple_quality: crate::presets::Quality,
    pub preset_backup: Option<crate::presets::PresetBackup>,
    /// Esito dell'ultima azione della modalità Semplice (mostrato nel pannello, non in finestra).
    pub simple_msg: Option<String>,
    /// "Pilota": la camera di rendering attiva segue la vista della viewport.
    pub camera_follow: bool,
    /// Animazione della vista avviata dal navigatore degli assi.
    pub nav_anim: Option<crate::ui::navigator::NavAnim>,
    /// Il prossimo clic nella viewport sceglie la distanza di messa a fuoco.
    pub focus_pick: bool,
    /// In attesa del clic sull'oggetto da inquadrare ("Punta su oggetto").
    pub aim_pick: bool,
    /// Oggetto a cui la camera è agganciata: si orbita attorno a lui e resta a fuoco.
    pub aim_lock: Option<forge_scene::ComponentId>,
    /// Centro dell'oggetto agganciato nel suo sistema locale (segue i suoi spostamenti).
    aim_local: forge_math::Vec3,
    /// Annulla/ripeti delle trasformazioni: stato precedente dei componenti toccati
    /// (con il numero di nodi della scena, per scartare voci non più valide).
    undo_stack: Vec<(usize, Vec<(usize, NodeState)>)>,
    redo_stack: Vec<(usize, Vec<(usize, NodeState)>)>,

    pub logs: LogBuffer,
    pub system: SystemInfo,
    pub monitor: ResourceMonitor,
    pub safe_mode: bool,
    startup: StartupOptions,
    screenshot_requested: bool,
    screenshot_done: Arc<std::sync::atomic::AtomicBool>,
    frames: u64,
    /// "Controllo IA" acceso: Claude può comandare la finestra (`docs/MCP.md`).
    pub mcp: Option<crate::mcp::Server>,
    /// Modalità Semplice: la GPU è già stata proposta come dispositivo predefinito (una volta
    /// sola, poi vale la scelta dell'utente).
    pub simple_gpu_checked: bool,
    /// Ultimo render avviato dall'assistente (salvato appena finisce).
    pub mcp_render: Option<crate::mcp::tools::RenderJob>,
}

impl ForgeApp {
    pub fn new(cc: &eframe::CreationContext<'_>, logs: LogBuffer, system: SystemInfo, startup: StartupOptions) -> Self {
        crate::ui::style::apply(&cc.egui_ctx);
        crate::ui::icons::install(&cc.egui_ctx);
        let viewport = cc.wgpu_render_state.as_ref().map(ViewportRenderer::new);
        if viewport.is_none() {
            tracing::warn!("modalità sicura: viewport 3D GPU non disponibile, il rendering CPU resta utilizzabile");
        }
        // Scena di partenza: oggetti di prova sul fondale fotografico con le luci da studio.
        let scene = crate::presets::studio_demo_scene();
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
            selected_fog: None,
            fog_gizmo_zone: false,
            selected_material: MaterialId::new("default"),
            view: View::Viewport,
            tab: InspectorTab::Materials,
            auto_restart: true,
            import_opts: ImportOptions::default(),
            unsaved: false,
            confirm_discard: None,
            exit_confirmed: false,
            toast: None,
            manual: Default::default(),
            error: None,
            info: None,
            rule_pattern: String::new(),
            collapsed: BTreeSet::new(),
            custom_samples: 2048,
            preview: Default::default(),
            pending: None,
            gpu: None,
            gpu_warmup: None,
            import_queue: Vec::new(),
            show_import_options: false,
            geometry_rev: 1,
            shading_rev: 1,
            camera_rev: 0,
            last_render_scene: None,
            appearance_rev: 1,
            displacement_sig: 0,
            startup_select: None,
            startup_light: None,
            startup_fog: None,
            startup_aim: None,
            geometry: None,
            pending_geometry: None,
            pending_fog: None,
            job: None,
            session: RenderSession::new(),
            viewport,
            viewport_uploaded: None,
            render_texture: None,
            render_texture_key: None,
            render_view: RenderView::Final,
            denoiser_info: Err(String::new()),
            denoise_sent: None,
            gizmo: Default::default(),
            library: Default::default(),
            prefs: Default::default(),
            prefs_open: false,
            prefs_error: None,
            languages: Vec::new(),
            language_errors: Vec::new(),
            splash_open: false,
            splash_texture: None,
            logo_texture: None,
            simple_look: crate::presets::Look::Light,
            simple_quality: crate::presets::Quality::Balanced,
            preset_backup: None,
            simple_msg: None,
            camera_follow: false,
            nav_anim: None,
            focus_pick: false,
            aim_pick: false,
            aim_lock: None,
            aim_local: forge_math::Vec3::ZERO,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            logs,
            system,
            monitor: ResourceMonitor::new(),
            safe_mode: startup.safe_mode,
            startup: startup.clone(),
            screenshot_requested: false,
            screenshot_done: Default::default(),
            frames: 0,
            mcp: None,
            mcp_render: None,
            simple_gpu_checked: false,
        };
        // Prepara il backend GPU in background (creazione dispositivo e compilazione
        // dello shader da parte del driver), così è pronto quando serve.
        if !app.safe_mode {
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::Builder::new()
                .name("forge-gpu-init".into())
                .spawn(move || {
                    let _ = tx.send(forge_render_gpu::SharedGpu::new().map_err(|e| format!("{e:#}")));
                })
                .ok();
            app.gpu_warmup = Some(rx);
        }
        // Impostazioni: libreria, import, valori dei nuovi progetti.
        let (prefs, err) = crate::prefs::Preferences::load_from(&crate::prefs::file_path());
        if let Some(e) = &err {
            tracing::warn!("{e}");
        }
        app.prefs_error = err;
        (app.languages, app.language_errors) = crate::i18n::available();
        for e in &app.language_errors {
            tracing::warn!("file di lingua non valido: {e}");
        }
        app.apply_prefs(prefs, true);
        // Avviato già comandato dall'assistente (prove automatiche): niente schermata davanti.
        app.splash_open = app.prefs.show_splash && !startup.mcp;
        if startup.mcp {
            crate::mcp::set_enabled(&mut app, &cc.egui_ctx, true);
        }
        // Cartelle della libreria (HDRI, materiali, texture).
        match forge_library::paths::ensure() {
            Ok(root) => tracing::info!("libreria: {}", root.display()),
            Err(e) => tracing::warn!("cartelle della libreria non create: {e}"),
        }
        // Denoiser: la libreria OIDN è opzionale (accanto all'eseguibile).
        match forge_denoise::OidnDenoiser::load() {
            Ok(d) => {
                app.denoiser_info = Ok(tf("Open Image Denoise {}", &[d.version().to_string()]));
                app.session.set_denoiser(Some(Box::new(d)));
            }
            Err(e) => {
                tracing::warn!("riduzione del rumore non disponibile: {e:#}");
                app.denoiser_info = Err(format!("{e:#}"));
            }
        }
        if let Some(p) = &startup.open {
            app.open_any(p);
        }
        app.startup_select = startup.select.clone();
        app.startup_light = startup.light.clone();
        app.startup_fog = startup.fog.clone();
        app.startup_aim = startup.aim.clone();
        app.gizmo.mode = match startup.gizmo.as_deref() {
            Some("rotate") => crate::ui::gizmo::GizmoMode::Rotate,
            Some("scale") => crate::ui::gizmo::GizmoMode::Scale,
            _ => crate::ui::gizmo::GizmoMode::Move,
        };
        if let Some(mode) = &startup.camera_test {
            app.add_render_camera(Some(tr("Frontale").into()));
            if mode == "dof" {
                let c = &mut app.scene.render_cameras[0].camera;
                c.set_focal_mm(85.0);
                c.lens.dof = true;
                c.lens.f_stop = 2.8;
                c.lens.focus_distance = (c.target - c.position).length();
            }
            if mode != "attraverso" {
                app.orbit.yaw += 1.2;
                app.orbit.pitch += 0.35;
                app.orbit.distance *= 2.6;
                app.camera_changed();
            }
            app.tab = InspectorTab::Camera;
        }
        app.library.open = startup.library;
        app.prefs_open = startup.settings;
        if let Some(m) = startup.ui_mode {
            app.prefs.ui_mode = m;
        }
        if startup.iso {
            app.orbit.lens.ortho = true;
            app.orbit.yaw = std::f32::consts::FRAC_PI_4;
            app.orbit.pitch = crate::ui::navigator::ISO_PITCH;
            app.camera_changed();
        }
        if let Some(l) = startup.prepare {
            app.simple_look = l;
            app.simple_prepare_scene();
        }
        if let Some(t) = &startup.tab {
            app.tab = match t.as_str() {
                "oggetto" => InspectorTab::Object,
                "luci" => InspectorTab::Lights,
                "ambiente" => InspectorTab::Environment,
                "effetti" => InspectorTab::Effects,
                "camera" => InspectorTab::Camera,
                "render" => InspectorTab::Render,
                _ => InspectorTab::Materials,
            };
        }
        if startup.auto_render {
            app.tab = InspectorTab::Render;
            app.start_render();
        }
        app
    }

    // ------------------------------------------------------------------
    // Revisioni e invalidazione
    // ------------------------------------------------------------------

    /// Cambiata solo la camera del render (posizione, obiettivo, camera attiva):
    /// il render riparte riusando la scena già compilata e già in memoria video.
    pub fn render_camera_changed(&mut self) {
        self.camera_rev += 1;
        self.unsaved = true;
    }

    /// Materiali, luci, ambiente o geometria modificati.
    pub fn shading_changed(&mut self) {
        self.shading_rev += 1;
        self.unsaved = true;
    }

    /// Materiali o assegnazioni modificati (cambiano anche i colori della viewport).
    pub fn appearance_changed(&mut self) {
        self.appearance_rev += 1;
        // Il displacement sposta i triangoli: cambiarlo richiede una nuova geometria.
        let sig = self.displacement_signature();
        if sig != self.displacement_sig {
            self.displacement_sig = sig;
            self.geometry_rev += 1;
        }
        self.shading_changed();
    }

    /// Impronta di componenti e parametri con displacement (0 = nessuno).
    fn displacement_signature(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        let mut any = false;
        for i in 0..self.scene.nodes.len() {
            let Some(m) = self.scene.materials.get(self.scene.resolve_material(i)) else {
                continue;
            };
            if !m.has_displacement() || self.scene.nodes[i].mesh.is_none() {
                continue;
            }
            any = true;
            i.hash(&mut h);
            m.height_texture.hash(&mut h);
            for v in [
                m.displacement_mm,
                m.displacement_mid,
                m.mapping.scale,
                m.mapping.rotation_deg,
                m.mapping.offset[0],
                m.mapping.offset[1],
            ] {
                v.to_bits().hash(&mut h);
            }
            (m.mapping.projection == forge_scene::Projection::Uv).hash(&mut h);
        }
        if any {
            h.finish() | 1
        } else {
            0
        }
    }

    /// Geometria, visibilità o struttura modificate.
    pub fn geometry_changed(&mut self) {
        self.geometry_rev += 1;
        self.appearance_rev += 1;
        self.shading_rev += 1;
        self.unsaved = true;
    }

    fn node_state(&self, i: usize) -> NodeState {
        let n = &self.scene.nodes[i];
        NodeState {
            user: n.user,
            visible: n.visible,
            removed: n.removed,
        }
    }

    /// Registra lo stato dei componenti indicati prima di una modifica (annulla).
    pub fn push_undo(&mut self, nodes: &[usize]) {
        let before = nodes.iter().map(|&i| (i, self.node_state(i))).collect();
        self.undo_stack.push((self.scene.nodes.len(), before));
        if self.undo_stack.len() > 200 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    /// Scarta l'ultima voce (modifica annullata con Esc prima di essere applicata).
    pub fn discard_last_undo(&mut self) {
        self.undo_stack.pop();
    }

    /// H: nasconde la selezione. Shift+H: nasconde tutto il resto. Alt+H: mostra tutto.
    pub fn hide_selection(&mut self, others: bool) {
        if self.selection.is_empty() {
            return;
        }
        let targets: Vec<usize> = if others {
            // Restano visibili i selezionati, i loro antenati e i loro discendenti.
            let mut keep = vec![false; self.scene.nodes.len()];
            for &s in &self.selection {
                let mut p = Some(s);
                while let Some(k) = p {
                    keep[k] = true;
                    p = self.scene.nodes[k].parent;
                }
                let mut stack = vec![s];
                while let Some(k) = stack.pop() {
                    keep[k] = true;
                    stack.extend(self.scene.nodes[k].children.iter().copied());
                }
            }
            (0..self.scene.nodes.len())
                .filter(|&i| !keep[i] && self.scene.nodes[i].visible)
                .collect()
        } else {
            self.selection.iter().copied().collect()
        };
        if targets.is_empty() {
            return;
        }
        self.push_undo(&targets);
        for &i in &targets {
            self.scene.nodes[i].visible = false;
        }
        if !others {
            self.selection.clear();
        }
        self.geometry_changed();
    }

    /// Ctrl+D: duplica i componenti selezionati (identici: materiale, posizione e impostazioni)
    /// e seleziona le copie con il gizmo di spostamento pronto, come in Blender.
    pub fn duplicate_selection(&mut self) {
        let sel = crate::ui::gizmo::top_selected(self);
        if sel.is_empty() {
            return;
        }
        let copies: Vec<usize> = sel.into_iter().map(|i| self.scene.duplicate(i)).collect();
        self.selection = copies.into_iter().collect();
        self.gizmo.mode = crate::ui::gizmo::GizmoMode::Move;
        self.geometry_changed();
        self.show_selection_tab();
    }

    /// Ingombro di riferimento per posizionare luci e oggetti nuovi, così compaiono vicino a
    /// ciò che si sta guardando: i pezzi selezionati, altrimenti il prodotto (senza pavimento,
    /// fondale e pareti), altrimenti tutta la scena. Prima si usava sempre tutta la scena: con un
    /// pavimento grande o un ambiente importato le luci finivano a metri di distanza.
    pub fn anchor_bounds(&self) -> forge_math::Aabb {
        let mut b = forge_math::Aabb::EMPTY;
        for &s in &self.selection {
            if s < self.scene.nodes.len() {
                b.grow(&self.scene.bounds_of(s));
            }
        }
        if b.is_empty() {
            b = crate::presets::product_bounds(&self.scene);
        }
        if b.is_empty() {
            b = self.scene.bounds();
        }
        b
    }

    /// Centro e raggio di [`Self::anchor_bounds`] (raggio almeno 5 cm; scena vuota: origine, 1 m).
    pub fn anchor(&self) -> (forge_math::Vec3, f32) {
        let b = self.anchor_bounds();
        if b.is_empty() {
            (forge_math::Vec3::ZERO, 1.0)
        } else {
            (b.center(), b.radius().max(0.05))
        }
    }

    /// Crea un oggetto semplice dimensionato e posizionato rispetto al riferimento
    /// ([`Self::anchor_bounds`]): pavimento e fondale sotto/dietro, le forme di lato sulla base.
    /// Aggiunge una luce vicino alla selezione o al prodotto e la seleziona (pannello Luci e
    /// menu Oggetti → Luce).
    pub fn add_light(&mut self, kind: NewLight) {
        use forge_math::Vec3;
        use forge_scene::{Light, LightKind};
        let (center, radius) = self.anchor();
        // Portale sulla finestra selezionata: rettangolo grande come il pezzo, sul suo lato più
        // sottile, rivolto verso il centro della scena (l'interno della stanza).
        let portal_from_selection = {
            let mut b = forge_math::Aabb::EMPTY;
            for &i in &self.selection {
                b.grow(&self.scene.exact_bounds_of(i));
            }
            (!b.is_empty()).then(|| {
                let s = b.max - b.min;
                let c = b.center();
                let axis = if s.x <= s.y && s.x <= s.z {
                    0
                } else if s.y <= s.z {
                    1
                } else {
                    2
                };
                let mut n = Vec3::ZERO;
                n[axis] = if (center - c)[axis] >= 0.0 { 1.0 } else { -1.0 };
                // Larghezza e altezza come le disegna il renderer (u orizzontale, v verticale).
                let hint = if n.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
                let u = hint.cross(n).normalize();
                let v = n.cross(u);
                let ext = |d: Vec3| (d.x.abs() * s.x + d.y.abs() * s.y + d.z.abs() * s.z).max(0.01);
                Light::portal(tr("Portale finestra"), c, c + n, ext(u), ext(v))
            })
        };
        let at = center + Vec3::new(1.0, 2.0, 1.5) * radius;
        let light = match kind {
            NewLight::Area => Light::area(tr("Luce area"), at, center, radius, radius, 5.0),
            NewLight::Point => Light::point(tr("Luce puntiforme"), at, 20.0 * radius * radius),
            NewLight::Spot => Light {
                kind: LightKind::Spot {
                    inner_angle_deg: 25.0,
                    outer_angle_deg: 40.0,
                },
                target: center,
                ..Light::point(tr("Spot"), at, 40.0 * radius * radius)
            },
            NewLight::Directional => Light {
                kind: LightKind::Directional,
                target: center,
                ..Light::point(tr("Sole"), at, 3.0)
            },
            NewLight::Portal => portal_from_selection.unwrap_or_else(|| Light::portal(tr("Portale finestra"), at, center, radius, radius)),
        };
        self.scene.lights.push(light);
        self.selected_light = Some(self.scene.lights.len() - 1);
        self.shading_changed();
    }

    pub fn add_object(&mut self, kind: AddKind) {
        use forge_math::{Mat4, Vec3};
        use forge_scene::AddedKind as K;
        let b = self.anchor_bounds();
        let (c, ground, e, back, right) = if b.is_empty() {
            (Vec3::ZERO, 0.0, 1.0, -0.5, 0.5)
        } else {
            let s = b.max - b.min;
            let c = (b.min + b.max) * 0.5;
            (c, b.min.y, s.x.max(s.z).max(0.5), b.min.z, b.max.x)
        };
        let at = |x: f32, z: f32| Mat4::from_translation(Vec3::new(x, ground, z));
        let side = right + 0.4 * e;
        let r = 0.15 * e;
        let (kind, placement) = match kind {
            AddKind::Floor => (K::Floor { size: (4.0 * e).max(4.0) }, at(c.x, c.z)),
            // Stessa regola della "Prepara scena" della modalità Semplice.
            AddKind::Backdrop => crate::presets::backdrop_for(&b, crate::presets::BackdropStyle::Curved),
            AddKind::StudioSweep => crate::presets::backdrop_for(&b, crate::presets::BackdropStyle::Sweep),
            AddKind::Wall => (
                K::Wall {
                    width: (3.0 * e).max(3.0),
                    height: (1.5 * e).max(2.0),
                },
                at(c.x, back - 0.3 * e),
            ),
            AddKind::Box => (
                K::Box {
                    size: Vec3::splat(2.0 * r),
                },
                at(side, c.z),
            ),
            AddKind::Sphere => (K::Sphere { radius: r }, at(side, c.z)),
            AddKind::Cylinder => (
                K::Cylinder {
                    radius: r,
                    height: 2.0 * r,
                },
                at(side, c.z),
            ),
            AddKind::Cup => (
                K::Cup {
                    radius: 0.6 * r,
                    height: 1.4 * r,
                },
                at(side, c.z),
            ),
        };
        let i = self.scene.add_object(kind, placement);
        self.selection = [i].into_iter().collect();
        self.geometry_changed();
        self.show_selection_tab();
    }

    /// Canc: elimina dal progetto i componenti selezionati. Copie e oggetti aggiunti si
    /// tolgono dalla scena; i componenti dei file importati restano nel file sorgente (che non
    /// viene toccato) ma vengono segnati come eliminati: spariscono da vista, render e albero,
    /// e si ripristinano con Ctrl+Z o dalla voce "Eliminati" dell'albero.
    pub fn delete_selected_copies(&mut self) {
        let ids: Vec<forge_scene::ComponentId> = crate::ui::gizmo::top_selected(self)
            .into_iter()
            .map(|i| self.scene.nodes[i].id.clone())
            .collect();
        let mut deleted = 0;
        let mut original_ids = Vec::new();
        for id in ids {
            match self.scene.find(&id) {
                Some(i) if self.scene.delete_copy(i) => deleted += 1,
                Some(i) if self.scene.delete_added(i) => deleted += 1,
                Some(_) => original_ids.push(id),
                None => {}
            }
        }
        // Gli indici si cercano dopo: eliminare copie e oggetti aggiunti li sposta.
        let originals: Vec<usize> = original_ids.iter().filter_map(|id| self.scene.find(id)).collect();
        if !originals.is_empty() {
            self.push_undo(&originals);
            for &i in &originals {
                self.scene.nodes[i].removed = true;
            }
            self.info = Some(tf(
                "{} componenti eliminati dal progetto (il file importato non viene modificato). \
                 Ctrl+Z o \"Eliminati\" in fondo all'albero per ripristinarli.",
                &[format!("{}", originals.len())],
            ));
        }
        if deleted > 0 || !originals.is_empty() {
            self.selection.clear();
            self.geometry_changed();
        }
    }

    /// Componenti dei file importati eliminati dal progetto.
    pub fn removed_nodes(&self) -> Vec<usize> {
        (0..self.scene.nodes.len()).filter(|&i| self.scene.nodes[i].removed).collect()
    }

    /// Riporta nel progetto i componenti eliminati indicati (annullabile).
    pub fn restore_removed(&mut self, nodes: &[usize]) {
        if nodes.is_empty() {
            return;
        }
        self.push_undo(nodes);
        for &i in nodes {
            self.scene.nodes[i].removed = false;
        }
        self.geometry_changed();
    }

    pub fn show_all(&mut self) {
        let hidden: Vec<usize> = (0..self.scene.nodes.len())
            .filter(|&i| !self.scene.nodes[i].visible && !self.scene.nodes[i].removed)
            .collect();
        if hidden.is_empty() {
            return;
        }
        self.push_undo(&hidden);
        for &i in &hidden {
            self.scene.nodes[i].visible = true;
        }
        self.geometry_changed();
    }

    pub fn can_undo_redo(&self) -> (bool, bool) {
        (!self.undo_stack.is_empty(), !self.redo_stack.is_empty())
    }

    fn swap_transforms(&mut self, from_undo: bool) {
        let n = self.scene.nodes.len();
        let (src, dst) = if from_undo {
            (&mut self.undo_stack, &mut self.redo_stack)
        } else {
            (&mut self.redo_stack, &mut self.undo_stack)
        };
        while let Some((count, entry)) = src.pop() {
            if count != n {
                continue; // la struttura della scena è cambiata: voce non più applicabile
            }
            let current = entry
                .iter()
                .map(|(i, _)| {
                    let n = &self.scene.nodes[*i];
                    (
                        *i,
                        NodeState {
                            user: n.user,
                            visible: n.visible,
                            removed: n.removed,
                        },
                    )
                })
                .collect();
            for (i, st) in entry {
                self.scene.nodes[i].user = st.user;
                self.scene.nodes[i].visible = st.visible;
                self.scene.nodes[i].removed = st.removed;
            }
            dst.push((n, current));
            self.geometry_changed();
            return;
        }
    }

    pub fn undo(&mut self) {
        self.swap_transforms(true);
    }

    pub fn redo(&mut self) {
        self.swap_transforms(false);
    }

    /// Camera spostata dall'utente (orbita/pan/zoom).
    pub fn camera_changed(&mut self) {
        if self.aim_lock.is_some() {
            // Agganciata a un oggetto: il fuoco resta sul suo centro mentre si gira o si zooma.
            self.orbit.lens.focus_distance = self.orbit.distance.max(0.01);
        }
        self.scene.camera = self.orbit.camera();
        match self.scene.active_camera {
            // La vista si muove liberamente: l'inquadratura del render non cambia.
            Some(_) if !self.camera_follow => {}
            Some(i) => {
                if let Some(c) = self.scene.render_cameras.get_mut(i) {
                    c.camera = self.scene.camera.clone();
                }
                self.camera_rev += 1;
                self.unsaved = true;
            }
            None => self.camera_rev += 1,
        }
    }

    /// Aggiunge una camera di rendering dalla vista attuale e la rende attiva.
    pub fn add_render_camera(&mut self, name: Option<String>) {
        let n = self.scene.render_cameras.len() + 1;
        self.scene.render_cameras.push(forge_scene::RenderCamera {
            name: name.unwrap_or_else(|| tf("Camera {n}", &[format!("{}", n)])),
            camera: self.scene.camera.clone(),
        });
        self.set_active_camera(Some(self.scene.render_cameras.len() - 1));
    }

    pub fn set_active_camera(&mut self, idx: Option<usize>) {
        self.scene.active_camera = idx.filter(|&i| i < self.scene.render_cameras.len());
        if self.scene.active_camera.is_none() {
            self.camera_follow = false;
        }
        self.render_camera_changed();
    }

    /// Aggancia la camera all'oggetto `node` (o al componente che lo contiene): lo inquadra,
    /// lo mette a fuoco e da lì in poi si orbita attorno a lui. Con una camera di rendering
    /// attiva, la camera segue la vista.
    pub fn aim_at(&mut self, node: usize) {
        let mut idx = node;
        // Le mesh importate stanno sotto un nodo col nome del pezzo: si aggancia quello.
        if let Some(p) = self.scene.nodes[idx].parent {
            if self.scene.nodes[idx].name.ends_with("(mesh)") {
                idx = p;
            }
        }
        let b = self.scene.exact_bounds_of(idx);
        if b.is_empty() {
            return;
        }
        self.aim_lock = Some(self.scene.nodes[idx].id.clone());
        self.aim_local = self.scene.world_transform(idx).inverse().transform_point3(b.center());
        self.orbit.frame(&b);
        self.orbit.lens.dof = true;
        if self.scene.active_camera.is_some() {
            self.camera_follow = true;
        }
        self.camera_changed();
    }

    /// Sgancia la camera dall'oggetto.
    pub fn release_aim(&mut self) {
        self.aim_lock = None;
    }

    /// Se l'oggetto agganciato si è spostato, la camera lo segue (una volta per frame).
    pub fn follow_aim(&mut self) {
        let Some(id) = &self.aim_lock else { return };
        let Some(i) = self.scene.find(id) else {
            self.aim_lock = None;
            return;
        };
        let c = self.scene.world_transform(i).transform_point3(self.aim_local);
        if (c - self.orbit.target).length() > 1e-5 * self.orbit.distance.max(1.0) {
            self.orbit.target = c;
            self.camera_changed();
        }
    }

    /// Porta la vista della viewport sull'inquadratura della camera di rendering.
    pub fn look_through(&mut self, idx: usize) {
        if let Some(c) = self.scene.render_cameras.get(idx) {
            self.scene.camera = c.camera.clone();
            self.sync_orbit_from_camera();
        }
    }

    /// Mette a fuoco la camera che si sta regolando (quella attiva, o la vista) sul punto `p`.
    pub fn set_focus_point(&mut self, p: forge_math::Vec3) {
        let through = self.viewing_through_active();
        let cam = match self.scene.active_camera {
            Some(i) => &mut self.scene.render_cameras[i].camera,
            None => &mut self.scene.camera,
        };
        cam.lens.focus_distance = (p - cam.position).dot(cam.forward()).max(0.01);
        cam.lens.dof = true;
        let lens = cam.lens.clone();
        if through || self.scene.active_camera.is_none() {
            self.scene.camera.lens = lens.clone();
            self.orbit.lens = lens;
        }
        self.shading_changed();
    }

    /// Vero se la viewport mostra esattamente l'inquadratura della camera attiva.
    pub fn viewing_through_active(&self) -> bool {
        self.scene.active_camera.is_some() && self.scene.render_camera() == &self.scene.camera
    }

    pub fn sync_orbit_from_camera(&mut self) {
        self.orbit = OrbitController::from_camera(&self.scene.camera);
    }

    /// Geometria compilata se è aggiornata (senza costruirla).
    pub fn compiled_geometry(&self) -> Option<Arc<RenderGeometry>> {
        self.current_geometry()
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
        self.pending_geometry.is_some() || self.pending_fog.is_some()
    }

    /// Nebbie in calcolo (per l'indicazione nell'interfaccia).
    pub fn is_baking_fog(&self) -> bool {
        self.pending_fog.is_some()
    }

    fn poll_fog(&mut self) {
        let Some(rx) = &self.pending_fog else { return };
        match rx.try_recv() {
            Ok(()) => {
                self.pending_fog = None;
                self.start_render();
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(_) => {
                self.pending_fog = None;
                self.error = Some(tr("calcolo della nebbia fallito").into());
            }
        }
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
                self.error = Some(tr("compilazione della geometria fallita").into());
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
        // Nebbie da calcolare: in background, poi si riparte da qui (le trova in cache).
        if !forge_render_core::fog::fogs_ready(&self.scene, &geometry) {
            if self.pending_fog.is_none() {
                let (tx, rx) = std::sync::mpsc::channel();
                let scene = self.scene.clone();
                let geo = geometry.clone();
                std::thread::Builder::new()
                    .name("forge-fog".into())
                    .spawn(move || {
                        forge_render_core::fog::bake_fogs(&scene, &geo);
                        let _ = tx.send(());
                    })
                    .expect("thread della nebbia");
                self.pending_fog = Some(rx);
            }
            self.view = View::Render;
            return;
        }
        let backend: Box<dyn forge_render_core::RenderBackend> = match self.settings.backend {
            forge_render_core::BackendKind::Gpu => match self.gpu_backend() {
                Ok(g) => Box::new(g),
                Err(e) => {
                    self.info = Some(tf(
                        "GPU non disponibile, il rendering prosegue sulla CPU.\n\n{e}",
                        std::slice::from_ref(&e),
                    ));
                    self.settings.backend = forge_render_core::BackendKind::Cpu;
                    return self.start_render();
                }
            },
            forge_render_core::BackendKind::Cpu => match CpuBackend::from_settings(&self.settings) {
                Ok(b) => Box::new(b),
                Err(e) => {
                    self.error = Some(tf("impossibile creare il backend CPU: {e}", &[format!("{}", e)]));
                    return;
                }
            },
        };
        let rs = match &self.last_render_scene {
            // Cambiata solo la camera: niente ricompilazione né nuovo caricamento sulla GPU.
            Some((g, sh, prev)) if *g == self.geometry_rev && *sh == self.shading_rev => {
                Arc::new(prev.with_camera(self.scene.render_camera().clone()))
            }
            _ => Arc::new(RenderScene::compile_with_geometry(&self.scene, geometry)),
        };
        self.last_render_scene = Some((self.geometry_rev, self.shading_rev, rs.clone()));
        self.session.start(rs, self.settings.clone(), backend);
        self.job = Some(JobInfo {
            geometry_rev: self.geometry_rev,
            shading_rev: self.shading_rev,
            camera_rev: self.camera_rev,
            key: self.settings.accumulation_key(),
            threads: self.settings.effective_threads(),
            target: self.settings.target_samples,
            adaptive: (self.settings.noise_threshold, self.settings.adaptive_min_samples),
            backend: self.settings.backend,
        });
        self.view = View::Render;
    }

    /// GPU utilizzabile? Senza aspettare: `None` finché la preparazione in background non è
    /// finita (o se non è mai partita, come in modalità sicura).
    pub fn gpu_ready(&mut self) -> Option<bool> {
        if self.gpu.is_none() {
            if let Some(Ok(r)) = self.gpu_warmup.as_ref().map(|rx| rx.try_recv()) {
                self.gpu = Some(r);
                self.gpu_warmup = None;
            }
        }
        self.gpu.as_ref().map(Result::is_ok)
    }

    /// Dispositivo GPU condiviso, creato al primo utilizzo.
    pub fn gpu_backend(&mut self) -> Result<forge_render_gpu::SharedGpu, String> {
        if self.gpu.is_none() {
            if let Some(rx) = self.gpu_warmup.take() {
                // Attende la preparazione in corso invece di crearne un secondo.
                if let Ok(r) = rx.recv() {
                    self.gpu = Some(r);
                    return self.gpu.clone().expect("gpu");
                }
            }
            self.gpu = Some(forge_render_gpu::SharedGpu::new().map_err(|e| format!("{e:#}")));
        }
        self.gpu.clone().expect("gpu")
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
                    || j.camera_rev != self.camera_rev
                    || j.key != self.settings.accumulation_key()
                    || j.threads != self.settings.effective_threads()
                    || j.backend != self.settings.backend
            }
        }
    }

    fn update_render_job(&mut self) {
        // Le impostazioni del denoiser non invalidano l'accumulo: si inviano alla sessione.
        if self.denoise_sent.as_ref() != Some(&self.settings.denoise) {
            self.session.set_denoise(self.settings.denoise.clone());
            self.denoise_sent = Some(self.settings.denoise.clone());
        }
        let Some(job) = &mut self.job else { return };
        if job.target != self.settings.target_samples {
            job.target = self.settings.target_samples;
            self.session.set_target(job.target);
        }
        let adaptive = (self.settings.noise_threshold, self.settings.adaptive_min_samples);
        if job.adaptive != adaptive {
            job.adaptive = adaptive;
            self.session.set_adaptive(adaptive.0, adaptive.1);
        }
        if should_auto_restart(self.auto_restart, self.render_is_stale(), self.is_preparing()) {
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
        let strength = self.settings.denoise.strength;
        let key = (
            version,
            self.settings.tone_mapping,
            self.settings.exposure_ev.to_bits(),
            strength.to_bits(),
            self.render_view,
        );
        if self.render_texture_key != Some(key) {
            if let Some(frame) = self.session.latest_frame() {
                let rgba = match (self.render_view, &frame.guides) {
                    (RenderView::Raw, _) => {
                        forge_render_core::tonemap_to_rgba8(&frame.linear, self.settings.tone_mapping, self.settings.exposure_ev)
                    }
                    // Buffer guida: valori già in [0, 1] (le normali rimappate da [-1, 1]).
                    (RenderView::Albedo, Some(g)) => forge_render_core::tonemap_to_rgba8(&g.albedo, ToneMapping::Standard, 0.0),
                    (RenderView::Normals, Some(g)) => {
                        let n: Vec<[f32; 3]> = g.normal.iter().map(|v| v.map(|c| c * 0.5 + 0.5)).collect();
                        forge_render_core::tonemap_to_rgba8(&n, ToneMapping::Standard, 0.0)
                    }
                    _ => {
                        forge_render_core::tonemap_to_rgba8(&frame.display(strength), self.settings.tone_mapping, self.settings.exposure_ev)
                    }
                };
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

    /// "Esporta…": un solo comando, il formato si sceglie nella finestra di salvataggio
    /// (PNG, JPG, TIFF 16 bit, EXR); vale l'estensione del nome scritto.
    pub fn export_dialog(&mut self) {
        use forge_image_io::ExportFormat;
        let Some(frame) = self.session.latest_frame() else {
            self.error = Some(tr("nessuna immagine da esportare: avvia prima un rendering").into());
            return;
        };
        let mut dialog = rfd::FileDialog::new().set_file_name(format!("render_{}spp.png", frame.samples));
        for f in ExportFormat::ALL {
            dialog = dialog.add_filter(tr(export_format_label(f)), f.extensions());
        }
        let Some(path) = dialog.save_file() else {
            return;
        };
        let (path, format) = match ExportFormat::from_path(&path) {
            Some(f) => (path, f),
            None => (with_extension(path, "png"), ExportFormat::Png),
        };
        match forge_image_io::save_frame(
            &path,
            format,
            &frame,
            self.settings.tone_mapping,
            self.settings.exposure_ev,
            self.settings.denoise.strength,
        ) {
            Ok(()) => {
                tracing::info!("immagine esportata: {} ({} campioni)", path.display(), frame.samples);
                self.info = Some(tf(
                    "Immagine esportata ({} campioni):\n{}",
                    &[format!("{}", frame.samples), format!("{}", path.display())],
                ));
            }
            Err(e) => self.error = Some(tf("esportazione fallita: {e:#}", &[format!("{:#}", e)])),
        }
    }

    // ------------------------------------------------------------------
    // Scena e progetto
    // ------------------------------------------------------------------

    pub fn replace_scene(&mut self, scene: Scene, meta: ProjectMeta, settings: Option<RenderSettings>) {
        self.session.cancel();
        self.job = None;
        self.scene = scene;
        self.meta = meta;
        if let Some(s) = settings {
            self.settings = s;
        }
        self.selection.clear();
        self.selected_light = None;
        self.selected_fog = None;
        self.collapsed.clear();
        self.sync_orbit_from_camera();
        self.geometry_changed();
        self.unsaved = false;
        self.view = View::Viewport;
    }

    pub fn new_test_scene(&mut self) {
        let settings = self.prefs.new_project_settings();
        self.replace_scene(
            Scene::test_scene(),
            ProjectMeta {
                test_scene: true,
                ..Default::default()
            },
            Some(settings),
        );
    }

    pub fn new_empty_scene(&mut self) {
        let s = Scene {
            lights: forge_scene::light::preset("studio", forge_math::Vec3::ZERO, 1.0).unwrap_or_default(),
            ..Default::default()
        };
        let settings = self.prefs.new_project_settings();
        self.replace_scene(s, ProjectMeta::default(), Some(settings));
    }

    // --- Modalità Semplice (logica in `presets`) ---

    /// Applica un aspetto (luci, ambiente, visualizzazione, colore del fondale).
    pub fn simple_apply_look(&mut self, look: crate::presets::Look) {
        self.preset_backup = Some(crate::presets::PresetBackup::capture(&self.scene, &self.settings));
        crate::presets::apply_look(&mut self.scene, &mut self.settings, look);
        self.simple_look = look;
        self.selected_light = None;
        self.appearance_changed();
        self.shading_changed();
    }

    /// Applica una qualità (campioni, rimbalzi, denoiser).
    pub fn simple_apply_quality(&mut self, q: crate::presets::Quality) {
        self.preset_backup = Some(crate::presets::PresetBackup::capture(&self.scene, &self.settings));
        q.apply(&mut self.settings);
        self.simple_quality = q;
    }

    /// "Prepara scena prodotto": appoggia, fondale se manca, aspetto, qualità, inquadratura.
    pub fn simple_prepare_scene(&mut self) {
        if crate::presets::product_bounds(&self.scene).is_empty() {
            self.simple_msg = Some(tr("Importa prima un modello: la scena non ha un prodotto da preparare.").into());
            return;
        }
        let backup = crate::presets::prepare_scene(&mut self.scene, &mut self.settings, self.simple_look, self.simple_quality);
        self.preset_backup = Some(backup);
        self.selected_light = None;
        self.selection.clear();
        self.geometry_changed();
        self.frame_product();
        self.simple_msg = Some(tf(
            "Scena pronta: {} · qualità {}. \"Annulla preset\" la riporta com'era.",
            &[self.simple_look.label().to_string(), self.simple_quality.label().to_string()],
        ));
    }

    /// "Adatta la scena al prodotto": fondale e luci rifatti sulle misure attuali del prodotto,
    /// poi inquadratura. Si annulla con "Annulla preset".
    pub fn simple_fit_scene(&mut self) {
        if crate::presets::product_bounds(&self.scene).is_empty() {
            self.simple_msg = Some(tr("Importa prima un modello: non c'è un prodotto a cui adattare la scena.").into());
            return;
        }
        let backup = crate::presets::fit_scene(&mut self.scene, &self.settings);
        self.preset_backup = Some(backup);
        self.selected_light = None;
        self.geometry_changed();
        self.frame_product();
        self.simple_msg =
            Some(tr("Scena adattata al prodotto: fondale, luci e inquadratura. \"Annulla preset\" la riporta com'era.").into());
    }

    /// Ripristina i valori di prima dell'ultimo preset (solo quelli che il preset ha cambiato).
    pub fn simple_undo_preset(&mut self) {
        if let Some(b) = self.preset_backup.take() {
            b.restore(&mut self.scene, &mut self.settings);
            self.selection.clear();
            self.geometry_changed();
            self.simple_msg = Some(tr("Preset annullato: luci, ambiente, qualità e fondale come prima.").into());
        }
    }

    /// Appoggia tutto il prodotto sulla base (annullabile con Ctrl+Z).
    pub fn drop_product_to_base(&mut self) {
        let roots = crate::presets::product_roots(&self.scene);
        if roots.is_empty() {
            return;
        }
        self.push_undo(&roots);
        if crate::presets::drop_product_to_base(&mut self.scene).abs() > 0.0 {
            self.geometry_changed();
        }
    }

    /// Alterna vista prospettica e ortografica (pulsante sotto il navigatore, tasto 5).
    pub fn toggle_ortho(&mut self) {
        self.orbit.lens.ortho = !self.orbit.lens.ortho;
        self.camera_changed();
    }

    /// Inquadra il prodotto (senza pavimento e fondale); muove anche la camera di render attiva.
    pub fn frame_product(&mut self) {
        let b = crate::presets::product_bounds(&self.scene);
        if b.is_empty() {
            let b = self.scene.bounds();
            self.orbit.frame(&b);
            return self.commit_view();
        }
        self.orbit.frame(&b);
        // Formati verticali (4:5, 9:16): conta anche l'apertura orizzontale, più stretta.
        let aspect = self.settings.aspect();
        if aspect < 1.0 {
            let half_v = (self.orbit.vfov_deg.to_radians() * 0.5).min(1.5);
            let half_h = (half_v.tan() * aspect).atan().max(1e-3);
            self.orbit.distance = b.radius().max(1e-4) / half_h.sin() * 1.1;
        }
        self.commit_view();
    }

    /// La vista attuale diventa anche l'inquadratura del render (camera attiva compresa).
    pub fn commit_view(&mut self) {
        self.scene.camera = self.orbit.camera();
        if let Some(c) = self.scene.active_camera.and_then(|i| self.scene.render_cameras.get_mut(i)) {
            c.camera = self.scene.camera.clone();
            self.unsaved = true;
        }
        self.camera_rev += 1;
    }

    /// Adotta nuove impostazioni: le applica e (se non è l'avvio) le salva subito.
    /// `startup`: anche i valori che valgono solo all'avvio (render della scena iniziale).
    pub fn apply_prefs(&mut self, prefs: crate::prefs::Preferences, startup: bool) {
        let prefs = prefs.sanitized();
        let library_changed = startup || prefs.library_dir != self.prefs.library_dir;
        if startup || prefs.language != self.prefs.language {
            crate::i18n::set(self.languages.iter().find(|l| l.code == prefs.language).cloned());
        }
        prefs.apply_global();
        self.auto_restart = prefs.auto_restart;
        self.library.apply_prefs(&prefs, library_changed);
        if startup {
            self.settings = prefs.new_project_settings();
        } else if prefs != self.prefs {
            match prefs.save_to(&crate::prefs::file_path()) {
                Ok(()) => self.prefs_error = None,
                Err(e) => self.prefs_error = Some(tf("impostazioni non salvate: {e:#}", &[format!("{:#}", e)])),
            }
        }
        if library_changed {
            if let Err(e) = forge_library::paths::ensure() {
                self.prefs_error = Some(tf("cartella della libreria non utilizzabile: {e}", &[format!("{}", e)]));
            }
        }
        self.prefs = prefs;
    }

    pub fn import_dialog(&mut self) {
        let mut exts: Vec<&str> = forge_import::SUPPORTED_EXTENSIONS.to_vec();
        exts.extend(forge_import::BLENDER_EXTENSIONS);
        if cfg!(windows) {
            exts.extend(forge_import::SOLIDWORKS_EXTENSIONS);
        }
        let files = rfd::FileDialog::new()
            .add_filter(tr("Modelli 3D e CAD"), &exts)
            .add_filter("STEP", &["step", "stp"])
            .add_filter(tr("Blender / glTF"), &["blend", "glb", "gltf"])
            .add_filter(tr("Mesh (OBJ, STL)"), &["obj", "stl"])
            .pick_files()
            .unwrap_or_default();
        self.ask_import(files);
    }

    /// Mostra la finestra delle opzioni di importazione per i file indicati.
    pub fn ask_import(&mut self, files: Vec<PathBuf>) {
        if !files.is_empty() {
            self.import_queue.extend(files);
            self.show_import_options = true;
        }
    }

    /// Avvia l'importazione dei file in coda con le opzioni correnti.
    pub fn run_import_queue(&mut self) {
        self.show_import_options = false;
        self.process_import_queue();
    }

    fn process_import_queue(&mut self) {
        while self.pending.is_none() {
            let Some(path) = (!self.import_queue.is_empty()).then(|| self.import_queue.remove(0)) else {
                break;
            };
            self.import_path(&path);
        }
    }

    fn is_cad_format(path: &Path) -> bool {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        matches!(ext.as_str(), "step" | "stp")
    }

    fn is_solidworks(path: &Path) -> bool {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        matches!(ext.as_str(), "sldprt" | "sldasm")
    }

    fn is_blend(path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("blend"))
    }

    pub fn import_path(&mut self, path: &Path) {
        let solidworks = Self::is_solidworks(path);
        if Self::is_blend(path) || solidworks {
            // Esportazione glTF da Blender o da SolidWorks (via COM) in background.
            let (tx, rx) = std::sync::mpsc::channel();
            let p = path.to_path_buf();
            let _ = tx.send(TaskMsg::Progress(
                if solidworks {
                    tr("esportazione GLB da SolidWorks…")
                } else {
                    tr("esportazione glTF da Blender…")
                }
                .into(),
            ));
            std::thread::Builder::new()
                .name("forge-export".into())
                .spawn(move || {
                    let r = if solidworks {
                        forge_import::solidworks::export_glb(&p)
                    } else {
                        forge_import::blender::blend_to_glb(&p)
                    };
                    let _ = tx.send(TaskMsg::Blend(r.map_err(|e| e.to_string())));
                })
                .expect("thread di esportazione");
            self.pending = Some(Pending {
                label: tf(
                    "Importazione di {}",
                    &[format!("{}", path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default())],
                ),
                message: if solidworks {
                    tr("avvio di SolidWorks…")
                } else {
                    tr("avvio di Blender…")
                }
                .into(),
                path: path.to_path_buf(),
                opts: self.import_opts.clone(),
                rx,
                started: std::time::Instant::now(),
            });
            return;
        }
        if Self::is_cad_format(path) {
            // Conversione lenta (Open CASCADE / SolidWorks): in background.
            let (tx, rx) = std::sync::mpsc::channel();
            let p = path.to_path_buf();
            let opts = self.import_opts.clone();
            std::thread::Builder::new()
                .name("forge-import".into())
                .spawn(move || {
                    let tx2 = tx.clone();
                    let r = forge_import::step::convert(&p, &opts, |m| {
                        let _ = tx2.send(TaskMsg::Progress(m));
                    });
                    let _ = tx.send(TaskMsg::Model(r.map_err(|e| e.to_string())));
                })
                .expect("thread di importazione");
            self.pending = Some(Pending {
                label: tf(
                    "Importazione di {}",
                    &[format!("{}", path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default())],
                ),
                message: tr("avvio del convertitore…").into(),
                path: path.to_path_buf(),
                opts: self.import_opts.clone(),
                rx,
                started: std::time::Instant::now(),
            });
            return;
        }
        self.leave_demo_scene();
        self.remove_previous_import(path);
        let was_empty = self.scene.nodes.is_empty();
        let r = forge_import::import_file(&mut self.scene, path, &self.import_opts);
        self.finish_import(path, self.import_opts.clone(), was_empty, r);
    }

    /// Il primo import sostituisce la scena dimostrativa con una scena pulita.
    fn leave_demo_scene(&mut self) {
        if self.meta.test_scene && self.meta.path.is_none() {
            let s = Scene {
                lights: Vec::new(),
                ..Default::default()
            };
            self.replace_scene(s, ProjectMeta::default(), None);
        }
    }

    /// Se lo stesso file era già stato importato, lo si sostituisce (stessi id stabili:
    /// materiali, visibilità e trasformazioni assegnati restano validi).
    fn remove_previous_import(&mut self, path: &Path) {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
        if let Some(old) = self.scene.find(&forge_scene::ComponentId(file_name)) {
            self.scene.remove_subtree(old);
            self.selection.clear();
        }
    }

    fn finish_import(
        &mut self,
        path: &Path,
        opts: ImportOptions,
        was_empty: bool,
        result: Result<forge_import::ImportReport, forge_import::ImportError>,
    ) {
        match result {
            Ok(r) => {
                // Una scena con camera o luci proprie (glTF, Blender) resta com'è: spostarla
                // la disallineerebbe rispetto a camera e luci.
                let staged = r.camera.is_some() || !r.lights.is_empty();
                if opts.drop_to_floor && !staged {
                    self.scene.drop_to_floor(r.root, 0.0);
                }
                let project_dir = self.meta.path.as_deref().and_then(Path::parent);
                self.meta.sources.retain(|s| s.absolute_path.file_name() != path.file_name());
                self.meta.sources.push(forge_project::source_ref(project_dir, path, opts));
                if was_empty && self.meta.test_scene {
                    self.meta.test_scene = false;
                }
                self.geometry_changed();
                self.selection = BTreeSet::from([r.root]);
                let b = self.scene.bounds_of(r.root);
                if !r.lights.is_empty() {
                    if was_empty {
                        self.scene.lights = r.lights.clone();
                    } else {
                        self.scene.lights.extend(r.lights.iter().cloned());
                    }
                } else if was_empty || self.scene.lights.is_empty() {
                    self.scene.lights = forge_scene::light::preset("studio", b.center(), b.radius()).unwrap_or_default();
                }
                if let Some(cam) = &r.camera {
                    // La camera del file diventa una camera di rendering (attiva) e la vista la segue.
                    self.scene.camera = cam.clone();
                    self.sync_orbit_from_camera();
                    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                    self.add_render_camera(Some(tf("Camera ({stem})", std::slice::from_ref(&stem))));
                } else {
                    self.orbit.frame(&b);
                    self.camera_changed();
                }
                self.tab = InspectorTab::Object;
                let mut msg = tf(
                    "{}: {} componenti, {} triangoli ({} ms)",
                    &[
                        format!("{}", path.display()),
                        format!("{}", r.components),
                        format!("{}", r.triangles),
                        format!("{}", r.duration.as_millis()),
                    ],
                );
                if !r.warnings.is_empty() {
                    msg.push_str(&tf("\nAvvisi:\n- {}", &[r.warnings.join("\n- ").to_string()]));
                    self.info = Some(msg);
                }
            }
            Err(e) => {
                self.error = Some(tf(
                    "importazione di {} fallita: {e}",
                    &[format!("{}", path.display()), format!("{}", e)],
                ))
            }
        }
    }

    /// Avanzamento delle operazioni in background (import CAD, apertura progetto).
    fn poll_pending(&mut self) {
        let Some(p) = &mut self.pending else { return };
        let mut done = None;
        while let Ok(msg) = p.rx.try_recv() {
            match msg {
                TaskMsg::Progress(m) => p.message = m,
                other => done = Some(other),
            }
        }
        let Some(done) = done else { return };
        let p = self.pending.take().expect("pending");
        match done {
            TaskMsg::Model(Ok(model)) => {
                self.leave_demo_scene();
                self.remove_previous_import(&p.path);
                let was_empty = self.scene.nodes.is_empty();
                let file_name = p.path.file_name().and_then(|n| n.to_str()).unwrap_or("modello").to_string();
                let r = forge_import::step::insert(&mut self.scene, &file_name, model, &p.opts).map(|r| forge_import::ImportReport {
                    duration: p.started.elapsed(),
                    ..r
                });
                self.finish_import(&p.path, p.opts.clone(), was_empty, r);
            }
            TaskMsg::Model(Err(e)) | TaskMsg::Blend(Err(e)) => {
                self.error = Some(tf(
                    "importazione di {} fallita:\n{e}",
                    &[format!("{}", p.path.display()), e.to_string()],
                ))
            }
            TaskMsg::Blend(Ok(export)) => {
                self.leave_demo_scene();
                self.remove_previous_import(&p.path);
                let was_empty = self.scene.nodes.is_empty();
                let file_name = p.path.file_name().and_then(|n| n.to_str()).unwrap_or("scena.blend").to_string();
                let dir = p.path.parent().unwrap_or(Path::new(".")).to_path_buf();
                let r =
                    forge_import::gltf_import::import_named(&mut self.scene, &export.glb, &file_name, &dir, &p.opts, export.area_lights)
                        .map(|r| forge_import::ImportReport {
                            duration: p.started.elapsed(),
                            ..r
                        });
                let _ = std::fs::remove_file(&export.glb);
                self.finish_import(&p.path, p.opts.clone(), was_empty, r);
            }
            TaskMsg::Project(Ok(l)) => self.apply_loaded_project(*l),
            TaskMsg::Project(Err(e)) => self.error = Some(tf("apertura del progetto fallita: {e}", std::slice::from_ref(&e))),
            TaskMsg::Progress(_) => {}
        }
        self.process_import_queue();
    }

    pub fn open_project_dialog(&mut self) {
        if let Some(p) = rfd::FileDialog::new()
            .add_filter(tr("Progetto ForgeRender"), &[forge_project::EXTENSION])
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
        if self.pending.is_some() {
            self.error = Some(tr("attendi la fine dell'operazione in corso").into());
            return;
        }
        // Il progetto può reimportare file STEP: caricamento in background.
        let (tx, rx) = std::sync::mpsc::channel();
        let p = path.to_path_buf();
        std::thread::Builder::new()
            .name("forge-open".into())
            .spawn(move || {
                let r = forge_project::load(&p).map(Box::new).map_err(|e| format!("{e:#}"));
                let _ = tx.send(TaskMsg::Project(r));
            })
            .expect("thread di apertura");
        self.pending = Some(Pending {
            label: tf(
                "Apertura di {}",
                &[format!("{}", path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default())],
            ),
            message: tr("lettura del progetto e dei file sorgente…").into(),
            path: path.to_path_buf(),
            opts: self.import_opts.clone(),
            rx,
            started: std::time::Instant::now(),
        });
    }

    fn apply_loaded_project(&mut self, l: forge_project::LoadedProject) {
        {
            let report = l.report;
            self.replace_scene(l.scene, l.meta, Some(l.settings));
            if let Some(p) = self.meta.path.clone() {
                self.remember_recent(&p);
            }
            let mut notes = Vec::new();
            if !report.missing_sources.is_empty() {
                notes.push(tf(
                    "File sorgente non trovati:\n- {}",
                    &[report
                        .missing_sources
                        .iter()
                        .map(|p| p.display().to_string())
                        .collect::<Vec<_>>()
                        .join("\n- ")
                        .to_string()],
                ));
            }
            if !report.missing_components.is_empty() {
                notes.push(tf("{} componenti con materiale assegnato non sono più presenti (rinominati o rimossi). Le assegnazioni sono conservate:\n- {}", &[format!("{}", report.missing_components.len()), report.missing_components.iter().take(20).map(|c| c.0.clone()).collect::<Vec<_>>().join("\n- ").to_string()]));
            }
            notes.extend(report.warnings);
            if !notes.is_empty() {
                self.info = Some(notes.join("\n\n"));
            }
        }
    }

    pub fn save_project(&mut self, save_as: bool) {
        let path = match (&self.meta.path, save_as) {
            (Some(p), false) => p.clone(),
            _ => {
                let Some(p) = rfd::FileDialog::new()
                    .add_filter(tr("Progetto ForgeRender"), &[forge_project::EXTENSION])
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
                self.remember_recent(&path);
                self.meta.path = Some(path);
                self.unsaved = false;
            }
            Err(e) => self.error = Some(tf("salvataggio fallito: {e:#}", &[format!("{:#}", e)])),
        }
    }

    /// Mostra un avviso breve in basso (scompare da solo dopo qualche secondo).
    pub fn toast(&mut self, text: impl Into<String>) {
        self.toast = Some((text.into(), std::time::Instant::now()));
    }

    /// Aggiunge il progetto a File → Progetti recenti (salvato in `impostazioni.json`).
    fn remember_recent(&mut self, path: &Path) {
        let mut p = self.prefs.clone();
        if p.push_recent(path) {
            self.apply_prefs(p, false);
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
    /// Dopo una selezione: mostra la scheda Materiali, salvo che l'utente stia
    /// già lavorando su Oggetto o Materiali.
    pub fn show_selection_tab(&mut self) {
        if !matches!(self.tab, InspectorTab::Object | InspectorTab::Materials) {
            self.tab = InspectorTab::Materials;
        }
    }

    /// Chiamata a ogni frame prima del disegno dell'interfaccia.
    /// Selezione richiesta da riga di comando: applicata appena il componente esiste.
    fn apply_startup_select(&mut self) {
        if self.pending.is_none() {
            if let Some(name) = self.startup_light.take() {
                if let Some(i) = self.scene.lights.iter().position(|l| l.name == name) {
                    self.selected_light = Some(i);
                    self.tab = InspectorTab::Lights;
                }
            }
            if let Some(name) = self.startup_fog.take() {
                if let Some(i) = self.scene.fogs.iter().position(|f| f.name == name) {
                    self.selected_fog = Some(i);
                    self.tab = InspectorTab::Effects;
                }
            }
            if let Some(name) = self.startup_aim.take() {
                if let Some(i) = self.scene.nodes.iter().position(|n| n.name == name) {
                    self.aim_at(i);
                    self.tab = InspectorTab::Camera;
                }
            }
        }
        let Some(name) = &self.startup_select else { return };
        if self.pending.is_some() {
            return; // progetto in caricamento
        }
        if let Some(i) = self.scene.nodes.iter().position(|n| &n.name == name) {
            self.selection.insert(i);
            self.selected_material = self.scene.resolve_material(i).clone();
            self.startup_select = None;
        }
    }

    pub fn tick(&mut self, ctx: &egui::Context) {
        self.frames += 1;
        self.poll_geometry();
        self.poll_fog();
        self.poll_pending();
        self.apply_startup_select();
        if let Some(rx) = &self.gpu_warmup {
            if let Ok(r) = rx.try_recv() {
                if let Err(e) = &r {
                    tracing::warn!("GPU non disponibile per il rendering: {e}");
                }
                self.gpu = Some(r);
                self.gpu_warmup = None;
            }
        }
        self.apply_simple_defaults();
        self.update_render_job();
        self.monitor.tick();
        let state = self.render_state();
        if state == SessionState::Rendering || self.is_preparing() || self.pending.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
        crate::mcp::process(self, ctx);
        self.handle_screenshot(ctx);
        let title = tf(
            "{APP_TITLE} — {}{}",
            &[
                APP_TITLE.to_string(),
                self.meta
                    .path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "senza titolo".into())
                    .to_string(),
                (if self.unsaved { " *" } else { "" }).to_string(),
            ],
        );
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
    }

    /// Modalità Semplice: valori di base sempre attivi (vedi `presets::apply_simple_base`) e GPU
    /// come dispositivo predefinito appena si sa che funziona.
    fn apply_simple_defaults(&mut self) {
        if self.prefs.ui_mode != crate::prefs::UiMode::Simple {
            return;
        }
        crate::presets::apply_simple_base(&mut self.settings);
        if !self.simple_gpu_checked {
            match self.gpu_ready() {
                Some(true) => {
                    self.settings.backend = forge_render_core::BackendKind::Gpu;
                    self.simple_gpu_checked = true;
                }
                Some(false) => self.simple_gpu_checked = true,
                None => {}
            }
        }
    }

    /// Screenshot automatico per i test della GUI (`--screenshot`).
    fn handle_screenshot(&mut self, ctx: &egui::Context) {
        let Some(path) = self.startup.screenshot.clone() else { return };
        if self.screenshot_done.load(std::sync::atomic::Ordering::Acquire) {
            self.exit_confirmed = true;
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

/// Voci del menu "Aggiungi".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddKind {
    Floor,
    Backdrop,
    StudioSweep,
    Wall,
    Box,
    Sphere,
    Cylinder,
    Cup,
}

pub const ADD_MENU: &[(&str, AddKind)] = &[
    (tk("Pavimento"), AddKind::Floor),
    (tk("Fondale da studio"), AddKind::Backdrop),
    (tk("Fondale fotografico (tela inclinata)"), AddKind::StudioSweep),
    (tk("Parete"), AddKind::Wall),
    (tk("Cubo"), AddKind::Box),
    (tk("Sfera"), AddKind::Sphere),
    (tk("Cilindro"), AddKind::Cylinder),
    (tk("Tazza"), AddKind::Cup),
];

/// Nome del formato nella finestra "Esporta…".
pub fn export_format_label(f: forge_image_io::ExportFormat) -> &'static str {
    use forge_image_io::ExportFormat::*;
    match f {
        Png => tk("PNG (senza perdita, 8 bit)"),
        Jpeg => tk("JPG (leggero, per web e social)"),
        Tiff16 => tk("TIFF 16 bit (per il fotoritocco)"),
        Exr => tk("EXR (valori lineari, per il compositing)"),
    }
}

/// Riavvio automatico del render quando la scena o la vista cambiano (casella "Riavvio
/// automatico"): acceso = riparte sempre, in qualunque vista e in qualunque stato (in corso,
/// in pausa, finito, fermato); spento = non riparte mai. Serve un render già avviato almeno
/// una volta (`stale` è falso finché non c'è).
///
/// Con una camera di render attiva (senza "Pilota") muovere la vista non cambia l'inquadratura
/// del render, quindi il render non diventa vecchio e non riparte.
pub fn should_auto_restart(auto: bool, stale: bool, preparing: bool) -> bool {
    auto && stale && !preparing
}

#[cfg(test)]
mod auto_restart_tests {
    use super::*;

    #[test]
    fn auto_restart_is_on_or_off() {
        assert!(should_auto_restart(true, true, false), "acceso: riparte sempre");
        assert!(!should_auto_restart(false, true, false), "spento: mai");
        assert!(!should_auto_restart(true, false, false), "immagine aggiornata");
        assert!(!should_auto_restart(true, true, true), "scena in preparazione: si aspetta");
    }
}
