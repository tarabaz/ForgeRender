//! Finestra di importazione STEP, su tre colonne: il file, la tassellazione e l'anteprima del
//! risultato. La conversione gira nel processo `forgerender-step` (Open CASCADE) e si può rifare con
//! altri parametri senza riaprire il file; nella scena entra solo con Importa.
//!
//! I controlli sono i parametri che il convertitore usa davvero: deviazione lineare relativa e
//! deviazione angolare di `BRepMesh_IncrementalMesh`. Lunghezza massima degli spigoli e spigoli vivi
//! non ci sono perché il convertitore non li espone. La geometria B-rep di partenza non si può mostrare:
//! Open CASCADE resta nel suo processo, qui arrivano solo i triangoli.
//!
//! L'anteprima è il modello come verrebbe importato: lo stesso inserimento nella scena di Importa
//! (pezzi, colori, asse verticale, appoggio sul pavimento) disegnato dallo stesso motore della
//! viewport, ombreggiato o con gli spigoli dei triangoli (vista Mesh). Importa mette nel progetto
//! proprio quel modello, senza convertirlo di nuovo.

use crate::app::ForgeApp;
use crate::i18n::{tf, tr};
use crate::ui::style;
use crate::viewport::ViewportRenderer;
use eframe::egui;
use eframe::egui_wgpu::RenderState;
use forge_import::frmesh::FrMesh;
use forge_import::{ImportOptions, StepQuality, UpAxis};
use forge_math::Vec3;
use forge_scene::{OrbitController, Scene};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Quality {
    Draft,
    Balanced,
    High,
    Custom,
}

impl Quality {
    /// Deviazione lineare relativa e deviazione angolare (radianti) che il convertitore applica.
    fn params(self) -> Option<(f64, f64)> {
        match self {
            Quality::Draft => Some(StepQuality::Bassa.deflections()),
            Quality::Balanced => Some(StepQuality::Media.deflections()),
            Quality::High => Some(StepQuality::Alta.deflections()),
            Quality::Custom => None,
        }
    }

    fn step_quality(self) -> Option<StepQuality> {
        match self {
            Quality::Draft => Some(StepQuality::Bassa),
            Quality::Balanced => Some(StepQuality::Media),
            Quality::High => Some(StepQuality::Alta),
            Quality::Custom => None,
        }
    }
}

enum Msg {
    Progress(String),
    Done(Box<Result<FrMesh, String>>),
}

struct Job {
    rx: Receiver<Msg>,
    cancel: Arc<AtomicBool>,
    started: Instant,
}

/// Risultato di una conversione.
struct Converted {
    parts: usize,
    triangles: usize,
    vertices: usize,
    unit_m: f64,
    seconds: f32,
    /// Il modello convertito: Importa mette nella scena proprio questo.
    model: FrMesh,
    /// Parametri con cui è stato fatto: se cambia la tassellazione, l'anteprima non è più aggiornata.
    made_with: ImportOptions,
    /// Il modello inserito in una scena vuota come lo inserirebbe Importa.
    preview: Preview,
}

/// Scena dell'anteprima e opzioni con cui è stata costruita (asse verticale, appoggio).
struct Preview {
    scene: Scene,
    up: UpAxis,
    drop_to_floor: bool,
    /// Dimensioni in millimetri, come risulteranno nella scena.
    size_mm: Vec3,
    radius: f32,
    /// Geometria già caricata sulla GPU dell'anteprima.
    uploaded: bool,
}

pub struct StepImport {
    pub open: bool,
    path: PathBuf,
    bytes: u64,
    opts: ImportOptions,
    quality: Quality,
    /// Deviazione lineare in percento e deviazione angolare in gradi.
    linear_pct: f64,
    angular_deg: f64,
    job: Option<Job>,
    status: String,
    error: Option<String>,
    result: Option<Converted>,
    /// Vista Mesh (spigoli dei triangoli sopra l'ombreggiatura), come nella viewport.
    wire: bool,
    orbit: OrbitController,
    /// Il motore della viewport, con texture sue (si tiene tra un file e l'altro).
    renderer: Option<ViewportRenderer>,
}

impl Default for StepImport {
    fn default() -> Self {
        Self {
            open: false,
            path: PathBuf::new(),
            bytes: 0,
            opts: ImportOptions::default(),
            quality: Quality::Balanced,
            linear_pct: 0.4,
            angular_deg: 17.2,
            job: None,
            status: String::new(),
            error: None,
            result: None,
            wire: false,
            orbit: OrbitController::default(),
            renderer: None,
        }
    }
}

impl StepImport {
    /// Apre la finestra sul file e fa subito una prima conversione con i parametri attuali.
    pub fn open_file(&mut self, path: &Path, opts: &ImportOptions) {
        self.cancel();
        let renderer = self.renderer.take();
        *self = Self {
            open: true,
            path: path.to_path_buf(),
            bytes: std::fs::metadata(path).map(|m| m.len()).unwrap_or(0),
            opts: opts.clone(),
            renderer,
            ..Self::default()
        };
        self.quality = match opts.step_quality {
            StepQuality::Bassa => Quality::Draft,
            StepQuality::Media => Quality::Balanced,
            StepQuality::Alta => Quality::High,
        };
        self.apply_quality();
        self.convert();
    }

    fn apply_quality(&mut self) {
        if let Some((l, a)) = self.quality.params() {
            self.linear_pct = l * 100.0;
            self.angular_deg = a.to_degrees();
        }
    }

    /// Opzioni che valgono ora, con i parametri della colonna centrale.
    fn options(&self) -> ImportOptions {
        let mut o = self.opts.clone();
        match self.quality.step_quality() {
            Some(q) => {
                o.step_quality = q;
                o.step_custom = None;
            }
            None => o.step_custom = Some([self.linear_pct / 100.0, self.angular_deg.to_radians()]),
        }
        o
    }

    fn cancel(&mut self) {
        if let Some(j) = self.job.take() {
            j.cancel.store(true, Ordering::Relaxed);
        }
    }

    fn convert(&mut self) {
        self.cancel();
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let (path, opts, stop) = (self.path.clone(), self.options(), cancel.clone());
        self.error = None;
        self.status = tr("avvio del convertitore…").to_string();
        let spawned = std::thread::Builder::new().name("forge-anteprima-step".into()).spawn(move || {
            let tx2 = tx.clone();
            let r = forge_import::step::convert_cancellable(&path, &opts, &stop, |m| {
                let _ = tx2.send(Msg::Progress(m));
            });
            let _ = tx.send(Msg::Done(Box::new(r.map_err(|e| e.to_string()))));
        });
        match spawned {
            Ok(_) => {
                self.job = Some(Job {
                    rx,
                    cancel,
                    started: Instant::now(),
                })
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    fn poll(&mut self, ctx: &egui::Context) {
        let Some(job) = &self.job else { return };
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
        let started = job.started;
        while let Ok(msg) = job.rx.try_recv() {
            match msg {
                Msg::Progress(m) => self.status = m,
                Msg::Done(r) => {
                    self.job = None;
                    match *r {
                        Ok(m) if m.meshes.iter().all(|x| x.indices.is_empty()) => {
                            self.error = Some(tr("Il file non contiene geometria da convertire").to_string());
                            self.result = None;
                        }
                        Ok(m) => {
                            let seconds = started.elapsed().as_secs_f32();
                            match summarize(&self.path, m, seconds, self.options()) {
                                Ok(c) => {
                                    self.orbit = framed_orbit(&c.preview);
                                    self.result = Some(c);
                                    self.error = None;
                                }
                                Err(e) => {
                                    self.error = Some(e);
                                    self.result = None;
                                }
                            }
                            self.status.clear();
                        }
                        Err(e) => {
                            self.error = Some(e);
                            self.status.clear();
                        }
                    }
                    return;
                }
            }
        }
    }
}

/// Tassellazione con cui sono fatte le opzioni: è l'unica cosa che richiede di convertire di nuovo.
fn tessellation(o: &ImportOptions) -> (StepQuality, Option<[f64; 2]>) {
    (o.step_quality, o.step_custom)
}

/// Inserisce il modello in una scena vuota esattamente come fa Importa: stessi pezzi, materiali
/// dai colori del file, asse verticale e appoggio sul pavimento.
fn build_preview(path: &Path, model: &FrMesh, opts: &ImportOptions) -> Result<Preview, String> {
    let mut scene = Scene {
        lights: Vec::new(),
        ..Default::default()
    };
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("modello").to_string();
    let r = forge_import::step::insert(&mut scene, &name, model.clone(), opts).map_err(|e| e.to_string())?;
    if opts.drop_to_floor {
        scene.drop_to_floor(r.root, 0.0);
    }
    let b = scene.bounds();
    Ok(Preview {
        size_mm: if b.is_empty() { Vec3::ZERO } else { (b.max - b.min) * 1000.0 },
        radius: b.radius().max(1e-3),
        scene,
        up: opts.up,
        drop_to_floor: opts.drop_to_floor,
        uploaded: false,
    })
}

/// Camera dell'anteprima: tre quarti dall'alto, con il modello inquadrato.
fn framed_orbit(p: &Preview) -> OrbitController {
    let mut orbit = OrbitController {
        yaw: 0.6,
        pitch: 0.35,
        ..Default::default()
    };
    orbit.frame(&p.scene.bounds());
    orbit
}

fn summarize(path: &Path, m: FrMesh, seconds: f32, made_with: ImportOptions) -> Result<Converted, String> {
    let preview = build_preview(path, &m, &made_with)?;
    Ok(Converted {
        parts: m.header.nodes.iter().filter(|n| n.mesh.is_some()).count(),
        triangles: m.meshes.iter().map(|x| x.indices.len()).sum(),
        vertices: m.meshes.iter().map(|x| x.positions.len()).sum(),
        unit_m: m.header.unit_m,
        seconds,
        model: m,
        made_with,
        preview,
    })
}

pub fn window(app: &mut ForgeApp, ctx: &egui::Context) {
    if !app.step_import.open {
        return;
    }
    app.step_import.poll(ctx);
    let rs = app.wgpu_state.clone();
    // Sfondo dell'anteprima: lo stesso della viewport (Impostazioni → Viewport).
    let background = app.viewport_background();
    let grid = app.viewport_grid();
    let mut open = true;
    let mut import = false;
    let mut close = false;
    let st = &mut app.step_import;
    st.refresh_preview();
    egui::Window::new(tr("Importazione STEP"))
        .id(egui::Id::NULL.with("win-step"))
        .open(&mut open)
        .collapsible(false)
        .default_size([1080.0, 600.0])
        .max_size(ctx.content_rect().size() - egui::vec2(24.0, 24.0))
        .show(ctx, |ui| {
            ui.columns(3, |c| {
                source_column(&mut c[0], st);
                settings_column(&mut c[1], st);
                preview_column(&mut c[2], st, rs.as_ref(), background, grid);
            });
            ui.separator();
            ui.horizontal(|ui| {
                let stale = st.is_stale();
                let ready = st.job.is_none() && st.result.is_some() && !stale;
                if ui
                    .add_enabled(
                        ready,
                        egui::Button::new(egui::RichText::new(tr("Importa")).strong()).fill(style::ACCENT),
                    )
                    .on_hover_text(tr("Mette nella scena il risultato che vedi nell'anteprima"))
                    .on_disabled_hover_text(tr("Prima converti con i parametri attuali"))
                    .clicked()
                {
                    import = true;
                }
                if ui.button(tr("Annulla")).clicked() {
                    close = true;
                }
            });
        });
    if import {
        let st = &mut app.step_import;
        st.open = false;
        let (path, opts) = (st.path.clone(), st.options());
        if let Some(c) = st.result.take() {
            let started = Instant::now() - std::time::Duration::from_secs_f32(c.seconds);
            app.import_converted_step(&path, opts, c.model, started);
        }
    } else if close || !open {
        app.step_import.cancel();
        app.step_import.open = false;
    }
}

fn source_column(ui: &mut egui::Ui, st: &mut StepImport) {
    ui.heading(tr("File"));
    let name = st.path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    ui.add(egui::Label::new(egui::RichText::new(name).strong()).truncate());
    ui.label(format!("{:.1} MB", st.bytes as f64 / 1e6));
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new(tr(
            "La geometria CAD originale non si può mostrare qui: l'anteprima è il risultato della conversione.",
        ))
        .weak(),
    );
    ui.separator();
    ui.heading(tr("Impostazioni"));
    egui::Grid::new("step-file").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr("Asse verticale"));
        egui::ComboBox::from_id_salt("step-up")
            .selected_text(tr(st.opts.up.label()))
            .show_ui(ui, |ui| {
                for u in [UpAxis::Auto, UpAxis::Y, UpAxis::Z] {
                    ui.selectable_value(&mut st.opts.up, u, tr(u.label()));
                }
            });
        ui.end_row();
        ui.label(tr("Posizione"));
        ui.checkbox(&mut st.opts.drop_to_floor, tr("Appoggia sul pavimento dopo l'importazione"));
        ui.end_row();
    });
    if let Some(r) = &st.result {
        ui.separator();
        ui.heading(tr("Modello"));
        let unit = match r.unit_m {
            u if (u - 0.001).abs() < 1e-9 => "mm".to_string(),
            u if (u - 0.01).abs() < 1e-9 => "cm".to_string(),
            u if (u - 1.0).abs() < 1e-9 => "m".to_string(),
            u if (u - 0.0254).abs() < 1e-9 => tr("pollici").to_string(),
            u => format!("{u} m"),
        };
        ui.label(tf("Unità dichiarate dal file: {}", &[unit]));
        let size = r.preview.size_mm;
        ui.label(tf(
            "Dimensioni: {} × {} × {} mm",
            &[format!("{:.1}", size.x), format!("{:.1}", size.y), format!("{:.1}", size.z)],
        ));
        ui.label(tf("Pezzi: {}", &[r.parts.to_string()]));
    }
}

fn settings_column(ui: &mut egui::Ui, st: &mut StepImport) {
    ui.heading(tr("Tassellazione"));
    ui.horizontal_wrapped(|ui| {
        for (q, label, hint) in [
            (
                Quality::Draft,
                tr("Bozza"),
                tr("Pochi triangoli: per provare in fretta (1 % e 28,6°)"),
            ),
            (
                Quality::Balanced,
                tr("Bilanciata"),
                tr("Il valore di sempre: curve pulite a un costo ragionevole (0,4 % e 17,2°)"),
            ),
            (
                Quality::High,
                tr("Alta qualità"),
                tr("Molti triangoli: per primi piani e superfici lucide (0,15 % e 8,6°)"),
            ),
            (Quality::Custom, tr("Personalizzata"), tr("I due valori qui sotto")),
        ] {
            if ui.selectable_label(st.quality == q, label).on_hover_text(hint).clicked() {
                st.quality = q;
                st.apply_quality();
            }
        }
    });
    ui.add_space(6.0);
    let custom = st.quality == Quality::Custom;
    egui::Grid::new("step-tess").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr("Deviazione lineare"));
        ui.add_enabled(
            custom,
            egui::DragValue::new(&mut st.linear_pct)
                .range(0.02..=5.0)
                .speed(0.01)
                .suffix(" %")
                .max_decimals(3),
        )
        .on_hover_text(tr("Quanto la mesh può scostarsi dalla superficie CAD, in percento della dimensione di ogni spigolo. Più bassa = curve più fedeli e più triangoli"));
        ui.end_row();
        ui.label(tr("Deviazione angolare"));
        ui.add_enabled(
            custom,
            egui::DragValue::new(&mut st.angular_deg)
                .range(1.0..=60.0)
                .speed(0.1)
                .suffix("°")
                .max_decimals(1),
        )
        .on_hover_text(tr("Angolo massimo tra due triangoli vicini su una curva. Più basso = archi più lisci e più triangoli"));
        ui.end_row();
    });
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        let busy = st.job.is_some();
        if ui.add_enabled(!busy, egui::Button::new(tr("Converti"))).clicked() {
            st.convert();
        }
        if busy && ui.button(tr("Ferma")).clicked() {
            st.cancel();
            st.status.clear();
        }
        if ui.button(tr("Valori predefiniti")).clicked() {
            st.quality = Quality::Balanced;
            st.apply_quality();
        }
    });
    if let Some(j) = &st.job {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(tf(
                "{} ({} s)",
                &[st.status.clone(), format!("{:.0}", j.started.elapsed().as_secs_f32())],
            ));
        });
    }
    if let Some(e) = &st.error {
        ui.colored_label(ui.visuals().error_fg_color, e);
    }
}

impl StepImport {
    /// Asse verticale o appoggio cambiati: si rifà la scena dell'anteprima dal modello già
    /// convertito (non serve convertire di nuovo).
    fn refresh_preview(&mut self) {
        let (up, drop) = (self.opts.up, self.opts.drop_to_floor);
        let Some(c) = &mut self.result else { return };
        if c.preview.up == up && c.preview.drop_to_floor == drop {
            return;
        }
        let mut opts = c.made_with.clone();
        opts.up = up;
        opts.drop_to_floor = drop;
        match build_preview(&self.path, &c.model, &opts) {
            Ok(p) => {
                c.preview = p;
                c.made_with = opts;
                self.orbit = framed_orbit(&c.preview);
            }
            Err(e) => self.error = Some(e),
        }
    }

    /// La tassellazione scelta non è quella dell'anteprima: Importa resta spento finché non si
    /// converte di nuovo.
    fn is_stale(&self) -> bool {
        self.result
            .as_ref()
            .is_some_and(|r| tessellation(&r.made_with) != tessellation(&self.options()))
    }
}

fn preview_column(
    ui: &mut egui::Ui,
    st: &mut StepImport,
    rs: Option<&RenderState>,
    background: Vec3,
    grid: Option<crate::viewport::GridLook>,
) {
    ui.heading(tr("Anteprima 3D"));
    let side = (ui.available_height() - 90.0).max(160.0).min(ui.available_width() * 1.2);
    let (rect, r) = ui.allocate_exact_size(egui::vec2(ui.available_width(), side), egui::Sense::click_and_drag());
    let [cr, cg, cb] = crate::ui::color_picker::to_srgb8(background.to_array());
    ui.painter().rect_filled(rect, 3.0, egui::Color32::from_rgb(cr, cg, cb));
    // Mouse come nella viewport: sinistro gira, Maiusc o destro/centrale sposta, rotella avvicina.
    let delta = r.drag_delta();
    if delta != egui::Vec2::ZERO {
        let shift = ui.input(|i| i.modifiers.shift);
        if r.dragged_by(egui::PointerButton::Secondary)
            || r.dragged_by(egui::PointerButton::Middle)
            || (shift && r.dragged_by(egui::PointerButton::Primary))
        {
            st.orbit.pan(delta.x / rect.height(), delta.y / rect.height());
        } else if r.dragged_by(egui::PointerButton::Primary) {
            st.orbit.orbit(delta.x * 0.008, delta.y * 0.008);
        }
    }
    if r.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            st.orbit.zoom((-scroll * 0.0015).exp());
        }
    }
    if r.double_clicked() {
        if let Some(c) = &st.result {
            st.orbit = framed_orbit(&c.preview);
        }
    }
    let StepImport {
        result,
        renderer,
        orbit,
        wire,
        ..
    } = st;
    match (result.as_mut(), rs) {
        (Some(c), Some(rs)) => {
            let vp = renderer.get_or_insert_with(|| ViewportRenderer::new(rs));
            if !c.preview.uploaded {
                vp.upload_scene(rs, &c.preview.scene, &BTreeSet::new());
                c.preview.uploaded = true;
            }
            let ppp = ui.ctx().pixels_per_point();
            let px = [(rect.width() * ppp).round() as u32, (rect.height() * ppp).round() as u32];
            if let Some(tex) = vp.render(
                rs,
                &orbit.camera(),
                c.preview.radius,
                px,
                crate::viewport::ViewStyle {
                    background,
                    mesh_view: *wire,
                    grid,
                },
            ) {
                let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                ui.painter_at(rect).image(tex, rect, uv, egui::Color32::WHITE);
            }
        }
        (Some(_), None) => {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                tr("Anteprima non disponibile: la viewport 3D è spenta (modalità sicura)"),
                egui::FontId::proportional(13.0),
                ui.visuals().weak_text_color(),
            );
        }
        (None, _) => {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                tr("Nessuna anteprima"),
                egui::FontId::proportional(13.0),
                ui.visuals().weak_text_color(),
            );
        }
    }
    r.on_hover_text(tr(
        "Il modello come entrerà nella scena, disegnato come nella viewport. Trascina per girare, \
         Maiusc o tasto destro per spostare, rotella per avvicinare, doppio clic per inquadrare.",
    ));
    ui.horizontal(|ui| {
        ui.selectable_value(&mut st.wire, false, tr("Ombreggiata"));
        ui.selectable_value(&mut st.wire, true, tr("Fil di ferro"));
        if ui.button(tr("Inquadra")).clicked() {
            if let Some(c) = &st.result {
                st.orbit = framed_orbit(&c.preview);
            }
        }
    });
    if let Some(c) = &st.result {
        ui.label(tf(
            "{} triangoli, {} vertici, {} s",
            &[c.triangles.to_string(), c.vertices.to_string(), format!("{:.1}", c.seconds)],
        ));
    }
    if st.is_stale() {
        ui.colored_label(style::WARN, tr("Parametri cambiati: premi Converti per aggiornare"));
    }
}
