//! Finestra "Impostazioni": ogni controllo comanda una funzione reale e viene salvato subito
//! in `impostazioni.json` (vedi `crate::prefs`).

use crate::app::ForgeApp;
use crate::i18n::{tf, tr};
use crate::prefs::Preferences;
use crate::ui::style;
use eframe::egui;
use forge_library::online::RESOLUTIONS;
use forge_library::store::ImportDisplacement;
use forge_render_core::BackendKind;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Section {
    #[default]
    Language,
    Library,
    Paths,
    Rendering,
    About,
}

pub fn window(app: &mut ForgeApp, ctx: &egui::Context) {
    if !app.prefs_open {
        return;
    }
    let id = egui::Id::NULL.with("settings-section");
    let mut section: Section = ctx.data(|d| d.get_temp(id)).unwrap_or_default();
    let mut p = app.prefs.clone();
    let mut reset = false;
    let mut open = true;
    egui::Window::new(tr("Impostazioni"))
        .id(egui::Id::NULL.with("win-impostazioni"))
        .open(&mut open)
        .default_size([620.0, 420.0])
        .resizable(true)
        .show(ctx, |ui| {
            if let Some(e) = &app.prefs_error {
                ui.colored_label(style::WARN, e);
                ui.separator();
            }
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(150.0);
                    for (s, label) in [
                        (Section::Language, tr("Lingua")),
                        (Section::Library, tr("Libreria e import")),
                        (Section::Paths, tr("Percorsi")),
                        (Section::Rendering, tr("Rendering")),
                        (Section::About, tr("Informazioni")),
                    ] {
                        ui.selectable_value(&mut section, s, label);
                    }
                    ui.add_space(12.0);
                    if ui
                        .button(tr("Ripristina predefiniti"))
                        .on_hover_text(tr("Riporta tutte le impostazioni ai valori iniziali"))
                        .clicked()
                    {
                        reset = true;
                    }
                });
                ui.add_space(10.0);
                ui.vertical(|ui| match section {
                    Section::Language => language(ui, &mut p, &app.languages, &app.language_errors),
                    Section::Library => library(ui, &mut p),
                    Section::Paths => paths(ui, &mut p),
                    Section::Rendering => rendering(ui, &mut p),
                    Section::About => about(ui),
                });
            });
            ui.separator();
            ui.label(
                egui::RichText::new(tf("Salvate in {}", &[format!("{}", crate::prefs::file_path().display())]))
                    .small()
                    .weak(),
            );
        });
    ctx.data_mut(|d| d.insert_temp(id, section));
    app.prefs_open = open;
    if reset {
        p = Preferences::default();
    }
    if p != app.prefs {
        app.apply_prefs(p, false);
    }
}

fn language(ui: &mut egui::Ui, p: &mut Preferences, langs: &[std::sync::Arc<crate::i18n::Language>], errors: &[String]) {
    ui.heading(tr("Lingua"));
    ui.label(tr("Cambia subito tutte le scritte dell'interfaccia."));
    ui.add_space(4.0);
    for l in langs {
        let origin = match &l.source {
            Some(path) => tf("file {}", &[format!("{}", path.display())]),
            None => tr("compresa nel programma").to_string(),
        };
        ui.radio_value(&mut p.language, l.code.clone(), format!("{}  ({})", l.name, origin));
    }
    ui.add_space(8.0);
    let dir = crate::i18n::external_dir().map(|d| d.display().to_string()).unwrap_or_default();
    ui.label(
        egui::RichText::new(tf(
            "Per aggiungere una lingua copia lang/Lan_eng.json come Lan_<nome>.json nella cartella {dir}, cambia \"code\" e \"name\" e traduci i testi. Si legge al prossimo avvio.",
            std::slice::from_ref(&dir),
        ))
        .small()
        .weak(),
    );
    for e in errors {
        ui.colored_label(style::WARN, e);
    }
}

fn library(ui: &mut egui::Ui, p: &mut Preferences) {
    ui.heading(tr("Libreria e import dei materiali"));
    egui::Grid::new("prefs-lib").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
        ui.label(tr("Risoluzione dei download"));
        egui::ComboBox::from_id_salt("prefs-res")
            .selected_text(RESOLUTIONS[p.download_resolution])
            .show_ui(ui, |ui| {
                for (i, r) in RESOLUTIONS.iter().enumerate() {
                    ui.selectable_value(&mut p.download_resolution, i, *r);
                }
            });
        ui.end_row();
        ui.label(tr("Sito proposto"));
        ui.horizontal(|ui| {
            ui.selectable_value(&mut p.download_provider, "ambientcg".to_string(), "ambientCG");
            ui.selectable_value(&mut p.download_provider, "polyhaven".to_string(), tr("Poly Haven"));
        });
        ui.end_row();
        ui.vertical(|ui| ui.label(tr("Displacement all'import")));
        ui.vertical(|ui| {
            ui.radio_value(&mut p.import_displacement, ImportDisplacement::Off, tr("Spento (consigliato)"))
                .on_hover_text(tr("La height map resta collegata: si accende dal pannello del materiale"));
            ui.radio_value(
                &mut p.import_displacement,
                ImportDisplacement::Proposed,
                tr("Valore proposto (3 % del lato della texture)"),
            )
            .on_hover_text(tr("Adatto a pietre, mattoni, cortecce; sui metalli lisci crea bozzi"));
            ui.radio_value(
                &mut p.import_displacement,
                ImportDisplacement::FromMaterialX,
                tr("Dal file MaterialX della fonte, se c'è"),
            )
            .on_hover_text(tr(
                "Usa la scala scritta nel .mtlx scaricato con il materiale; senza file resta spento",
            ));
        });
        ui.end_row();
    });
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new(tr(
            "Le mappe si importano così come sono: i cursori del materiale le moltiplicano (1 = valore della mappa). \
             Valgono per i prossimi import dall'app; la riga di comando usa i valori predefiniti.",
        ))
        .small()
        .weak(),
    );
}

fn paths(ui: &mut egui::Ui, p: &mut Preferences) {
    ui.heading(tr("Percorsi"));
    ui.label(tr("Cartella della libreria (HDRI, Materiali, Texture)"));
    let current = forge_library::paths::root();
    ui.label(egui::RichText::new(current.display().to_string()).monospace());
    ui.horizontal(|ui| {
        if ui.button(tr("Cambia…")).clicked() {
            if let Some(dir) = rfd::FileDialog::new().set_directory(&current).pick_folder() {
                p.library_dir = Some(dir);
            }
        }
        if ui
            .add_enabled(p.library_dir.is_some(), egui::Button::new(tr("Usa la predefinita")))
            .on_hover_text(forge_library::paths::default_root().display().to_string())
            .clicked()
        {
            p.library_dir = None;
        }
        if ui.button(tr("Apri")).clicked() {
            crate::ui::library::open_folder(&current);
        }
    });
    if std::env::var_os("FORGE_LIBRARY_DIR").is_some() {
        ui.colored_label(
            style::WARN,
            tr("La variabile d'ambiente FORGE_LIBRARY_DIR ha la precedenza su questa scelta."),
        );
    }
    ui.label(
        egui::RichText::new(tr(
            "La cartella si può copiare su un altro PC o su un disco esterno: ogni materiale è autosufficiente.",
        ))
        .small()
        .weak(),
    );
}

fn rendering(ui: &mut egui::Ui, p: &mut Preferences) {
    ui.heading(tr("Rendering"));
    ui.label(
        egui::RichText::new(tr("Valori dei nuovi progetti (i progetti salvati tengono i loro)."))
            .small()
            .weak(),
    );
    egui::Grid::new("prefs-render").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
        ui.label(tr("Dispositivo"));
        ui.horizontal(|ui| {
            ui.selectable_value(&mut p.new_backend, BackendKind::Cpu, "CPU");
            ui.selectable_value(&mut p.new_backend, BackendKind::Gpu, "GPU");
        });
        ui.end_row();
        ui.label(tr("Campioni per pixel"));
        ui.add(egui::DragValue::new(&mut p.new_samples).range(0..=1_000_000).speed(16))
            .on_hover_text(tr("0 = progressivo senza limite"));
        ui.end_row();
        ui.label(tr("Thread CPU"));
        ui.add(egui::DragValue::new(&mut p.new_threads).range(0..=1024))
            .on_hover_text(tr("0 = automatico (thread logici − 1)"));
        ui.end_row();
        ui.label(tr("Riavvio automatico"));
        ui.checkbox(&mut p.auto_restart, "")
            .on_hover_text(tr("Quando cambi materiali, luci o camera il rendering riparte da solo"));
        ui.end_row();
    });
}

fn about(ui: &mut egui::Ui) {
    ui.heading(tr("Informazioni"));
    ui.label(crate::app::APP_TITLE);
    egui::Grid::new("prefs-about").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr("Libreria"));
        ui.label(egui::RichText::new(forge_library::paths::root().display().to_string()).monospace());
        ui.end_row();
        ui.label(tr("Log"));
        ui.label(
            egui::RichText::new(
                forge_telemetry::default_log_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "—".into()),
            )
            .monospace(),
        );
        ui.end_row();
        ui.label(tr("Impostazioni"));
        ui.label(egui::RichText::new(crate::prefs::file_path().display().to_string()).monospace());
        ui.end_row();
    });
}
