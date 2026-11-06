//! Finestra di importazione STEP, su tre colonne: il file, la tassellazione e l'anteprima del
//! risultato. La conversione gira nel processo `forgerender-step` (Open CASCADE) e si può rifare con
//! altri parametri senza riaprire il file; nella scena entra solo con Importa.
//!
//! I controlli sono i parametri che il convertitore usa davvero: deviazione lineare relativa e
//! deviazione angolare di `BRepMesh_IncrementalMesh`. Lunghezza massima degli spigoli e spigoli vivi
//! non ci sono perché il convertitore non li espone. La geometria B-rep di partenza non si può mostrare:
//! Open CASCADE resta nel suo processo, qui arrivano solo i triangoli.

use crate::app::ForgeApp;
use crate::i18n::{tf, tr};
use crate::ui::style;
use eframe::egui;
use forge_import::frmesh::FrMesh;
use forge_import::{ImportOptions, StepQuality, UpAxis};
use forge_math::{Mat4, Vec3};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::Instant;

/// Triangoli al massimo nell'anteprima (è disegnata dall'interfaccia, non dal render).
const PREVIEW_TRIANGLES: usize = 60_000;
/// Triangoli al massimo nel fil di ferro.
const WIRE_TRIANGLES: usize = 15_000;

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
    /// Dimensioni in millimetri.
    size_mm: Vec3,
    unit_m: f64,
    seconds: f32,
    /// Triangoli del modello in spazio normalizzato, con la normale e il colore.
    tris: Vec<([Vec3; 3], Vec3, [f32; 3])>,
    /// Parametri con cui è stato fatto: se cambiano, l'anteprima non è più aggiornata.
    made_with: ImportOptions,
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
    wire: bool,
    yaw: f32,
    pitch: f32,
    zoom: f32,
    light: f32,
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
            yaw: 0.6,
            pitch: 0.4,
            zoom: 1.0,
            light: 0.5,
        }
    }
}

impl StepImport {
    /// Apre la finestra sul file e fa subito una prima conversione con i parametri attuali.
    pub fn open_file(&mut self, path: &Path, opts: &ImportOptions) {
        *self = Self {
            open: true,
            path: path.to_path_buf(),
            bytes: std::fs::metadata(path).map(|m| m.len()).unwrap_or(0),
            opts: opts.clone(),
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
                            self.result = Some(summarize(&m, started.elapsed().as_secs_f32(), self.options()));
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

/// Mette insieme il modello: ogni pezzo al suo posto, centrato e riportato a misura uno.
fn summarize(m: &FrMesh, seconds: f32, made_with: ImportOptions) -> Converted {
    let nodes = &m.header.nodes;
    let matrix = |i: usize| {
        let a = nodes[i].matrix.map(|v| v as f32);
        Mat4::from_cols_array(&a)
    };
    let world = |mut i: usize| {
        let mut w = matrix(i);
        while let Some(p) = nodes[i].parent {
            i = p as usize;
            w = matrix(i) * w;
        }
        w
    };
    let z_up = made_with.up.is_z_up("step");
    let up = if z_up {
        Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2)
    } else {
        Mat4::IDENTITY
    };
    let triangles: usize = m.meshes.iter().map(|x| x.indices.len()).sum();
    let vertices: usize = m.meshes.iter().map(|x| x.positions.len()).sum();
    let step = (triangles / PREVIEW_TRIANGLES).max(1);
    let mut tris = Vec::with_capacity(triangles.min(PREVIEW_TRIANGLES) + 16);
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    let mut parts = 0;
    for (i, n) in nodes.iter().enumerate() {
        let Some(mesh) = n.mesh.and_then(|k| m.meshes.get(k as usize)) else {
            continue;
        };
        parts += 1;
        let xf = up * world(i);
        let color = n.color.unwrap_or([0.72, 0.74, 0.78]);
        for p in &mesh.positions {
            let q = xf.transform_point3(Vec3::from(*p));
            lo = lo.min(q);
            hi = hi.max(q);
        }
        for t in mesh.indices.iter().step_by(step) {
            let v = t.map(|k| xf.transform_point3(Vec3::from(mesh.positions[k as usize])));
            let n = (v[1] - v[0]).cross(v[2] - v[0]).normalize_or_zero();
            tris.push((v, n, color));
        }
    }
    let center = (lo + hi) * 0.5;
    let size = (hi - lo).max(Vec3::splat(1e-9));
    let scale = 1.0 / size.max_element();
    for t in &mut tris {
        for v in &mut t.0 {
            *v = (*v - center) * scale;
        }
    }
    Converted {
        parts,
        triangles,
        vertices,
        size_mm: size * (m.header.unit_m * 1000.0) as f32,
        unit_m: m.header.unit_m,
        seconds,
        tris,
        made_with,
    }
}

pub fn window(app: &mut ForgeApp, ctx: &egui::Context) {
    if !app.step_import.open {
        return;
    }
    app.step_import.poll(ctx);
    let mut open = true;
    let mut import = false;
    let mut close = false;
    let st = &mut app.step_import;
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
                preview_column(&mut c[2], st);
            });
            ui.separator();
            ui.horizontal(|ui| {
                let stale = st.result.as_ref().is_some_and(|r| r.made_with != st.options());
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
        app.import_opts = opts;
        app.import_path(&path);
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
        ui.label(tf(
            "Dimensioni: {} × {} × {} mm",
            &[
                format!("{:.1}", r.size_mm.x),
                format!("{:.1}", r.size_mm.y),
                format!("{:.1}", r.size_mm.z),
            ],
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

fn preview_column(ui: &mut egui::Ui, st: &mut StepImport) {
    ui.heading(tr("Anteprima 3D"));
    let side = ui.available_width().min(ui.available_height() - 90.0).max(160.0);
    let (rect, r) = ui.allocate_exact_size(egui::vec2(ui.available_width(), side), egui::Sense::drag());
    ui.painter().rect_filled(rect, 3.0, egui::Color32::from_gray(24));
    if r.dragged() {
        let d = r.drag_delta();
        st.yaw += d.x * 0.01;
        st.pitch = (st.pitch + d.y * 0.01).clamp(-1.5, 1.5);
    }
    if r.hovered() {
        let wheel = ui.input(|i| i.smooth_scroll_delta.y);
        st.zoom = (st.zoom * (1.0 + wheel * 0.002)).clamp(0.2, 10.0);
    }
    match &st.result {
        Some(c) => draw_model(ui, rect, c, st.wire, st.yaw, st.pitch, st.zoom, st.light),
        None => {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                tr("Nessuna anteprima"),
                egui::FontId::proportional(13.0),
                ui.visuals().weak_text_color(),
            );
        }
    }
    ui.horizontal(|ui| {
        ui.selectable_value(&mut st.wire, false, tr("Ombreggiata"));
        ui.selectable_value(&mut st.wire, true, tr("Fil di ferro"));
        ui.add(egui::Slider::new(&mut st.light, 0.0..=1.0).show_value(false))
            .on_hover_text(tr("Direzione della luce dell'anteprima"));
    });
    if let Some(c) = &st.result {
        ui.label(tf(
            "{} triangoli, {} vertici, {} s",
            &[c.triangles.to_string(), c.vertices.to_string(), format!("{:.1}", c.seconds)],
        ));
        if c.tris.len() < c.triangles {
            ui.label(
                egui::RichText::new(tf(
                    "Anteprima con {} triangoli: il modello li ha tutti",
                    &[c.tris.len().to_string()],
                ))
                .weak(),
            );
        }
        if c.made_with != st.options() {
            ui.colored_label(style::WARN, tr("Parametri cambiati: premi Converti per aggiornare"));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_model(ui: &egui::Ui, rect: egui::Rect, c: &Converted, wire: bool, yaw: f32, pitch: f32, zoom: f32, light: f32) {
    let rot = Mat4::from_rotation_x(pitch) * Mat4::from_rotation_y(yaw);
    let scale = rect.width().min(rect.height()) * 0.72 * zoom;
    let to_screen = |p: Vec3| {
        // Prospettiva leggera: profondità sufficiente a capire la forma.
        let k = 2.6 / (2.6 - p.z);
        egui::pos2(rect.center().x + p.x * scale * k, rect.center().y - p.y * scale * k)
    };
    let painter = ui.painter_at(rect);
    let a = light * std::f32::consts::PI - std::f32::consts::FRAC_PI_2;
    let sun = Vec3::new(a.sin(), 0.6, a.cos()).normalize();
    if wire {
        let stroke = egui::Stroke::new(0.6, egui::Color32::from_gray(190));
        let every = (c.tris.len() / WIRE_TRIANGLES).max(1);
        for (v, ..) in c.tris.iter().step_by(every) {
            let p = v.map(|q| to_screen(rot.transform_point3(q)));
            painter.line_segment([p[0], p[1]], stroke);
            painter.line_segment([p[1], p[2]], stroke);
            painter.line_segment([p[2], p[0]], stroke);
        }
        return;
    }
    // Dal più lontano al più vicino: l'interfaccia non ha un buffer di profondità.
    let mut order: Vec<(f32, usize)> = c
        .tris
        .iter()
        .enumerate()
        .map(|(i, (v, ..))| {
            (
                (rot.transform_point3(v[0]).z + rot.transform_point3(v[1]).z + rot.transform_point3(v[2]).z) / 3.0,
                i,
            )
        })
        .collect();
    order.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut mesh = egui::Mesh::default();
    for (_, i) in order {
        let (v, n, color) = &c.tris[i];
        let normal = rot.transform_vector3(*n);
        if normal.z <= 0.0 {
            continue;
        }
        let shade = 0.25 + 0.75 * normal.dot(sun).max(0.0);
        let [r, g, b] = color.map(|x| (x.clamp(0.0, 1.0) * shade * 255.0) as u8);
        let col = egui::Color32::from_rgb(r, g, b);
        let base = mesh.vertices.len() as u32;
        for q in v {
            mesh.colored_vertex(to_screen(rot.transform_point3(*q)), col);
        }
        mesh.add_triangle(base, base + 1, base + 2);
    }
    painter.add(egui::Shape::mesh(mesh));
}
