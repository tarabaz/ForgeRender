//! Finestra "Libreria": materiali PBR con texture e HDRI.
//!
//! - **La mia libreria**: materiali in `Libreria/Materiali` con miniatura; un clic
//!   li applica alla selezione. Import da cartella o da zip.
//! - **Online**: ricerca su ambientCG e Poly Haven (CC0), miniature prima del
//!   download, scelta della risoluzione; i materiali finiscono nella libreria e
//!   le HDRI in `Libreria/HDRI`.
//!
//! Tutte le operazioni lente (download, decodifica delle miniature, anteprime
//! HDRI) girano su thread separati e comunicano con un canale.

use crate::app::ForgeApp;
use crate::ui::style;
use eframe::egui;
use forge_library::online::{self, AssetKind, OnlineAsset, Provider};
use forge_library::store::{self, LibraryMaterial};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Mine,
    Online,
}

enum Msg {
    Thumb(String, Result<egui::ColorImage, String>),
    Search(u64, Result<Vec<OnlineAsset>, String>),
    Material(Result<Box<LibraryMaterial>, String>),
    Hdri(Result<PathBuf, String>),
    Progress(String),
}

enum Thumb {
    Loading,
    Ready(egui::TextureHandle),
    Failed,
}

pub struct LibraryUi {
    pub open: bool,
    tab: Tab,
    provider: Provider,
    kind: AssetKind,
    query: String,
    my_query: String,
    resolution: usize,
    items: Option<Vec<LibraryMaterial>>,
    hdris: Option<Vec<PathBuf>>,
    results: Vec<OnlineAsset>,
    /// Il prossimo caricamento riscarica il catalogo dal sito invece di usare la cache.
    refresh_catalog: bool,
    searching: Option<u64>,
    search_id: u64,
    busy: Option<String>,
    status: Option<(String, bool)>,
    thumbs: HashMap<String, Thumb>,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
}

impl Default for LibraryUi {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            open: false,
            tab: Tab::Mine,
            provider: Provider::AmbientCg,
            kind: AssetKind::Material,
            query: String::new(),
            my_query: String::new(),
            resolution: 1,
            items: None,
            hdris: None,
            results: Vec::new(),
            refresh_catalog: false,
            searching: None,
            search_id: 0,
            busy: None,
            status: None,
            thumbs: HashMap::new(),
            tx,
            rx,
        }
    }
}

/// Apre una cartella nel file manager del sistema.
pub fn open_folder(dir: &Path) {
    let _ = std::fs::create_dir_all(dir);
    let cmd = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    if let Err(e) = std::process::Command::new(cmd).arg(dir).spawn() {
        tracing::warn!("apertura di {} fallita: {e}", dir.display());
    }
}

/// Anteprima di una HDRI: equirettangolare ridotta con tone mapping, salvata in cache.
fn hdri_preview(path: &Path) -> anyhow::Result<image::RgbaImage> {
    let meta = std::fs::metadata(path)?;
    let stamp = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let key = format!(
        "{}_{}_{stamp}.png",
        store::folder_name(&path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()),
        meta.len()
    );
    let cache = forge_library::paths::cache_dir().join("anteprime_hdri").join(key);
    if let Ok(img) = image::open(&cache) {
        return Ok(img.to_rgba8());
    }
    let hdr = image::open(path)?.to_rgb32f();
    let small = image::imageops::resize(&hdr, 256, 128, image::imageops::FilterType::Triangle);
    // Esposizione automatica sulla luminanza media logaritmica, poi ACES e sRGB.
    let mut log_sum = 0.0f64;
    for p in small.pixels() {
        let y = 0.2126 * p.0[0] + 0.7152 * p.0[1] + 0.0722 * p.0[2];
        log_sum += (y.max(1e-4) as f64).ln();
    }
    let avg = (log_sum / (256.0 * 128.0)).exp() as f32;
    let exposure = 0.18 / avg.max(1e-4);
    let lin: Vec<[f32; 3]> = small.pixels().map(|p| p.0.map(|c| c * exposure)).collect();
    let rgba = forge_render_core::tonemap_to_rgba8(&lin, forge_render_core::ToneMapping::AcesFilmic, 0.0);
    let img = image::RgbaImage::from_raw(256, 128, rgba).ok_or_else(|| anyhow::anyhow!("anteprima non valida"))?;
    if let Some(d) = cache.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = img.save(&cache);
    Ok(img)
}

fn to_color_image(img: image::RgbaImage) -> egui::ColorImage {
    egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw())
}

impl LibraryUi {
    /// Apre la finestra sulla ricerca di HDRI online (Poly Haven).
    pub fn open_hdri_tab(&mut self) {
        self.open = true;
        self.tab = Tab::Online;
        self.provider = Provider::PolyHaven;
        self.kind = AssetKind::Hdri;
    }

    /// HDRI della cartella della libreria (elenco aggiornato con «Aggiorna» o dopo un download).
    pub fn hdri_list(&mut self) -> Vec<PathBuf> {
        self.hdris.get_or_insert_with(forge_library::paths::list_hdri).clone()
    }

    pub fn refresh(&mut self) {
        self.items = None;
        self.hdris = None;
    }

    fn materials(&mut self) -> &[LibraryMaterial] {
        self.items.get_or_insert_with(store::list)
    }

    /// Versione salvata nella libreria del materiale `id` (`lib:…`), se c'è.
    pub fn saved_version(&mut self, id: &forge_scene::MaterialId) -> Option<&LibraryMaterial> {
        self.materials().iter().find(|m| &m.material.id == id)
    }

    /// Dopo un salvataggio: elenco e miniatura del materiale si ricaricano.
    pub fn material_saved(&mut self, m: &LibraryMaterial) {
        self.items = None;
        self.thumbs.remove(&format!("mat:{}", m.dir.display()));
    }

    /// Texture della miniatura `key`; avvia il caricamento in background se serve.
    fn thumb(
        &mut self,
        ctx: &egui::Context,
        key: &str,
        load: impl FnOnce() -> anyhow::Result<egui::ColorImage> + Send + 'static,
    ) -> Option<egui::TextureHandle> {
        match self.thumbs.get(key) {
            Some(Thumb::Ready(t)) => Some(t.clone()),
            Some(_) => None,
            None => {
                self.thumbs.insert(key.to_string(), Thumb::Loading);
                let tx = self.tx.clone();
                let key = key.to_string();
                let ctx = ctx.clone();
                std::thread::spawn(move || {
                    let r = load().map_err(|e| format!("{e:#}"));
                    let _ = tx.send(Msg::Thumb(key, r));
                    ctx.request_repaint();
                });
                None
            }
        }
    }

    /// Miniatura di una HDRI locale (anche per la scheda Ambiente).
    pub fn hdri_thumb(&mut self, ctx: &egui::Context, path: &Path) -> Option<egui::TextureHandle> {
        let p = path.to_path_buf();
        self.thumb(ctx, &format!("hdri:{}", path.display()), move || {
            Ok(to_color_image(hdri_preview(&p)?))
        })
    }

    fn material_thumb(&mut self, ctx: &egui::Context, m: &LibraryMaterial) -> Option<egui::TextureHandle> {
        let dir = m.dir.clone();
        let mat = m.material.clone();
        self.thumb(ctx, &format!("mat:{}", m.dir.display()), move || {
            let path = dir.join(store::THUMBNAIL_FILE);
            if let Ok(img) = image::open(&path) {
                return Ok(to_color_image(img.to_rgba8()));
            }
            // Prima volta: anteprima renderizzata con il path tracer e salvata nella cartella.
            let rgba = crate::ui::central::render_preview(&mat);
            let n = (rgba.len() / 4) as f64;
            let side = n.sqrt() as u32;
            let _ = store::save_thumbnail(&dir, side, side, &rgba);
            Ok(egui::ColorImage::from_rgba_unmultiplied([side as usize, side as usize], &rgba))
        })
    }

    fn online_thumb(&mut self, ctx: &egui::Context, a: &OnlineAsset) -> Option<egui::TextureHandle> {
        let asset = a.clone();
        self.thumb(ctx, &format!("web:{:?}:{}", a.provider, a.id), move || {
            let bytes = online::thumbnail(&asset)?;
            Ok(to_color_image(image::load_from_memory(&bytes)?.to_rgba8()))
        })
    }

    fn spawn(&mut self, ctx: &egui::Context, label: String, job: impl FnOnce(&Sender<Msg>) -> Msg + Send + 'static) {
        self.busy = Some(label);
        self.status = None;
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let msg = job(&tx);
            let _ = tx.send(msg);
            ctx.request_repaint();
        });
    }

    fn search(&mut self, ctx: &egui::Context) {
        self.search_id += 1;
        let id = self.search_id;
        self.searching = Some(id);
        self.results.clear();
        let (p, k, q) = (self.provider, self.kind, self.query.clone());
        let refresh = std::mem::take(&mut self.refresh_catalog);
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            // Catalogo completo (in cache su disco), filtrato localmente.
            let r = online::catalog(p, k, refresh)
                .map(|all| online::filter(&all, &q))
                .map_err(|e| format!("{e:#}"));
            let _ = tx.send(Msg::Search(id, r));
            ctx.request_repaint();
        });
    }
}

/// Riceve i risultati delle operazioni in background. Restituisce un materiale
/// appena scaricato/importato da applicare, o una HDRI da usare.
fn poll(app: &mut ForgeApp, ctx: &egui::Context) {
    while let Ok(msg) = app.library.rx.try_recv() {
        let lib = &mut app.library;
        match msg {
            Msg::Thumb(key, Ok(img)) => {
                let tex = ctx.load_texture(&key, img, egui::TextureOptions::LINEAR);
                lib.thumbs.insert(key, Thumb::Ready(tex));
            }
            Msg::Thumb(key, Err(e)) => {
                tracing::debug!("miniatura {key} non disponibile: {e}");
                lib.thumbs.insert(key, Thumb::Failed);
            }
            Msg::Search(id, r) if Some(id) == lib.searching => {
                lib.searching = None;
                match r {
                    Ok(v) => {
                        if v.is_empty() {
                            lib.status = Some(("Nessun risultato.".into(), false));
                        }
                        lib.results = v;
                    }
                    Err(e) => lib.status = Some((format!("Ricerca non riuscita: {e}"), true)),
                }
            }
            Msg::Search(..) => {}
            Msg::Progress(p) => {
                if let Some(b) = &mut lib.busy {
                    *b = format!("{} — {p}", b.split(" — ").next().unwrap_or_default());
                }
            }
            Msg::Material(r) => {
                lib.busy = None;
                lib.items = None;
                match r {
                    Ok(m) => {
                        lib.status = Some((format!("«{}» aggiunto alla libreria.", m.material.name), false));
                        lib.tab = Tab::Mine;
                        apply_material(app, &m);
                    }
                    Err(e) => lib.status = Some((e, true)),
                }
            }
            Msg::Hdri(r) => {
                lib.busy = None;
                lib.hdris = None;
                match r {
                    Ok(p) => {
                        lib.status = Some((format!("HDRI salvata in {}", p.display()), false));
                        crate::ui::inspector::set_hdri(app, p);
                        app.shading_changed();
                    }
                    Err(e) => lib.status = Some((e, true)),
                }
            }
        }
    }
}

/// Aggiunge il materiale della libreria al progetto e lo assegna alla selezione. Se il
/// progetto lo contiene già, si tengono le sue impostazioni (anche quelle non salvate
/// nella libreria): si ripristinano con "Ripristina dalla libreria".
pub fn apply_material(app: &mut ForgeApp, m: &LibraryMaterial) {
    let id = m.material.id.clone();
    if app.scene.materials.get(&id).is_none() {
        app.scene.materials.upsert(m.material.clone());
    }
    for &i in &app.selection {
        app.scene.assignments.insert(app.scene.nodes[i].id.clone(), id.clone());
    }
    app.selected_material = id;
    app.appearance_changed();
}

const CARD: f32 = 112.0;

/// Miniatura con titolo, a larghezza fissa (il testo lungo viene troncato).
pub fn thumb_card(
    ui: &mut egui::Ui,
    tex: Option<egui::TextureHandle>,
    size: egui::Vec2,
    title: &str,
    subtitle: &str,
    selected: bool,
) -> egui::Response {
    let h = size.y + if subtitle.is_empty() { 18.0 } else { 34.0 };
    ui.allocate_ui_with_layout(egui::vec2(size.x, h), egui::Layout::top_down(egui::Align::Min), |ui| {
        ui.set_width(size.x);
        let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
        match tex {
            Some(t) => {
                ui.painter().image(
                    t.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
            None => {
                ui.painter().rect_filled(rect, 4.0, egui::Color32::from_gray(40));
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "anteprima…",
                    egui::FontId::proportional(11.0),
                    egui::Color32::GRAY,
                );
            }
        }
        if selected || resp.hovered() {
            let c = if selected { style::ACCENT } else { egui::Color32::from_gray(200) };
            ui.painter()
                .rect_stroke(rect, 4.0, egui::Stroke::new(2.0, c), egui::StrokeKind::Inside);
        }
        ui.add(egui::Label::new(egui::RichText::new(title).small()).truncate());
        if !subtitle.is_empty() {
            ui.add(egui::Label::new(egui::RichText::new(subtitle).small().weak()).truncate());
        }
        resp
    })
    .inner
}

/// Griglia di miniature a colonne fisse (non allarga mai il contenitore).
fn grid_cols(ui: &egui::Ui, cell: f32) -> usize {
    let gap = ui.spacing().item_spacing.x;
    (((ui.available_width() + gap) / (cell + gap)).floor() as usize).max(1)
}

/// Finestra della libreria.
pub fn window(app: &mut ForgeApp, ctx: &egui::Context) {
    poll(app, ctx);
    if !app.library.open {
        return;
    }
    let mut open = true;
    egui::Window::new("Libreria materiali e HDRI")
        .open(&mut open)
        .default_size([860.0, 600.0])
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut app.library.tab, Tab::Mine, "La mia libreria");
                ui.selectable_value(&mut app.library.tab, Tab::Online, "Online (ambientCG, Poly Haven)");
                ui.separator();
                if ui
                    .button("Apri cartella")
                    .on_hover_text(forge_library::paths::root().display().to_string())
                    .clicked()
                {
                    open_folder(&forge_library::paths::root());
                }
                if ui.button("Aggiorna").clicked() {
                    app.library.refresh();
                    app.library.thumbs.retain(|k, _| k.starts_with("web:"));
                }
            });
            if let Some(b) = &app.library.busy {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(b);
                });
            }
            if let Some((s, err)) = &app.library.status {
                ui.colored_label(
                    if *err {
                        style::WARN
                    } else {
                        egui::Color32::from_rgb(140, 200, 140)
                    },
                    s,
                );
            }
            ui.separator();
            match app.library.tab {
                Tab::Mine => mine(app, ui),
                Tab::Online => online_tab(app, ui),
            }
        });
    app.library.open = open;
}

fn mine(app: &mut ForgeApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut app.library.my_query)
                .hint_text("cerca…")
                .desired_width(160.0),
        );
        if ui
            .button("Importa cartella…")
            .on_hover_text("Una cartella con le mappe di un materiale (*_Color, *_Roughness, *_Normal…)")
            .clicked()
        {
            if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                app.library.spawn(&ctx, "Importazione".into(), move |_| {
                    Msg::Material(
                        store::import_folder(&dir, None, None, "importato da cartella")
                            .map(Box::new)
                            .map_err(|e| format!("{e:#}")),
                    )
                });
            }
        }
        if ui
            .button("Importa zip…")
            .on_hover_text("Archivi scaricati da ambientCG, Poly Haven, ecc.")
            .clicked()
        {
            if let Some(files) = rfd::FileDialog::new().add_filter("Zip", &["zip"]).pick_files() {
                app.library.spawn(&ctx, "Importazione".into(), move |tx| {
                    let mut last = Err("nessun file".to_string());
                    for (i, f) in files.iter().enumerate() {
                        let _ = tx.send(Msg::Progress(format!("{}/{}", i + 1, files.len())));
                        last = store::import_zip(f, None, None, "importato da zip").map_err(|e| format!("{}: {e:#}", f.display()));
                    }
                    Msg::Material(last.map(Box::new))
                });
            }
        }
        let n = app.selection.len();
        ui.label(
            egui::RichText::new(if n == 0 {
                "Clic su un materiale: lo aggiunge al progetto".to_string()
            } else {
                format!("Clic su un materiale: lo applica ai {n} componenti selezionati")
            })
            .weak()
            .small(),
        );
    });
    let q = app.library.my_query.to_lowercase();
    let items: Vec<LibraryMaterial> = app
        .library
        .materials()
        .iter()
        .filter(|m| q.is_empty() || m.material.name.to_lowercase().contains(&q) || m.source.to_lowercase().contains(&q))
        .cloned()
        .collect();
    if items.is_empty() {
        ui.add_space(20.0);
        ui.label("La libreria è vuota: scarica un materiale dalla scheda Online oppure importa una cartella o uno zip.");
        ui.label(
            egui::RichText::new(forge_library::paths::materials_dir().display().to_string())
                .weak()
                .small(),
        );
        return;
    }
    let mut apply = None;
    let mut delete = None;
    let cols = grid_cols(ui, CARD);
    egui::ScrollArea::vertical().id_salt("lib-mine").show(ui, |ui| {
        for row in items.chunks(cols) {
            ui.horizontal(|ui| {
                for m in row {
                    let tex = app.library.material_thumb(&ctx, m);
                    let maps = m.material.texture_slots().iter().filter(|s| s.is_some()).count();
                    let r = thumb_card(ui, tex, egui::vec2(CARD, CARD), &m.material.name, &format!("{maps} mappe"), false);
                    let r = r.on_hover_text(format!("{}\n{}", m.source, m.dir.display()));
                    if r.clicked() {
                        apply = Some(m.clone());
                    }
                    r.context_menu(|ui| {
                        if ui.button("Apri cartella").clicked() {
                            open_folder(&m.dir);
                            ui.close();
                        }
                        if ui.button("Elimina dalla libreria").clicked() {
                            delete = Some(m.clone());
                            ui.close();
                        }
                    });
                }
            });
        }
    });
    if let Some(m) = apply {
        let kept = app.scene.materials.get(&m.material.id).is_some_and(|p| *p != m.material);
        apply_material(app, &m);
        app.library.status = Some((
            if kept {
                format!(
                    "«{}» applicato con le impostazioni del progetto (non ancora salvate nella libreria).",
                    m.material.name
                )
            } else {
                format!("«{}» applicato.", m.material.name)
            },
            false,
        ));
    }
    if let Some(m) = delete {
        match store::delete(&m) {
            Ok(()) => app.library.status = Some((format!("«{}» eliminato.", m.material.name), false)),
            Err(e) => app.library.status = Some((format!("{e:#}"), true)),
        }
        app.library.items = None;
    }
}

fn online_tab(app: &mut ForgeApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let lib = &mut app.library;
    let mut do_search = false;
    ui.horizontal(|ui| {
        for p in [Provider::AmbientCg, Provider::PolyHaven] {
            if ui.selectable_value(&mut lib.provider, p, p.label()).changed() {
                do_search = true;
            }
        }
        ui.separator();
        do_search |= ui.selectable_value(&mut lib.kind, AssetKind::Material, "Materiali").changed();
        do_search |= ui.selectable_value(&mut lib.kind, AssetKind::Hdri, "HDRI").changed();
        ui.separator();
        let r = ui.add(
            egui::TextEdit::singleline(&mut lib.query)
                .hint_text("es. wood, metal, studio…")
                .desired_width(170.0),
        );
        do_search |= r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        do_search |= ui.button("Cerca").clicked();
        if ui
            .button("Aggiorna catalogo")
            .on_hover_text("Riscarica l'elenco completo dal sito (si aggiorna da solo una volta a settimana)")
            .clicked()
        {
            lib.refresh_catalog = true;
            do_search = true;
        }
        egui::ComboBox::from_id_salt("lib-res")
            .selected_text(online::RESOLUTIONS[lib.resolution])
            .width(60.0)
            .show_ui(ui, |ui| {
                for (i, r) in online::RESOLUTIONS.iter().enumerate() {
                    ui.selectable_value(&mut lib.resolution, i, *r);
                }
            })
            .response
            .on_hover_text("Risoluzione del download: 1K per prove veloci, 2K–4K per i render finali, 8K solo per primi piani");
    });
    ui.label(
        egui::RichText::new(
            "Contenuti CC0: liberi anche per uso commerciale, senza obbligo di citazione. Le parole di ricerca sono in inglese.",
        )
        .weak()
        .small(),
    );
    if do_search || (lib.results.is_empty() && lib.searching.is_none() && lib.status.is_none()) {
        lib.search(&ctx);
    }
    if lib.searching.is_some() {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Ricerca in corso…");
        });
        return;
    }
    let results = lib.results.clone();
    let res = online::RESOLUTIONS[lib.resolution].to_string();
    // Già scaricati (a qualsiasi risoluzione): segnati con ✓.
    let local: std::collections::HashSet<String> = lib
        .materials()
        .iter()
        .filter_map(|m| m.source.rsplit_once(" (").map(|(k, _)| k.to_string()))
        .collect();
    let hdris = lib.hdri_list();
    let mut download = None;
    let size = if lib.kind == AssetKind::Hdri {
        egui::vec2(CARD * 2.0, CARD)
    } else {
        egui::vec2(CARD, CARD)
    };
    ui.label(
        egui::RichText::new(format!(
            "{} {} nel catalogo{}",
            results.len(),
            if lib.kind == AssetKind::Hdri { "HDRI" } else { "materiali" },
            if lib.query.trim().is_empty() {
                String::new()
            } else {
                format!(" per «{}»", lib.query.trim())
            }
        ))
        .small(),
    );
    let cols = grid_cols(ui, size.x);
    // Si disegnano (e si scaricano le miniature) solo delle righe visibili.
    let row_h = size.y + 34.0 + ui.spacing().item_spacing.y;
    let rows = results.len().div_ceil(cols);
    egui::ScrollArea::vertical()
        .id_salt("lib-online")
        .show_rows(ui, row_h, rows, |ui, range| {
            for row in range {
                ui.horizontal(|ui| {
                    for a in &results[row * cols..((row + 1) * cols).min(results.len())] {
                        let tex = app.library.online_thumb(&ctx, a);
                        let have = match a.kind {
                            AssetKind::Material => online::already_downloaded(a, &local),
                            AssetKind::Hdri => online::hdri_downloaded(a, &hdris),
                        };
                        let mut sub = match a.size_m {
                            Some(s) => format!("{} · {:.2} m", a.category, s),
                            None => a.category.clone(),
                        };
                        if have {
                            sub = format!("✓ in libreria · {sub}");
                        }
                        let r = thumb_card(ui, tex, size, &a.name, &sub, have)
                            .on_hover_text(format!("{}\nClic: scarica a {res}", a.tags.join(", ")));
                        if r.clicked() && app.library.busy.is_none() {
                            download = Some(a.clone());
                        }
                    }
                });
            }
        });
    if let Some(a) = download {
        let label = format!("Download di «{}» ({res})", a.name);
        app.library.spawn(&ctx, label, move |tx| {
            let progress = |s: &str| {
                let _ = tx.send(Msg::Progress(s.to_string()));
            };
            match a.kind {
                AssetKind::Material => Msg::Material(
                    online::download_material(&a, &res, &progress)
                        .map(Box::new)
                        .map_err(|e| format!("{e:#}")),
                ),
                AssetKind::Hdri => Msg::Hdri(online::download_hdri(&a, &res, &progress).map_err(|e| format!("{e:#}"))),
            }
        });
    }
}
