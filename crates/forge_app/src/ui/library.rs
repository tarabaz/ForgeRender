//! Finestra "Libreria": materiali PBR con texture e HDRI.
//!
//! - **Materiali**: in un solo elenco la **libreria ufficiale** (i materiali predefiniti del
//!   programma, anche quelli con le mappe) e gli **scaricati** (`Libreria/Materiali`: ambientCG,
//!   Poly Haven, cartelle e zip importati), con miniatura; un clic li applica alla selezione.
//! - **Online**: ricerca su ambientCG e Poly Haven (CC0), miniature prima del
//!   download, scelta della risoluzione; i materiali finiscono nella libreria e
//!   le HDRI in `Libreria/HDRI`.
//!
//! Tutte le operazioni lente (download, decodifica delle miniature, anteprime
//! HDRI) girano su thread separati e comunicano con un canale.

use crate::app::ForgeApp;
use crate::i18n::{tf, tr, tr_dyn};
use crate::ui::style;
use eframe::egui;
use forge_library::categories;
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
    /// Esito di "Aggiorna categorie".
    Categories(Result<String, String>),
    Progress(String),
}

/// Filtro per categoria della scheda "Materiali".
#[derive(Clone, Debug, PartialEq, Eq)]
enum CategoryFilter {
    /// Libreria ufficiale (materiali predefiniti): tutte le categorie o una.
    Official(Option<String>),
    /// Scaricati: tutti.
    All,
    /// Categoria del sito, com'è scritta in `materiale.json` (in minuscolo).
    Is(String),
    /// Materiali senza categorie (importati da cartella o zip, o non ancora aggiornati).
    None,
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
    category: CategoryFilter,
    /// Lato delle anteprime della scheda "La mia libreria" (dalle impostazioni).
    card: f32,
    /// Risoluzione minima delle miniature (Impostazioni → Anteprime dei materiali).
    min_tier: u32,
    resolution: usize,
    items: Option<Vec<LibraryMaterial>>,
    /// Materiali predefiniti (libreria ufficiale), letti alla prima apertura.
    presets: Option<Vec<forge_scene::Material>>,
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

impl LibraryUi {
    /// Apre la finestra sulla scheda "La mia libreria" (`online` = false) o "Online".
    pub fn open_on(&mut self, online: bool) {
        self.open = true;
        self.tab = if online { Tab::Online } else { Tab::Mine };
    }
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
            category: CategoryFilter::Official(None),
            card: CARD,
            min_tier: 128,
            resolution: 1,
            items: None,
            presets: None,
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

/// Al massimo due anteprime dei materiali si renderizzano insieme (ognuna usa più thread):
/// con centinaia di materiali visibili non si avviano centinaia di render in parallelo.
fn render_slot() -> impl Drop {
    use std::sync::{Condvar, Mutex};
    static STATE: (Mutex<usize>, Condvar) = (Mutex::new(0), Condvar::new());
    struct Slot;
    impl Drop for Slot {
        fn drop(&mut self) {
            *STATE.0.lock().unwrap() -= 1;
            STATE.1.notify_one();
        }
    }
    let mut n = STATE.0.lock().unwrap();
    while *n >= 2 {
        n = STATE.1.wait(n).unwrap();
    }
    *n += 1;
    Slot
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

    /// Valori proposti dalle impostazioni; con `library_changed` si rilegge la libreria.
    pub fn apply_prefs(&mut self, p: &crate::prefs::Preferences, library_changed: bool) {
        self.resolution = p.download_resolution.min(online::RESOLUTIONS.len() - 1);
        self.card = p.library_card_size as f32;
        self.min_tier = p.preview_resolution;
        self.provider = if p.download_provider == "polyhaven" {
            Provider::PolyHaven
        } else {
            Provider::AmbientCg
        };
        if library_changed {
            self.refresh();
            self.thumbs.retain(|k, _| k.starts_with("web:"));
            self.results.clear();
        }
    }

    /// Dimentica le miniature dei materiali caricate (dopo "Svuota cache anteprime" o un
    /// cambio delle impostazioni): si ricaricano o si rifanno alla prossima visualizzazione.
    pub fn forget_thumbnails(&mut self) {
        self.thumbs.retain(|k, _| k.starts_with("web:") || k.starts_with("hdri:"));
    }

    pub fn refresh(&mut self) {
        self.items = None;
        self.hdris = None;
    }

    pub fn materials(&mut self) -> &[LibraryMaterial] {
        self.items.get_or_insert_with(store::list)
    }

    /// Versione salvata nella libreria del materiale `id` (`lib:…`), se c'è.
    pub fn saved_version(&mut self, id: &forge_scene::MaterialId) -> Option<&LibraryMaterial> {
        self.materials().iter().find(|m| &m.material.id == id)
    }

    /// Dopo un salvataggio: elenco e miniatura del materiale si ricaricano.
    pub fn material_saved(&mut self, m: &LibraryMaterial) {
        self.items = None;
        let prefix = format!("mat:{}:", m.dir.display());
        self.thumbs.retain(|k, _| !k.starts_with(&prefix));
    }

    /// Miniatura già pronta (senza avviarne il caricamento).
    fn ready_thumb(&self, key: &str) -> Option<egui::TextureHandle> {
        match self.thumbs.get(key) {
            Some(Thumb::Ready(t)) => Some(t.clone()),
            _ => None,
        }
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

    /// Miniatura del materiale di lato `size` (uno di [`store::THUMBNAIL_SIZES`]). Finché non è
    /// pronta si mostra un'altra già caricata dello stesso materiale.
    fn material_thumb(&mut self, ctx: &egui::Context, m: &LibraryMaterial, size: u32) -> Option<egui::TextureHandle> {
        let dir = m.dir.clone();
        let mat = m.material.clone();
        let key = format!("mat:{}:{size}", m.dir.display());
        let tex = self.thumb(ctx, &key, move || {
            let path = dir.join(store::thumbnail_file(size));
            if let Ok(img) = image::open(&path) {
                return Ok(to_color_image(img.to_rgba8()));
            }
            // Prima volta: anteprima renderizzata con il path tracer e salvata nella cartella.
            let _slot = render_slot();
            let rgba = crate::ui::central::render_preview(&mat, size);
            let side = ((rgba.len() / 4) as f64).sqrt() as u32;
            let _ = store::save_thumbnail(&dir, side, side, &rgba);
            Ok(egui::ColorImage::from_rgba_unmultiplied([side as usize, side as usize], &rgba))
        });
        tex.or_else(|| {
            store::THUMBNAIL_SIZES
                .iter()
                .rev()
                .find_map(|s| self.ready_thumb(&format!("mat:{}:{s}", m.dir.display())))
        })
    }

    /// Materiali predefiniti (libreria ufficiale), nell'ordine del programma.
    fn presets(&mut self) -> &[forge_scene::Material] {
        self.presets.get_or_insert_with(forge_scene::material::presets)
    }

    /// Miniatura di un materiale predefinito: renderizzata la prima volta e salvata nella cache
    /// della libreria, con un'impronta dei valori (cambiando il materiale si rifà).
    pub(crate) fn preset_thumb(&mut self, ctx: &egui::Context, m: &forge_scene::Material, size: u32) -> Option<egui::TextureHandle> {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        serde_json::to_string(m).unwrap_or_default().hash(&mut h);
        // Anche i campioni: cambiandoli nelle impostazioni le anteprime si rifanno.
        crate::ui::central::preview_samples().hash(&mut h);
        crate::ui::central::preview_object_cm().hash(&mut h);
        let hash = h.finish();
        let key = |s: u32| format!("preset:{}:{s}:{hash:016x}", m.id.0);
        let file = store::preset_thumbnail_dir().join(format!("{}-{size}-{hash:016x}.png", m.id.0));
        let mat = m.clone();
        let tex = self.thumb(ctx, &key(size), move || {
            if let Ok(img) = image::open(&file) {
                return Ok(to_color_image(img.to_rgba8()));
            }
            let _slot = render_slot();
            let rgba = crate::ui::central::render_preview(&mat, size);
            let side = ((rgba.len() / 4) as f64).sqrt() as u32;
            if let (Some(dir), Some(img)) = (file.parent(), image::RgbaImage::from_raw(side, side, rgba.clone())) {
                let _ = std::fs::create_dir_all(dir);
                let _ = img.save(&file);
            }
            Ok(egui::ColorImage::from_rgba_unmultiplied([side as usize, side as usize], &rgba))
        });
        tex.or_else(|| store::THUMBNAIL_SIZES.iter().rev().find_map(|&s| self.ready_thumb(&key(s))))
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
                            lib.status = Some((tr("Nessun risultato.").into(), false));
                        }
                        lib.results = v;
                    }
                    Err(e) => lib.status = Some((tf("Ricerca non riuscita: {e}", std::slice::from_ref(&e)), true)),
                }
            }
            Msg::Search(..) => {}
            Msg::Progress(p) => {
                if let Some(b) = &mut lib.busy {
                    *b = tf("{} — {p}", &[b.split(" — ").next().unwrap_or_default().to_string(), p.to_string()]);
                }
            }
            Msg::Material(r) => {
                lib.busy = None;
                lib.items = None;
                match r {
                    Ok(m) => {
                        let mut msg = tf("«{}» aggiunto alla libreria.", std::slice::from_ref(&m.material.name));
                        if !m.notes.is_empty() {
                            msg += &tf(" Attenzione: {}.", &[m.notes.join("; ").to_string()]);
                        }
                        lib.status = Some((msg, !m.notes.is_empty()));
                        lib.tab = Tab::Mine;
                        apply_material(app, &m);
                    }
                    Err(e) => lib.status = Some((e, true)),
                }
            }
            Msg::Categories(r) => {
                lib.busy = None;
                lib.items = None;
                lib.status = Some(match r {
                    Ok(s) => (s, false),
                    Err(e) => (e, true),
                });
            }
            Msg::Hdri(r) => {
                lib.busy = None;
                lib.hdris = None;
                match r {
                    Ok(p) => {
                        lib.status = Some((tf("HDRI salvata in {}", &[format!("{}", p.display())]), false));
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
                    tr("anteprima…"),
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
    egui::Window::new(tr("Libreria materiali e HDRI"))
        .id(egui::Id::NULL.with("win-libreria-materiali-e-hdri"))
        .open(&mut open)
        .default_size([860.0, 600.0])
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut app.library.tab, Tab::Mine, tr("Materiali"));
                ui.selectable_value(&mut app.library.tab, Tab::Online, tr("Online (ambientCG, Poly Haven)"));
                ui.separator();
                if ui
                    .button(tr("Apri cartella"))
                    .on_hover_text(forge_library::paths::root().display().to_string())
                    .clicked()
                {
                    open_folder(&forge_library::paths::root());
                }
                if ui.button(tr("Aggiorna")).clicked() {
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
                .hint_text(tr("cerca (nome, categoria, tag)…"))
                .desired_width(190.0),
        )
        .on_hover_text(tr(
            "Cerca nel nome, nella provenienza, nelle categorie e nei tag (in inglese, es. oak, rusty, tiles)",
        ));
        if ui
            .button(tr("Importa cartella…"))
            .on_hover_text(tr("Una cartella con le mappe di un materiale (*_Color, *_Roughness, *_Normal…)"))
            .clicked()
        {
            if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                app.library.spawn(&ctx, tr("Importazione").into(), move |_| {
                    Msg::Material(
                        store::import_folder(&dir, None, None, tr("importato da cartella"))
                            .map(Box::new)
                            .map_err(|e| format!("{e:#}")),
                    )
                });
            }
        }
        if ui
            .button(tr("Importa da Blender…"))
            .on_hover_text(tr(
                "Un materiale da un file .blend (anche BlenderKit), con anteprima prima di salvarlo",
            ))
            .clicked()
        {
            app.blend_import.open = true;
        }
        if ui
            .button(tr("Importa zip…"))
            .on_hover_text(tr("Archivi scaricati da ambientCG, Poly Haven, ecc."))
            .clicked()
        {
            if let Some(files) = rfd::FileDialog::new().add_filter(tr("Zip"), &["zip"]).pick_files() {
                app.library.spawn(&ctx, tr("Importazione").into(), move |tx| {
                    let mut last = Err(tr("nessun file").to_string());
                    for (i, f) in files.iter().enumerate() {
                        let _ = tx.send(Msg::Progress(format!("{}/{}", i + 1, files.len())));
                        last = store::import_zip(f, None, None, tr("importato da zip"))
                            .map_err(|e| tf("{}: {e:#}", &[format!("{}", f.display()), format!("{:#}", e)]));
                    }
                    Msg::Material(last.map(Box::new))
                });
            }
        }
        let n = app.selection.len();
        ui.label(
            egui::RichText::new(if n == 0 {
                tr("Clic su un materiale: lo aggiunge al progetto").to_string()
            } else {
                tf(
                    "Clic su un materiale: lo applica ai {n} componenti selezionati",
                    &[format!("{}", n)],
                )
            })
            .weak()
            .small(),
        );
    });
    let all: Vec<LibraryMaterial> = app.library.materials().to_vec();
    // Dimensione delle anteprime (salvata nelle impostazioni a fine trascinamento).
    let mut save_card = false;
    let needs_categories = all.iter().any(|m| {
        m.categories.is_empty()
            && m.tags.is_empty()
            && [Provider::AmbientCg, Provider::PolyHaven]
                .iter()
                .any(|p| m.source.starts_with(p.label()))
    });
    ui.horizontal(|ui| {
        let r = ui
            .add(
                egui::Slider::new(&mut app.library.card, 64.0..=512.0)
                    .step_by(8.0)
                    .suffix(tr(" px"))
                    .text("anteprime"),
            )
            .on_hover_text(tr(
                "Dimensione delle anteprime. Oltre i 128 pixel le sfere si renderizzano a 256 o 512 pixel \
                 (la prima volta qualche secondo per materiale, poi restano salvate nella sua cartella).",
            ));
        save_card = r.drag_stopped() || (r.changed() && !r.dragged());
        if needs_categories
            && ui
                .button(tr("Aggiorna categorie"))
                .on_hover_text(tr(
                    "Aggiunge categorie e tag dei cataloghi di ambientCG e Poly Haven ai materiali scaricati che non li hanno",
                ))
                .clicked()
            && app.library.busy.is_none()
        {
            app.library.spawn(&ctx, tr("Aggiornamento delle categorie").into(), |_| {
                let catalogs: Vec<_> = [Provider::AmbientCg, Provider::PolyHaven]
                    .into_iter()
                    .filter_map(|p| online::catalog(p, AssetKind::Material, false).ok().map(|c| (p, c)))
                    .collect();
                if catalogs.is_empty() {
                    return Msg::Categories(Err(tr("Cataloghi non disponibili: controlla la connessione.").into()));
                }
                let r = online::fill_catalog_info(&catalogs);
                let mut s = tf("Categorie aggiunte a {} materiali.", &[format!("{}", r.updated)]);
                if !r.not_found.is_empty() {
                    s += &tf(" Non trovati nel catalogo: {}.", &[format!("{}", r.not_found.len())]);
                }
                Msg::Categories(Ok(s))
            });
        }
    });
    if save_card {
        let p = crate::prefs::Preferences {
            library_card_size: app.library.card.round() as u32,
            ..app.prefs.clone()
        };
        app.apply_prefs(p, false);
    }

    let q = app.library.my_query.trim().to_lowercase();
    let matched: Vec<&LibraryMaterial> = all.iter().filter(|m| matches_query(m, &q)).collect();
    // Conteggi per categoria (sui materiali che rispondono alla ricerca).
    let mut counts: std::collections::BTreeMap<String, usize> = Default::default();
    let mut uncategorized = 0;
    for m in &matched {
        if m.categories.is_empty() {
            uncategorized += 1;
        }
        let mut seen = std::collections::HashSet::new();
        for c in &m.categories {
            let c = c.trim().to_lowercase();
            if seen.insert(c.clone()) {
                *counts.entry(c).or_default() += 1;
            }
        }
    }
    let mut cats: Vec<(String, usize)> = counts.into_iter().collect();
    cats.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| cat_label(&a.0).cmp(&cat_label(&b.0))));
    let (secondary, primary): (Vec<_>, Vec<_>) = cats.into_iter().partition(|(c, _)| categories::is_secondary(c));
    // Libreria ufficiale: predefiniti che rispondono alla ricerca, categorie nell'ordine del programma.
    let presets: Vec<forge_scene::Material> = app.library.presets().to_vec();
    let official: Vec<&forge_scene::Material> = presets
        .iter()
        .filter(|m| {
            q.split_whitespace().all(|w| {
                m.name.to_lowercase().contains(w)
                    || tr_dyn(&m.name).to_lowercase().contains(w)
                    || m.category.to_lowercase().contains(w)
                    || tr_dyn(&m.category).to_lowercase().contains(w)
            })
        })
        .collect();
    let mut official_cats: Vec<(String, usize)> = Vec::new();
    for m in &official {
        match official_cats.iter_mut().find(|e| e.0 == m.category) {
            Some(e) => e.1 += 1,
            None => official_cats.push((m.category.clone(), 1)),
        }
    }
    let mut filter = app.library.category.clone();
    let shown: Vec<LibraryMaterial> = matched
        .iter()
        .filter(|m| match &filter {
            CategoryFilter::Official(_) => false,
            CategoryFilter::All => true,
            CategoryFilter::None => m.categories.is_empty(),
            CategoryFilter::Is(c) => m.categories.iter().any(|x| x.trim().eq_ignore_ascii_case(c)),
        })
        .map(|m| (*m).clone())
        .collect();

    let mut apply = None;
    let mut apply_preset: Option<forge_scene::Material> = None;
    let mut delete = None;
    let card = app.library.card;
    // Miniatura abbastanza grande per i pixel reali dello schermo.
    let need_px = card * ctx.pixels_per_point();
    let tier = store::THUMBNAIL_SIZES
        .iter()
        .copied()
        .find(|s| *s as f32 >= need_px * 0.9)
        .unwrap_or(store::THUMBNAIL_SIZES[store::THUMBNAIL_SIZES.len() - 1])
        .max(app.library.min_tier);
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(170.0);
            egui::ScrollArea::vertical().id_salt("lib-cats").show(ui, |ui| {
                let mut item = |ui: &mut egui::Ui, f: CategoryFilter, text: String| {
                    if ui.selectable_label(filter == f, text).clicked() {
                        filter = f;
                    }
                };
                ui.strong(tr("Libreria ufficiale"));
                item(
                    ui,
                    CategoryFilter::Official(None),
                    tf("Tutti i predefiniti ({})", &[format!("{}", official.len())]),
                );
                for (c, n) in &official_cats {
                    item(ui, CategoryFilter::Official(Some(c.clone())), format!("{} ({n})", tr_dyn(c)));
                }
                ui.separator();
                ui.strong(tr("Scaricati"));
                if all.is_empty() {
                    ui.label(
                        egui::RichText::new(tr(
                            "La libreria è vuota: scarica un materiale dalla scheda Online oppure importa una cartella o uno zip.",
                        ))
                        .weak()
                        .small(),
                    );
                    return;
                }
                item(ui, CategoryFilter::All, tf("Tutti ({})", &[format!("{}", matched.len())]));
                for (c, n) in &primary {
                    item(
                        ui,
                        CategoryFilter::Is(c.clone()),
                        tf("{} ({n})", &[cat_label(c).to_string(), format!("{}", n)]),
                    );
                }
                if uncategorized > 0 {
                    item(
                        ui,
                        CategoryFilter::None,
                        tf("Senza categoria ({uncategorized})", &[format!("{}", uncategorized)]),
                    );
                }
                if !secondary.is_empty() {
                    ui.collapsing(tr("Altre etichette"), |ui| {
                        for (c, n) in &secondary {
                            item(
                                ui,
                                CategoryFilter::Is(c.clone()),
                                tf("{} ({n})", &[cat_label(c).to_string(), format!("{}", n)]),
                            );
                        }
                    });
                }
            });
        });
        ui.separator();
        ui.vertical(|ui| {
            if let CategoryFilter::Official(cat) = &filter {
                let list: Vec<forge_scene::Material> = official
                    .iter()
                    .filter(|m| cat.as_ref().is_none_or(|c| &m.category == c))
                    .map(|m| (*m).clone())
                    .collect();
                if list.is_empty() {
                    ui.label(tr("Nessun materiale con questi filtri."));
                    return;
                }
                let cols = grid_cols(ui, card);
                let row_h = card + 34.0 + ui.spacing().item_spacing.y;
                egui::ScrollArea::vertical()
                    .id_salt("lib-official")
                    .show_rows(ui, row_h, list.len().div_ceil(cols), |ui, range| {
                        for row in range {
                            ui.horizontal(|ui| {
                                for m in &list[row * cols..((row + 1) * cols).min(list.len())] {
                                    preset_card(app, &ctx, ui, m, card, tier, &mut apply_preset);
                                }
                            });
                        }
                    });
                return;
            }
            if shown.is_empty() {
                ui.label(tr("Nessun materiale con questi filtri."));
                return;
            }
            let cols = grid_cols(ui, card);
            // Si disegnano (e si renderizzano le miniature) solo le righe visibili.
            let row_h = card + 34.0 + ui.spacing().item_spacing.y;
            let rows = shown.len().div_ceil(cols);
            egui::ScrollArea::vertical()
                .id_salt("lib-mine")
                .show_rows(ui, row_h, rows, |ui, range| {
                    for row in range {
                        ui.horizontal(|ui| {
                            for m in &shown[row * cols..((row + 1) * cols).min(shown.len())] {
                                material_card(app, &ctx, ui, m, card, tier, &mut apply, &mut delete);
                            }
                        });
                    }
                });
        });
    });
    app.library.category = filter;
    if let Some(m) = apply_preset {
        // Come i pulsanti del pannello Materiali: si tiene la versione del progetto se c'è.
        if app.scene.materials.get(&m.id).is_none() {
            app.scene.materials.upsert(m.clone());
        }
        for &i in &app.selection {
            app.scene.assignments.insert(app.scene.nodes[i].id.clone(), m.id.clone());
        }
        app.selected_material = m.id.clone();
        app.appearance_changed();
        app.library.status = Some((tf("«{}» applicato.", &[tr_dyn(&m.name).to_string()]), false));
    }
    if let Some(m) = apply {
        let kept = app.scene.materials.get(&m.material.id).is_some_and(|p| *p != m.material);
        apply_material(app, &m);
        app.library.status = Some((
            if kept {
                tf(
                    "«{}» applicato con le impostazioni del progetto (non ancora salvate nella libreria).",
                    std::slice::from_ref(&m.material.name),
                )
            } else {
                tf("«{}» applicato.", std::slice::from_ref(&m.material.name))
            },
            false,
        ));
    }
    if let Some(m) = delete {
        match store::delete(&m) {
            Ok(()) => app.library.status = Some((tf("«{}» eliminato.", std::slice::from_ref(&m.material.name)), false)),
            Err(e) => app.library.status = Some((format!("{e:#}"), true)),
        }
        app.library.items = None;
    }
}

/// Nome della categoria: in italiano la traduzione della libreria, nelle altre lingue il nome
/// originale del sito (che è in inglese), con l'iniziale maiuscola.
fn cat_label(c: &str) -> String {
    if crate::i18n::current_code() == crate::i18n::SOURCE_CODE {
        return categories::label(c);
    }
    let c = c.trim();
    let c = c
        .strip_prefix("collection:")
        .map(|r| format!("Collection {}", r.trim().replace('_', " ")))
        .unwrap_or_else(|| c.to_string());
    let mut ch = c.chars();
    ch.next()
        .map(|f| f.to_uppercase().collect::<String>() + ch.as_str())
        .unwrap_or_default()
}

/// Il materiale risponde alla ricerca? Tutte le parole devono comparire nel nome, nella
/// provenienza, nelle categorie (anche col nome italiano) o nei tag.
fn matches_query(m: &LibraryMaterial, q: &str) -> bool {
    q.split_whitespace().all(|w| {
        m.material.name.to_lowercase().contains(w)
            || m.source.to_lowercase().contains(w)
            || m.tags.iter().any(|t| t.to_lowercase().contains(w))
            || m.categories
                .iter()
                .any(|c| c.to_lowercase().contains(w) || cat_label(c).to_lowercase().contains(w))
    })
}

/// Tessera di un materiale della libreria: clic = applica, menu = apri cartella / elimina.
#[allow(clippy::too_many_arguments)]
fn material_card(
    app: &mut ForgeApp,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    m: &LibraryMaterial,
    card: f32,
    tier: u32,
    apply: &mut Option<LibraryMaterial>,
    delete: &mut Option<LibraryMaterial>,
) {
    let tex = app.library.material_thumb(ctx, m, tier);
    let maps = m.material.texture_slots().iter().filter(|s| s.is_some()).count();
    let main: Vec<String> = m
        .categories
        .iter()
        .filter(|c| !categories::is_secondary(c))
        .take(2)
        .map(|c| cat_label(c))
        .collect();
    let sub = if !m.misassigned.is_empty() {
        tr("⚠ da riscaricare").to_string()
    } else if main.is_empty() {
        tf("{maps} mappe", &[format!("{}", maps)])
    } else {
        main.join(" · ")
    };
    let selected = app.material_editor.current_id() == Some(&m.material.id);
    let r = thumb_card(ui, tex, egui::vec2(card, card), &m.material.name, &sub, selected);
    let notes = if m.notes.is_empty() {
        String::new()
    } else {
        format!("\n\n{}", m.notes.join("\n"))
    };
    let cats = if m.categories.is_empty() {
        String::new()
    } else {
        tf(
            "\nCategorie: {}",
            &[m.categories.iter().map(|c| cat_label(c)).collect::<Vec<_>>().join(", ").to_string()],
        )
    };
    let tags = if m.tags.is_empty() {
        String::new()
    } else {
        tf("\nTag: {}", &[m.tags.join(", ").to_string()])
    };
    let r = if m.misassigned.is_empty() {
        r.on_hover_text(tf(
            "{}\n{}\n{maps} mappe{cats}{tags}{notes}",
            &[
                m.source.to_string(),
                format!("{}", m.dir.display()),
                format!("{}", maps),
                cats.to_string(),
                tags.to_string(),
                notes.to_string(),
            ],
        ))
    } else {
        r.on_hover_text(tf(
            "{}\n{}\n\nImportato con un errore di riconoscimento delle mappe:\n{}\n\n\
             Riscaricalo dalla scheda Online (la copia difettosa viene sostituita).",
            &[
                m.source.to_string(),
                format!("{}", m.dir.display()),
                m.misassigned.join("\n").to_string(),
            ],
        ))
    };
    // Clic: si apre "Modifica materiale" (non si applica); doppio clic: si applica subito.
    if r.double_clicked() {
        *apply = Some(m.clone());
    } else if r.clicked() {
        let in_project = app.scene.materials.get(&m.material.id).cloned();
        app.material_editor.open_library(m, in_project.as_ref());
    }
    r.context_menu(|ui| {
        if ui.button(tr("Apri cartella")).clicked() {
            open_folder(&m.dir);
            ui.close();
        }
        if ui.button(tr("Elimina dalla libreria")).clicked() {
            *delete = Some(m.clone());
            ui.close();
        }
    });
}

/// Tessera di un materiale della libreria ufficiale (predefinito): clic = modifica, doppio clic = applica.
fn preset_card(
    app: &mut ForgeApp,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    m: &forge_scene::Material,
    card: f32,
    tier: u32,
    apply: &mut Option<forge_scene::Material>,
) {
    let tex = app.library.preset_thumb(ctx, m, tier);
    let maps = m.texture_slots().iter().filter(|s| s.is_some()).count();
    let sub = if maps > 0 {
        tf("{maps} mappe", &[format!("{}", maps)])
    } else {
        tr_dyn(&m.category)
    };
    let selected = app.material_editor.current_id() == Some(&m.id);
    let r = thumb_card(ui, tex, egui::vec2(card, card), &tr_dyn(&m.name), &sub, selected)
        .on_hover_text(tf("{}\nMateriale predefinito di ForgeRender", &[tr_dyn(&m.category).to_string()]));
    // Clic: si apre "Modifica materiale" (non si applica); doppio clic: si applica subito.
    if r.double_clicked() {
        *apply = Some(m.clone());
    } else if r.clicked() {
        let in_project = app.scene.materials.get(&m.id).cloned();
        app.material_editor.open_preset(m, in_project.as_ref());
    }
}

fn online_tab(app: &mut ForgeApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    let lib = &mut app.library;
    let mut do_search = false;
    ui.horizontal(|ui| {
        for p in [Provider::AmbientCg, Provider::PolyHaven] {
            if ui.selectable_value(&mut lib.provider, p, tr(p.label())).changed() {
                do_search = true;
            }
        }
        ui.separator();
        do_search |= ui.selectable_value(&mut lib.kind, AssetKind::Material, tr("Materiali")).changed();
        do_search |= ui.selectable_value(&mut lib.kind, AssetKind::Hdri, "HDRI").changed();
        ui.separator();
        let r = ui.add(
            egui::TextEdit::singleline(&mut lib.query)
                .hint_text(tr("es. wood, metal, studio…"))
                .desired_width(170.0),
        );
        do_search |= r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        do_search |= ui.button(tr("Cerca")).clicked();
        if ui
            .button(tr("Aggiorna catalogo"))
            .on_hover_text(tr(
                "Riscarica l'elenco completo dal sito (si aggiorna da solo una volta a settimana)",
            ))
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
            .on_hover_text(tr(
                "Risoluzione del download: 1K per prove veloci, 2K–4K per i render finali, 8K solo per primi piani",
            ));
    });
    ui.label(
        egui::RichText::new(tr(
            "Contenuti CC0: liberi anche per uso commerciale, senza obbligo di citazione. Le parole di ricerca sono in inglese.",
        ))
        .weak()
        .small(),
    );
    if do_search || (lib.results.is_empty() && lib.searching.is_none() && lib.status.is_none()) {
        lib.search(&ctx);
    }
    if lib.searching.is_some() {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(tr("Ricerca in corso…"));
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
        egui::RichText::new(tf(
            "{} {} nel catalogo{}",
            &[
                format!("{}", results.len()),
                (if lib.kind == AssetKind::Hdri { "HDRI" } else { "materiali" }).to_string(),
                (if lib.query.trim().is_empty() {
                    String::new()
                } else {
                    tf(" per «{}»", &[lib.query.trim().to_string()])
                })
                .to_string(),
            ],
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
                            Some(s) => tf("{} · {:.2} m", &[a.category.to_string(), format!("{:.2}", s)]),
                            None => a.category.clone(),
                        };
                        if have {
                            sub = tf("✓ in libreria · {sub}", &[sub.to_string()]);
                        }
                        let r = thumb_card(ui, tex, size, &a.name, &sub, have)
                            .on_hover_text(tf("{}\nClic: scarica a {res}", &[a.tags.join(", ").to_string(), res.to_string()]));
                        if r.clicked() && app.library.busy.is_none() {
                            download = Some(a.clone());
                        }
                    }
                });
            }
        });
    if let Some(a) = download {
        let label = tf("Download di «{}» ({res})", &[a.name.to_string(), res.to_string()]);
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
