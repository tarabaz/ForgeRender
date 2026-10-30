//! Finestra "Manuale" (Guida → Manuale, F1): indice a sinistra, pagina al centro, ricerca,
//! Home, Indietro/Avanti, lingua. I contenuti sono nella cartella `manuale/` (crate
//! `forge_manual`): si aggiornano senza ricompilare.
//!
//! Aiuto contestuale: un controllo si collega a una pagina con [`HelpExt::help`] (F1 mentre il
//! mouse è sopra apre quella pagina) o con [`help_button`] (icona "?" accanto).

use crate::app::ForgeApp;
use crate::i18n::{tf, tr};
use crate::ui::{icons, style};
use eframe::egui;
use forge_manual::{Block, Manual, Span};
use std::collections::HashMap;
use std::path::PathBuf;

const HOVER_KEY: &str = "manuale-controllo-sotto-il-mouse";
const OPEN_KEY: &str = "manuale-apri";

/// Cartella del manuale: `manuale` accanto al programma; durante lo sviluppo quella del
/// repository.
pub fn root() -> Option<PathBuf> {
    let beside = std::env::current_exe().ok()?.parent()?.join("manuale");
    if beside.join("indice.json").is_file() {
        return Some(beside);
    }
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../manuale");
    dev.join("indice.json").is_file().then_some(dev)
}

#[derive(Default)]
pub struct ManualUi {
    pub open: bool,
    manual: Option<Manual>,
    /// Lingua scelta nella finestra (None = quella dell'interfaccia).
    lang: Option<String>,
    history: Vec<String>,
    pos: usize,
    query: String,
    textures: HashMap<PathBuf, Option<egui::TextureHandle>>,
    error: Option<String>,
    scroll_top: bool,
    /// Pagina (o controllo) da aprire al prossimo disegno.
    pending_target: Option<String>,
}

impl ManualUi {
    /// Apre il manuale sulla pagina `id` (o sulla pagina che descrive il controllo `id`).
    pub fn open_at(&mut self, id: Option<&str>) {
        self.open = true;
        if let Some(id) = id {
            self.pending_target = Some(id.to_string());
        }
    }

    fn current(&self) -> Option<&str> {
        self.history.get(self.pos).map(String::as_str)
    }

    fn go(&mut self, id: &str) {
        if self.current() == Some(id) {
            return;
        }
        self.history.truncate(self.pos + 1);
        self.history.push(id.to_string());
        self.pos = self.history.len() - 1;
        self.scroll_top = true;
    }

    fn ensure_loaded(&mut self, ui_lang: &str) {
        let want = self.lang.clone().unwrap_or_else(|| ui_lang.to_string());
        if self.manual.as_ref().is_some_and(|m| m.lang == want) {
            return;
        }
        let Some(root) = root() else {
            self.error = Some(tr("Cartella \"manuale\" non trovata accanto al programma.").into());
            self.manual = None;
            return;
        };
        let langs = forge_manual::languages(&root);
        // Lingua non disponibile: italiano, poi la prima che c'è.
        let lang = if langs.contains(&want) {
            want
        } else if langs.iter().any(|l| l == "it") {
            "it".into()
        } else {
            langs.first().cloned().unwrap_or_default()
        };
        match Manual::load(&root, &lang) {
            Ok(m) => {
                self.manual = Some(m);
                self.error = None;
                self.textures.clear();
            }
            Err(e) => self.error = Some(format!("{e:#}")),
        }
    }

    fn home_id(&self) -> Option<String> {
        let m = self.manual.as_ref()?;
        m.index
            .capitoli
            .iter()
            .flat_map(|c| c.pagine.iter())
            .find(|id| m.pages.contains_key(*id))
            .cloned()
    }
}

/// Collega un controllo a una pagina del manuale (F1 col mouse sopra la apre).
pub trait HelpExt {
    fn help(self, id: &'static str) -> Self;
}

impl HelpExt for egui::Response {
    fn help(self, id: &'static str) -> Self {
        if self.hovered() {
            self.ctx.data_mut(|d| d.insert_temp(egui::Id::NULL.with(HOVER_KEY), id));
        }
        self
    }
}

/// Icona "?" che apre la pagina `id` del manuale.
pub fn help_button(ui: &mut egui::Ui, id: &'static str) -> egui::Response {
    let r = ui
        .add(egui::Button::new(egui::RichText::new(icons::HELP).color(ui.visuals().weak_text_color())).frame(false))
        .on_hover_text(tr("Apri la pagina del manuale (F1)"));
    if r.clicked() {
        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::NULL.with(OPEN_KEY), id));
    }
    r.help(id)
}

/// Pagina da aprire con F1: il controllo sotto il mouse, altrimenti la scheda attiva.
pub fn context_target(app: &ForgeApp, ctx: &egui::Context) -> String {
    let hovered: Option<&'static str> = ctx.data(|d| d.get_temp(egui::Id::NULL.with(HOVER_KEY)));
    if let Some(id) = hovered {
        return id.to_string();
    }
    match app.prefs.ui_mode {
        crate::prefs::UiMode::Simple => "intro.benvenuto".into(),
        crate::prefs::UiMode::Advanced => crate::commands::spec(crate::commands::Cmd::Tab(app.tab)).id.to_string(),
    }
}

/// Da chiamare a inizio frame: dimentica il controllo sotto il mouse del frame precedente.
pub fn begin_frame(ctx: &egui::Context) {
    ctx.data_mut(|d| d.remove::<&'static str>(egui::Id::NULL.with(HOVER_KEY)));
}

pub fn window(app: &mut ForgeApp, ctx: &egui::Context) {
    if let Some(id) = ctx.data_mut(|d| d.remove_temp::<&'static str>(egui::Id::NULL.with(OPEN_KEY))) {
        app.manual.open_at(Some(id));
    }
    if !app.manual.open {
        return;
    }
    let ui_lang = app.prefs.language.clone();
    let mui = &mut app.manual;
    mui.ensure_loaded(&ui_lang);
    // Destinazione richiesta (F1, "?", Guida): pagina con quell'id o che descrive quel controllo.
    if let Some(t) = mui.pending_target.take() {
        let found = mui.manual.as_ref().and_then(|m| m.page_for_ui(&t)).map(|p| p.id.clone());
        if let Some(id) = found.or_else(|| mui.home_id()) {
            mui.go(&id);
        }
    }
    if mui.current().is_none() {
        if let Some(h) = mui.home_id() {
            mui.go(&h);
        }
    }
    let mut open = true;
    egui::Window::new(format!("{}  {}", icons::HELP, tr("Manuale")))
        .id(egui::Id::NULL.with("win-manuale"))
        .open(&mut open)
        .default_size([940.0, 640.0])
        .min_size([560.0, 360.0])
        .resizable(true)
        .show(ctx, |ui| contents(mui, ui));
    mui.open = open;
}

fn contents(mui: &mut ManualUi, ui: &mut egui::Ui) {
    // Barra: Home, Indietro, Avanti, ricerca, lingua.
    ui.horizontal(|ui| {
        if ui.button(format!("{}  {}", icons::HOME, tr("Home"))).clicked() {
            if let Some(h) = mui.home_id() {
                mui.query.clear();
                mui.go(&h);
            }
        }
        if ui
            .add_enabled(mui.pos > 0, egui::Button::new(icons::BACK))
            .on_hover_text(tr("Indietro"))
            .clicked()
        {
            mui.pos -= 1;
            mui.scroll_top = true;
        }
        if ui
            .add_enabled(mui.pos + 1 < mui.history.len(), egui::Button::new(icons::FORWARD))
            .on_hover_text(tr("Avanti"))
            .clicked()
        {
            mui.pos += 1;
            mui.scroll_top = true;
        }
        ui.add(
            egui::TextEdit::singleline(&mut mui.query)
                .hint_text(format!("{}  {}", icons::SEARCH, tr("Cerca nel manuale…")))
                .desired_width(280.0),
        );
        if let Some(root) = root() {
            let langs = forge_manual::languages(&root);
            let current = mui.manual.as_ref().map(|m| m.lang.clone()).unwrap_or_default();
            let mut pick = current.clone();
            egui::ComboBox::from_id_salt("manuale-lingua")
                .selected_text(current.to_uppercase())
                .width(56.0)
                .show_ui(ui, |ui| {
                    for l in &langs {
                        ui.selectable_value(&mut pick, l.clone(), l.to_uppercase());
                    }
                })
                .response
                .on_hover_text(tr("Lingua del manuale (di solito segue quella dell'interfaccia)"));
            if pick != current {
                mui.lang = Some(pick);
            }
        }
    });
    ui.separator();
    if let Some(e) = &mui.error {
        ui.colored_label(style::WARN, e);
        return;
    }
    // Il manuale esce dallo stato per il tempo del disegno (si prende in prestito il resto).
    let Some(owned) = mui.manual.take() else { return };
    let manual = &owned;
    let mut goto: Option<String> = None;
    let avail = ui.available_height();
    ui.horizontal_top(|ui| {
        // Indice o risultati della ricerca.
        ui.vertical(|ui| {
            ui.set_width(230.0);
            egui::ScrollArea::vertical()
                .id_salt("manuale-indice")
                .max_height(avail)
                .show(ui, |ui| {
                    if mui.query.trim().is_empty() {
                        for ch in &manual.index.capitoli {
                            egui::CollapsingHeader::new(egui::RichText::new(manual.chapter_title(ch)).strong())
                                .id_salt(&ch.id)
                                .default_open(true)
                                .show(ui, |ui| {
                                    for id in &ch.pagine {
                                        if let Some(p) = manual.pages.get(id) {
                                            let sel = mui.current() == Some(id.as_str());
                                            if ui.selectable_label(sel, &p.title).clicked() {
                                                goto = Some(id.clone());
                                            }
                                        }
                                    }
                                });
                        }
                    } else {
                        let hits = manual.search(&mui.query);
                        ui.label(egui::RichText::new(tf("{} risultati", &[hits.len().to_string()])).weak());
                        for h in hits.iter().take(60) {
                            if let Some(p) = manual.pages.get(&h.id) {
                                let chapter = manual
                                    .index
                                    .capitoli
                                    .iter()
                                    .find(|c| c.id == p.chapter)
                                    .map(|c| manual.chapter_title(c))
                                    .unwrap_or_default();
                                let sel = mui.current() == Some(h.id.as_str());
                                let r = ui.selectable_label(sel, &p.title);
                                ui.label(egui::RichText::new(chapter).small().weak());
                                if r.clicked() {
                                    goto = Some(h.id.clone());
                                }
                            }
                        }
                    }
                });
        });
        ui.separator();
        // Pagina.
        ui.vertical(|ui| {
            let mut area = egui::ScrollArea::vertical().id_salt("manuale-pagina").max_height(avail);
            if mui.scroll_top {
                area = area.vertical_scroll_offset(0.0);
            }
            area.show(ui, |ui| {
                ui.set_max_width(ui.available_width().min(720.0));
                let Some(page) = mui.current().and_then(|id| manual.pages.get(id)) else {
                    ui.label(tr("Pagina non trovata."));
                    return;
                };
                for b in forge_manual::parse_blocks(&page.body) {
                    block(ui, &b, manual, &mut mui.textures, &mut goto);
                }
                if !page.related.is_empty() {
                    ui.add_space(14.0);
                    ui.separator();
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new(tr("Vedi anche:")).strong());
                        for r in &page.related {
                            if let Some(p) = manual.pages.get(r) {
                                if ui.link(&p.title).clicked() {
                                    goto = Some(r.clone());
                                }
                            }
                        }
                    });
                }
            });
        });
    });
    mui.scroll_top = false;
    mui.manual = Some(owned);
    if let Some(id) = goto {
        mui.go(&id);
    }
}

fn block(
    ui: &mut egui::Ui,
    b: &Block,
    manual: &Manual,
    textures: &mut HashMap<PathBuf, Option<egui::TextureHandle>>,
    goto: &mut Option<String>,
) {
    match b {
        Block::Heading { level, spans } => {
            ui.add_space(if *level == 1 { 4.0 } else { 10.0 });
            let size = match level {
                1 => 24.0,
                2 => 18.0,
                _ => 15.0,
            };
            let text: String = spans.iter().map(|s| s.text.as_str()).collect();
            let mut rt = egui::RichText::new(text).size(size).strong();
            if *level == 1 {
                rt = rt.color(style::ACCENT);
            }
            ui.label(rt);
            ui.add_space(4.0);
        }
        Block::Paragraph(spans) => {
            spans_ui(ui, spans, goto);
            ui.add_space(6.0);
        }
        Block::Quote(spans) => {
            egui::Frame::new()
                .fill(ui.visuals().faint_bg_color)
                .inner_margin(egui::Margin::symmetric(10, 6))
                .corner_radius(4.0)
                .show(ui, |ui| spans_ui(ui, spans, goto));
            ui.add_space(6.0);
        }
        Block::List { start, items } => {
            for (i, item) in items.iter().enumerate() {
                ui.horizontal_top(|ui| {
                    let mark = match start {
                        Some(n) => format!("{}.", n + i as u64),
                        None => "•".into(),
                    };
                    ui.add_sized([18.0, 14.0], egui::Label::new(egui::RichText::new(mark).color(style::ACCENT)));
                    ui.vertical(|ui| spans_ui(ui, item, goto));
                });
            }
            ui.add_space(6.0);
        }
        Block::Code(code) => {
            egui::Frame::new()
                .fill(ui.visuals().extreme_bg_color)
                .inner_margin(egui::Margin::symmetric(10, 6))
                .corner_radius(4.0)
                .show(ui, |ui| ui.label(egui::RichText::new(code).monospace()));
            ui.add_space(6.0);
        }
        Block::Table { header, rows } => {
            // Le colonne si dividono la larghezza della pagina: il testo va a capo.
            let n = header.len().max(1) as f32;
            let gap = 14.0;
            let col = ((ui.available_width() - gap * (n - 1.0)) / n).max(60.0);
            egui::Grid::new(ui.next_auto_id())
                .striped(true)
                .spacing([gap, 6.0])
                .min_col_width(col)
                .max_col_width(col)
                .show(ui, |ui| {
                    for cell in header {
                        let text: String = cell.iter().map(|s| s.text.as_str()).collect();
                        ui.add(egui::Label::new(egui::RichText::new(text).strong()).wrap());
                    }
                    ui.end_row();
                    for row in rows {
                        for cell in row {
                            ui.vertical(|ui| {
                                ui.set_max_width(col);
                                spans_ui(ui, cell, goto);
                            });
                        }
                        ui.end_row();
                    }
                });
            ui.add_space(8.0);
        }
        Block::Image { src, caption } => {
            let path = manual.image_path(src);
            let tex = textures.entry(path.clone()).or_insert_with(|| load_texture(ui.ctx(), &path));
            match tex {
                Some(t) => {
                    let [w, h] = t.size();
                    let width = (w as f32).min(ui.available_width());
                    let size = egui::vec2(width, width * h as f32 / w.max(1) as f32);
                    ui.add(egui::Image::new((t.id(), size)).corner_radius(4.0));
                }
                None => {
                    ui.colored_label(style::WARN, tf("Immagine mancante: {}", std::slice::from_ref(src)));
                }
            }
            if !caption.is_empty() {
                ui.label(egui::RichText::new(caption).italics().weak());
            }
            ui.add_space(8.0);
        }
        Block::Rule => {
            ui.separator();
        }
    }
}

fn spans_ui(ui: &mut egui::Ui, spans: &[Span], goto: &mut Option<String>) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for s in spans {
            // "→" non è nel carattere dell'interfaccia: si usa la freccia delle icone.
            let text = s.text.replace('→', icons::FORWARD);
            let mut rt = egui::RichText::new(text);
            if s.bold {
                rt = rt.strong();
            }
            if s.italic {
                rt = rt.italics();
            }
            if s.code {
                rt = rt.code();
            }
            match &s.link {
                Some(target) => {
                    let r = ui.link(rt);
                    if r.clicked() {
                        if forge_manual::is_internal(target) {
                            *goto = Some(target.clone());
                        } else {
                            ui.ctx().open_url(egui::OpenUrl::new_tab(target));
                        }
                    }
                }
                None => {
                    ui.label(rt);
                }
            }
        }
    });
}

fn load_texture(ctx: &egui::Context, path: &std::path::Path) -> Option<egui::TextureHandle> {
    let img = image::open(path).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let ci = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    Some(ctx.load_texture(path.display().to_string(), ci, egui::TextureOptions::LINEAR))
}

#[cfg(test)]
mod tests {
    /// Ogni collegamento dell'interfaccia al manuale (`.help("…")`, `help_button(ui, "…")`,
    /// `open_at(Some("…"))`) porta a una pagina che esiste in tutte le lingue.
    #[test]
    fn every_help_link_has_a_page_in_every_language() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut ids = std::collections::BTreeSet::new();
        let mut files = vec![src.join("commands.rs")];
        files.extend(std::fs::read_dir(src.join("ui")).unwrap().flatten().map(|e| e.path()));
        let pats = [".help(\"", "help_button(ui, \"", "open_at(Some(\""];
        for f in files.iter().filter(|f| f.extension().is_some_and(|x| x == "rs")) {
            if f.ends_with("manual.rs") {
                continue;
            }
            let text = std::fs::read_to_string(f).unwrap();
            for p in pats {
                for (i, _) in text.match_indices(p) {
                    let rest = &text[i + p.len()..];
                    ids.insert(rest[..rest.find('"').unwrap()].to_string());
                }
            }
        }
        assert!(ids.len() >= 4, "{ids:?}");
        let root = super::root().expect("cartella manuale");
        for lang in forge_manual::languages(&root) {
            let m = forge_manual::Manual::load(&root, &lang).unwrap();
            let missing: Vec<_> = ids.iter().filter(|id| m.page_for_ui(id).is_none()).collect();
            assert!(missing.is_empty(), "[{lang}] controlli senza pagina nel manuale: {missing:?}");
        }
    }
}
