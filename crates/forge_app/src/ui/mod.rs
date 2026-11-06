//! Interfaccia utente (egui). Layout:
//! barra dei menu e toolbar delle azioni rapide in alto (comandi in `crate::commands`),
//! pannello sinistro (albero della scena),
//! pannello destro (proprietà), pannello inferiore (progressi, statistiche,
//! log), area centrale (viewport o render).

pub(crate) mod ai_setup;
pub(crate) mod atmosphere;
pub(crate) mod blend_import;
pub mod central;
pub(crate) mod color_picker;
mod demo_info;
mod fireflies;
pub mod gizmo;
pub mod icons;
pub(crate) mod inspector;
pub mod library;
pub mod manual;
pub(crate) mod material_editor;
pub mod menubar;
pub(crate) mod mode_choice;
pub mod navigator;
mod quickbar;
pub(crate) mod sequence;
pub(crate) mod settings;
mod simple;
pub(crate) mod smoke;
pub(crate) mod smoke_preview;
mod splash;
pub(crate) mod status;
pub(crate) mod step_import;
pub mod style;
pub(crate) mod styles;
pub(crate) mod timeline;
mod transform;
pub mod tree;
mod video_settings;

use crate::app::ForgeApp;
use crate::i18n::{tf, tr};
use eframe::egui;

impl eframe::App for ForgeApp {
    /// Chiudendo il programma il controllo IA si spegne (file di contatto tolto).
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(s) = self.mcp.take() {
            s.shutdown();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.tick(&ctx);
        self.handle_dropped_files(&ctx);
        timeline::keyboard(self, &ctx);
        self.handle_shortcuts(&ctx);
        manual::begin_frame(&ctx);

        self.handle_close_request(&ctx);
        egui::Panel::top("menubar").show(ui, |ui| menubar::show(self, ui));
        egui::Panel::top("quickbar").show(ui, |ui| quickbar::show(self, ui));
        // Limite d'altezza anche per quella salvata: in una finestra piccola il pannello non deve
        // coprire la viewport e l'albero.
        let tall = (ui.available_height() * 0.6).max(120.0);
        egui::Panel::bottom("status-timeline")
            .resizable(true)
            .default_size(320.0_f32.min(tall))
            .min_size(70.0)
            .max_size(tall)
            .show(ui, |ui| status::show(self, ui));
        // Limiti di larghezza: un contenuto largo (es. molte miniature) non può più far
        // crescere un pannello fino a coprire la viewport; vale anche per le larghezze
        // salvate dalle sessioni precedenti.
        let total = ui.available_width();
        egui::Panel::left("tree")
            .resizable(true)
            .default_size(270.0)
            .min_size(180.0)
            .max_size((total * 0.3).max(200.0))
            .show(ui, |ui| tree::show(self, ui));
        egui::Panel::right("inspector")
            .resizable(true)
            .default_size(360.0)
            .min_size(260.0)
            .max_size((total * 0.45).max(280.0))
            .show(ui, |ui| match self.prefs.ui_mode {
                crate::prefs::UiMode::Simple => simple::show(self, ui),
                crate::prefs::UiMode::Advanced => inspector::show(self, ui),
            });
        egui::CentralPanel::default().show(ui, |ui| central::show(self, ui, frame));

        self.dialogs(&ctx);
        step_import::window(self, &ctx);
        timeline::window(self, &ctx);
    }
}

impl ForgeApp {
    fn import_options_window(&mut self, ctx: &egui::Context) {
        let mut open = true;
        let mut go = false;
        egui::Window::new(tr("Opzioni di importazione"))
            .id(egui::Id::NULL.with("win-opzioni-di-importazione"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                for f in self.import_queue.iter().take(8) {
                    ui.label(f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
                }
                if self.import_queue.len() > 8 {
                    ui.label(tf("… e altri {}", &[format!("{}", self.import_queue.len() - 8)]));
                }
                ui.separator();
                let o = &mut self.import_opts;
                egui::Grid::new("import-opts").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                    ui.label(tr("Unità (STL / OBJ)"));
                    egui::ComboBox::from_id_salt("unit")
                        .selected_text(tr(o.unit.label()))
                        .show_ui(ui, |ui| {
                            for u in forge_scene::LengthUnit::ALL {
                                ui.selectable_value(&mut o.unit, u, tr(u.label()));
                            }
                        });
                    ui.end_row();
                    ui.label(tr("Asse verticale"));
                    egui::ComboBox::from_id_salt("up")
                        .selected_text(tr(o.up.label()))
                        .show_ui(ui, |ui| {
                            for u in [forge_import::UpAxis::Auto, forge_import::UpAxis::Y, forge_import::UpAxis::Z] {
                                ui.selectable_value(&mut o.up, u, tr(u.label()));
                            }
                        })
                        .response
                        .on_hover_text(tr("Automatico: STL con Z in alto; OBJ, STEP e SolidWorks con Y in alto"));
                    ui.end_row();
                    ui.label(tr("Qualità triangolazione (STEP)"));
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut o.step_quality, forge_import::StepQuality::Bassa, tr("Bassa"));
                        ui.radio_value(&mut o.step_quality, forge_import::StepQuality::Media, tr("Media"));
                        ui.radio_value(&mut o.step_quality, forge_import::StepQuality::Alta, tr("Alta"));
                    });
                    ui.end_row();
                    ui.label(tr("Posizione"));
                    ui.checkbox(&mut o.drop_to_floor, tr("Appoggia sul pavimento dopo l'importazione"));
                    ui.end_row();
                    ui.label(tr("Angolo di piega normali"));
                    ui.add(egui::Slider::new(&mut o.crease_angle_deg, 0.0..=90.0).suffix("°"))
                        .on_hover_text(tr(
                            "Solo per mesh senza normali (STL): sotto questo angolo le facce vengono smussate",
                        ));
                    ui.end_row();
                });
                ui.label(
                    egui::RichText::new(tr(
                        "Lo STEP dichiara le proprie unità; i file SolidWorks richiedono SolidWorks installato su questo PC.",
                    ))
                    .weak()
                    .small(),
                );
                ui.separator();
                ui.horizontal(|ui| {
                    go = ui.button(egui::RichText::new(tr("Importa")).strong()).clicked();
                    if ui.button(tr("Annulla")).clicked() {
                        self.import_queue.clear();
                        self.show_import_options = false;
                    }
                });
            });
        if !open {
            self.import_queue.clear();
            self.show_import_options = false;
        }
        if go {
            self.run_import_queue();
        }
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let mut files: Vec<std::path::PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect());
        // Con l'importatore di materiali aperto un .blend va lì (non si importa come modello).
        if self.blend_import.open {
            let is_blend = |f: &std::path::PathBuf| f.extension().is_some_and(|e| e.eq_ignore_ascii_case("blend"));
            if let Some(b) = files.iter().find(|f| is_blend(f)).cloned() {
                self.blend_import.open_file(b);
            }
            files.retain(|f| !is_blend(f));
        }
        let (projects, models): (Vec<_>, Vec<_>) = files.into_iter().partition(|f| {
            f.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                e.eq_ignore_ascii_case(forge_project::EXTENSION) || e.eq_ignore_ascii_case(forge_project::pack::PACK_EXTENSION)
            })
        });
        if let Some(p) = projects.first() {
            self.open_any(p);
        }
        self.ask_import(models);
    }

    /// Tastiera: le scorciatoie sono nell'elenco unico dei comandi.
    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let gizmo_free = !self.gizmo.is_dragging();
        for cmd in crate::commands::pressed(ctx, gizmo_free) {
            crate::commands::run(self, ctx, cmd);
        }
    }

    /// Chiusura della finestra (X, Alt+F4) con modifiche non salvate: prima si chiede.
    fn handle_close_request(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) && !self.exit_confirmed && self.unsaved {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.confirm_discard = Some(crate::commands::Cmd::Exit);
        }
    }

    /// "Salvare le modifiche?" prima di nuova scena, apertura o uscita.
    fn confirm_discard_window(&mut self, ctx: &egui::Context) {
        let Some(cmd) = self.confirm_discard else { return };
        let mut choice = None;
        egui::Modal::new(egui::Id::NULL.with("conferma-modifiche")).show(ctx, |ui| {
            ui.set_width(380.0);
            ui.heading(tr("Modifiche non salvate"));
            ui.label(tr("Il progetto ha modifiche non salvate. Salvarle prima di continuare?"));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .add(egui::Button::new(egui::RichText::new(tr("Salva")).strong()).fill(style::ACCENT))
                    .clicked()
                {
                    choice = Some(0);
                }
                if ui.button(tr("Non salvare")).clicked() {
                    choice = Some(1);
                }
                if ui.button(tr("Annulla")).clicked() {
                    choice = Some(2);
                }
            });
        });
        match choice {
            Some(0) => {
                self.confirm_discard = None;
                self.save_project(false);
                // Salvataggio annullato o fallito: non si butta via niente.
                if !self.unsaved {
                    crate::commands::run_confirmed(self, ctx, cmd);
                }
            }
            Some(1) => {
                self.confirm_discard = None;
                crate::commands::run_confirmed(self, ctx, cmd);
            }
            Some(_) => self.confirm_discard = None,
            None => {}
        }
    }

    /// Avviso breve in alto al centro: non blocca niente e sparisce da solo (si ferma finché ci
    /// si passa sopra con il mouse, così c'è tempo per il pulsante).
    fn toast_area(&mut self, ctx: &egui::Context) {
        let Some(toast) = &self.toast else { return };
        let life = if toast.folder.is_some() { 8.0 } else { 5.0 };
        if toast.at.elapsed().as_secs_f32() > life {
            self.toast = None;
            return;
        }
        let (mut open_dir, mut close) = (false, false);
        let area = egui::Area::new(egui::Id::NULL.with("avviso"))
            .anchor(egui::Align2::CENTER_TOP, [0.0, 92.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_max_width(620.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&toast.text).color(style::ACCENT));
                        if toast.folder.is_some() {
                            open_dir = ui.button(tr("Apri cartella")).clicked();
                        }
                        close = ui.small_button(crate::ui::icons::CLOSE).clicked();
                    });
                });
            });
        if area.response.contains_pointer() {
            let now = std::time::Instant::now();
            if let Some(t) = &mut self.toast {
                t.at = now;
            }
        }
        if open_dir {
            if let Some(dir) = self.toast.as_ref().and_then(|t| t.folder.as_deref()) {
                crate::ui::library::open_folder(dir);
            }
        }
        if close || open_dir {
            self.toast = None;
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        self.confirm_discard_window(ctx);
        self.toast_area(ctx);
        crate::ui::library::window(self, ctx);
        crate::ui::blend_import::window(self, ctx);
        crate::ui::ai_setup::window(self, ctx);
        crate::ui::material_editor::window(self, ctx);
        crate::ui::sequence::window(self, ctx);
        crate::ui::manual::window(self, ctx);
        crate::ui::settings::window(self, ctx);
        crate::ui::splash::show(self, ctx);
        crate::ui::mode_choice::show(self, ctx);
        crate::ui::demo_info::show(self, ctx);
        if self.show_import_options {
            self.import_options_window(ctx);
        }
        if let Some(p) = &self.pending {
            egui::Window::new(tr("Operazione in corso"))
                .id(egui::Id::NULL.with("win-operazione-in-corso"))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -200.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.strong(&p.label);
                    });
                    ui.label(&p.message);
                    ui.label(egui::RichText::new(tf("{:.0} s", &[format!("{:.0}", p.started.elapsed().as_secs_f32())])).weak());
                    if !self.import_queue.is_empty() {
                        ui.label(tf("altri {} file in coda", &[format!("{}", self.import_queue.len())]));
                    }
                });
        }
        if let Some(msg) = self.error.clone() {
            let mut open = true;
            egui::Window::new(tr("Errore"))
                .id(egui::Id::NULL.with("win-errore"))
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new(&msg).color(ui.visuals().error_fg_color));
                    if ui.button("OK").clicked() {
                        self.error = None;
                    }
                });
            if !open {
                self.error = None;
            }
        }
        if let Some(msg) = self.info.clone() {
            let mut open = true;
            egui::Window::new(tr("Informazioni"))
                .id(egui::Id::NULL.with("win-informazioni"))
                .collapsible(false)
                .open(&mut open)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| ui.label(&msg));
                    if ui.button("OK").clicked() {
                        self.info = None;
                    }
                });
            if !open {
                self.info = None;
            }
        }
    }
}

/// Finestra che si può staccare dal programma. `docked`: sopra la scena, come prima; altrimenti
/// in una finestra del sistema a parte, che si sposta anche fuori dal programma o su un altro
/// schermo (se il sistema non permette più finestre resta dentro). `open` diventa falso quando la
/// si chiude. `raise`: riportarla davanti (riaperta mentre era già aperta: una finestra a parte
/// finita dietro al programma non torna davanti da sola).
#[allow(clippy::too_many_arguments)]
pub fn detachable(
    ctx: &egui::Context,
    id: &str,
    title: &str,
    size: [f32; 2],
    docked: bool,
    raise: bool,
    open: &mut bool,
    mut body: impl FnMut(&mut egui::Ui),
) {
    let window = |ctx: &egui::Context, open: &mut bool, body: &mut dyn FnMut(&mut egui::Ui)| {
        egui::Window::new(title)
            .id(egui::Id::NULL.with(id))
            .open(open)
            .default_size(size)
            .resizable(true)
            .show(ctx, |ui| body(ui));
    };
    if docked {
        if raise {
            ctx.move_to_top(egui::LayerId::new(egui::Order::Middle, egui::Id::NULL.with(id)));
        }
        window(ctx, open, &mut body);
        return;
    }
    if raise {
        let vid = egui::ViewportId::from_hash_of(id);
        ctx.send_viewport_cmd_to(vid, egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd_to(vid, egui::ViewportCommand::Focus);
    }
    let builder = egui::ViewportBuilder::default().with_title(title).with_inner_size(size);
    ctx.show_viewport_immediate(egui::ViewportId::from_hash_of(id), builder, |ui, class| {
        if class == egui::ViewportClass::EmbeddedWindow {
            body(ui);
            return;
        }
        egui::CentralPanel::default().show(ui, |ui| body(ui));
        if ui.ctx().input(|i| i.viewport().close_requested()) {
            *open = false;
        }
    });
}

/// Pulsante per staccare la finestra dal programma o riportarla dentro; vero se è cambiato.
pub fn dock_button(ui: &mut egui::Ui, docked: &mut bool) -> bool {
    let (text, hint) = if *docked {
        (
            format!("{} {}", crate::ui::icons::EXPORT, tr("Stacca dal programma")),
            tr("Apre questa finestra a parte: la sposti dove vuoi, anche fuori dal programma o su un altro schermo"),
        )
    } else {
        (
            format!("{} {}", crate::ui::icons::BACK, tr("Riporta nel programma")),
            tr("Rimette questa finestra dentro il programma, sopra la scena"),
        )
    };
    let changed = ui.small_button(text).on_hover_text(hint).clicked();
    if changed {
        *docked = !*docked;
    }
    changed
}
