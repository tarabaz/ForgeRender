//! Elenco unico dei comandi: testo, icona, scorciatoia, suggerimento e azione.
//! Barra dei menu, toolbar e tastiera leggono tutti da qui, così la scorciatoia scritta nel
//! menu è sempre quella vera e un comando non esiste in due versioni diverse. Gli `id` sono
//! stabili: li useranno anche le pagine del manuale.

use crate::app::{ForgeApp, InspectorTab, NewLight, View};
use crate::i18n::{tk, tr};
use crate::ui::icons;
use eframe::egui::{self, Key, KeyboardShortcut, Modifiers};
use forge_render_core::SessionState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmd {
    // File
    NewEmpty,
    NewTest,
    Open,
    OpenRecent(usize),
    Save,
    SaveAs,
    Import,
    Export,
    Exit,
    // Modifica
    Undo,
    Redo,
    Duplicate,
    Delete,
    Hide,
    Isolate,
    ShowAll,
    Preferences,
    ResetPreferences,
    // Scena
    PrepareScene,
    FitScene,
    UndoPreset,
    DropToBase,
    FrameProduct,
    // Oggetti
    AddLight(NewLight),
    // Vista
    FrameAll,
    FrameSelection,
    ToggleOrtho,
    Isometric,
    StandardView(usize),
    ShowViewport,
    ShowRender,
    // Strumenti del gizmo
    GizmoSelect,
    GizmoMove,
    GizmoRotate,
    GizmoScale,
    // Rendering
    RenderToggle,
    Stop,
    Restart,
    AutoRestart,
    // Libreria
    LibraryMine,
    LibraryOnline,
    LibraryFolder,
    // Strumenti
    AiControl,
    // Pannelli della modalità Avanzata
    Tab(InspectorTab),
    // Guida
    Manual,
    Shortcuts,
    Welcome,
}

/// Formati che l'import accetta davvero (vedi `ForgeApp::import_dialog`).
pub const IMPORT_HINT: &str = tk(
    "Importa un modello (Ctrl+I): glTF/GLB, OBJ, STL, Blender .blend (con Blender installato), \
     STEP (con il convertitore forgerender-step), SolidWorks SLDPRT/SLDASM (su Windows, con SolidWorks installato)",
);

/// Descrizione di un comando per menu, toolbar e manuale.
pub struct Spec {
    /// Identificativo stabile (pagine del manuale, test).
    #[allow(dead_code)] // letto dai test; il manuale lo userà per i collegamenti
    pub id: &'static str,
    /// Testo (chiave di traduzione).
    pub label: &'static str,
    pub icon: &'static str,
    pub shortcut: Option<KeyboardShortcut>,
    /// Suggerimento al passaggio del mouse (chiave di traduzione, vuoto = nessuno).
    pub hint: &'static str,
}

const fn key(m: Modifiers, k: Key) -> Option<KeyboardShortcut> {
    Some(KeyboardShortcut::new(m, k))
}
const NONE: Modifiers = Modifiers::NONE;
const CMD: Modifiers = Modifiers::COMMAND;
const CMD_SHIFT: Modifiers = Modifiers {
    alt: false,
    ctrl: false,
    shift: true,
    mac_cmd: false,
    command: true,
};

pub fn spec(cmd: Cmd) -> Spec {
    let s = |id, label, icon, shortcut, hint| Spec {
        id,
        label,
        icon,
        shortcut,
        hint,
    };
    match cmd {
        Cmd::NewEmpty => s(
            "file.new_empty",
            tk("Nuova scena vuota"),
            icons::NEW,
            key(CMD, Key::N),
            tk("Scena senza oggetti né luci"),
        ),
        Cmd::NewTest => s(
            "file.new_test",
            tk("Nuova scena di test"),
            icons::NEW,
            None,
            tk("Sfere, cubo e toro su fondale, con luci da studio: per provare materiali e impostazioni"),
        ),
        Cmd::Open => s(
            "file.open",
            tk("Apri progetto…"),
            icons::OPEN,
            key(CMD, Key::O),
            tk("Apre un progetto ForgeRender (.frproject)"),
        ),
        Cmd::OpenRecent(_) => s("file.recent", tk("Progetti recenti"), icons::RECENT, None, ""),
        Cmd::Save => s(
            "file.save",
            tk("Salva progetto"),
            icons::SAVE,
            key(CMD, Key::S),
            tk("Salva il progetto (la prima volta chiede dove)"),
        ),
        Cmd::SaveAs => s(
            "file.save_as",
            tk("Salva progetto con nome…"),
            icons::SAVE,
            key(CMD_SHIFT, Key::S),
            tk("Salva una copia del progetto con un altro nome"),
        ),
        Cmd::Import => s("file.import", tk("Importa modello…"), icons::IMPORT, key(CMD, Key::I), IMPORT_HINT),
        Cmd::Export => s(
            "file.export",
            tk("Esporta immagine…"),
            icons::EXPORT,
            key(CMD, Key::E),
            tk("Salva il render: PNG, JPG, TIFF 16 bit o EXR (si sceglie nella finestra)"),
        ),
        Cmd::Exit => s(
            "file.exit",
            tk("Esci"),
            icons::EXIT,
            None,
            tk("Chiude ForgeRender (chiede prima di perdere modifiche non salvate)"),
        ),
        Cmd::Undo => s(
            "edit.undo",
            tk("Annulla"),
            icons::UNDO,
            key(CMD, Key::Z),
            tk("Annulla l'ultima modifica di posizione, rotazione, scala o eliminazione"),
        ),
        Cmd::Redo => s(
            "edit.redo",
            tk("Ripeti"),
            icons::REDO,
            key(CMD, Key::Y),
            tk("Rifà la modifica annullata (anche Ctrl+Maiusc+Z)"),
        ),
        Cmd::Duplicate => s(
            "edit.duplicate",
            tk("Duplica"),
            icons::DUPLICATE,
            key(CMD, Key::D),
            tk("Copia gli oggetti selezionati"),
        ),
        Cmd::Delete => s(
            "edit.delete",
            tk("Elimina"),
            icons::DELETE,
            key(NONE, Key::Delete),
            tk("Toglie dal progetto gli oggetti selezionati (il file importato non viene toccato; Ctrl+Z li riporta)"),
        ),
        Cmd::Hide => s("edit.hide", tk("Nascondi selezione"), icons::HIDE, key(NONE, Key::H), ""),
        Cmd::Isolate => s(
            "edit.isolate",
            tk("Nascondi tutto il resto"),
            icons::HIDE,
            key(Modifiers::SHIFT, Key::H),
            "",
        ),
        Cmd::ShowAll => s("edit.show_all", tk("Mostra tutto"), icons::SHOW, key(Modifiers::ALT, Key::H), ""),
        Cmd::Preferences => s(
            "edit.preferences",
            tk("Impostazioni…"),
            icons::SETTINGS,
            None,
            tk("Lingua, libreria e import, percorsi, valori dei nuovi progetti"),
        ),
        Cmd::ResetPreferences => s(
            "edit.reset_preferences",
            tk("Ripristina impostazioni predefinite"),
            icons::SETTINGS,
            None,
            tk("Riporta tutte le impostazioni ai valori iniziali"),
        ),
        Cmd::PrepareScene => s(
            "scene.prepare",
            tk("Prepara scena prodotto"),
            icons::SCENE,
            None,
            tk("Appoggia il prodotto, aggiunge il fondale se manca, applica aspetto e qualità e inquadra"),
        ),
        Cmd::FitScene => s(
            "scene.fit",
            tk("Adatta la scena al prodotto"),
            icons::SCENE,
            None,
            tk("Rifà fondale e luci sulle misure attuali del prodotto"),
        ),
        Cmd::UndoPreset => s(
            "scene.undo_preset",
            tk("Annulla preset"),
            icons::UNDO,
            None,
            tk("Riporta luci, ambiente, qualità, fondale e posizione del prodotto a com'erano prima dell'ultimo preset"),
        ),
        Cmd::DropToBase => s(
            "scene.drop_to_base",
            tk("Appoggia sulla base"),
            icons::SCENE,
            None,
            tk("Appoggia il prodotto sul pavimento o sul fondale"),
        ),
        Cmd::FrameProduct => s(
            "scene.frame_product",
            tk("Inquadra il prodotto"),
            icons::FRAME,
            None,
            tk("Inquadra il prodotto senza pavimento e fondale"),
        ),
        Cmd::AddLight(_) => s("objects.light", tk("Luce"), icons::LIGHT, None, ""),
        Cmd::FrameAll => s("view.frame_all", tk("Inquadra tutto"), icons::FRAME, key(NONE, Key::Home), ""),
        Cmd::FrameSelection => s(
            "view.frame_selection",
            tk("Inquadra selezione"),
            icons::FRAME,
            key(NONE, Key::F),
            "",
        ),
        Cmd::ToggleOrtho => s(
            "view.ortho",
            tk("Prospettica / ortografica"),
            icons::VIEWS,
            key(NONE, Key::Num5),
            tk("Ortografica: misure senza prospettiva, come in un disegno tecnico"),
        ),
        Cmd::Isometric => s(
            "view.isometric",
            tk("Isometrica"),
            icons::VIEWS,
            None,
            tk("Vista isometrica (ortografica, 45° di lato)"),
        ),
        Cmd::StandardView(_) => s("view.standard", tk("Viste standard"), icons::VIEWS, None, ""),
        Cmd::ShowViewport => s(
            "view.viewport",
            tk("Viewport"),
            icons::VIEWPORT,
            None,
            tk("Vista di lavoro in tempo reale"),
        ),
        Cmd::ShowRender => s("view.render", tk("Render"), icons::IMAGE, None, tk("Immagine del path tracer")),
        Cmd::GizmoSelect => s("tool.select", tk("Seleziona"), icons::VIEWPORT, key(NONE, Key::Q), ""),
        Cmd::GizmoMove => s("tool.move", tk("Sposta"), icons::VIEWPORT, key(NONE, Key::W), ""),
        Cmd::GizmoRotate => s("tool.rotate", tk("Ruota"), icons::VIEWPORT, key(NONE, Key::E), ""),
        Cmd::GizmoScale => s("tool.scale", tk("Scala"), icons::VIEWPORT, key(NONE, Key::R), ""),
        Cmd::RenderToggle => s(
            "render.start",
            tk("Avvia render"),
            icons::RENDER,
            key(NONE, Key::F5),
            tk("Avvia, mette in pausa o riprende il path tracing"),
        ),
        Cmd::Stop => s(
            "render.stop",
            tk("Stop"),
            icons::STOP,
            None,
            tk("Ferma il rendering mantenendo l'immagine corrente"),
        ),
        Cmd::Restart => s(
            "render.restart",
            tk("Riavvia"),
            icons::RESTART,
            None,
            tk("Riparte da zero campioni"),
        ),
        Cmd::AutoRestart => s(
            "render.auto_restart",
            tk("Riavvio automatico"),
            icons::RESTART,
            None,
            tk("Quando cambi materiali, luci o camera il rendering riparte senza riutilizzare campioni incompatibili"),
        ),
        Cmd::LibraryMine => s(
            "library.mine",
            tk("La mia libreria"),
            icons::LIBRARY,
            None,
            tk("Materiali con texture e HDRI salvati sul computer"),
        ),
        Cmd::LibraryOnline => s(
            "library.online",
            tk("Scarica da ambientCG / Poly Haven…"),
            icons::LIBRARY,
            None,
            tk("Cerca e scarica materiali e HDRI gratuiti"),
        ),
        Cmd::LibraryFolder => s(
            "library.folder",
            tk("Cartella della libreria…"),
            icons::OPEN,
            None,
            tk("Dove sono salvati materiali, HDRI e texture"),
        ),
        Cmd::AiControl => s(
            "tools.ai_control",
            tk("Controllo IA"),
            icons::AI,
            None,
            tk("Permette a Claude di comandare questa finestra (tramite forgerender-mcp)"),
        ),
        Cmd::Tab(t) => s(tab_id(t), tab_label(t), tab_icon(t), None, ""),
        Cmd::Manual => s(
            "help.manual",
            tk("Manuale"),
            icons::HELP,
            key(NONE, Key::F1),
            tk("Il manuale di ForgeRender. F1 sopra un controllo apre la sua pagina"),
        ),
        Cmd::Shortcuts => s("help.shortcuts", tk("Scorciatoie da tastiera"), icons::KEYS, None, ""),
        Cmd::Welcome => s("help.welcome", tk("Benvenuto e informazioni…"), icons::INFO, None, ""),
    }
}

fn tab_id(t: InspectorTab) -> &'static str {
    match t {
        InspectorTab::Object => "panel.object",
        InspectorTab::Materials => "panel.materials",
        InspectorTab::Lights => "panel.lights",
        InspectorTab::Environment => "panel.environment",
        InspectorTab::Effects => "panel.effects",
        InspectorTab::Camera => "panel.camera",
        InspectorTab::Render => "panel.render",
    }
}

pub fn tab_label(t: InspectorTab) -> &'static str {
    match t {
        InspectorTab::Object => tk("Oggetto"),
        InspectorTab::Materials => tk("Materiali"),
        InspectorTab::Lights => tk("Luci"),
        InspectorTab::Environment => tk("Ambiente"),
        InspectorTab::Effects => tk("Effetti"),
        InspectorTab::Camera => tk("Camera"),
        InspectorTab::Render => tk("Render"),
    }
}

fn tab_icon(t: InspectorTab) -> &'static str {
    match t {
        InspectorTab::Object => icons::ADD,
        InspectorTab::Materials => icons::MATERIAL,
        InspectorTab::Lights => icons::LIGHT,
        InspectorTab::Environment => icons::ENVIRONMENT,
        InspectorTab::Effects => icons::SCENE,
        InspectorTab::Camera => icons::CAMERA,
        InspectorTab::Render => icons::QUALITY,
    }
}

/// Scorciatoie nell'ordine in cui vanno lette: prima le più specifiche (egui ignora Maiusc e
/// Alt in più, quindi Ctrl+Maiusc+S va controllata prima di Ctrl+S, Maiusc+H prima di H).
const SHORTCUTS: &[(Modifiers, Key, Cmd)] = &[
    (CMD_SHIFT, Key::S, Cmd::SaveAs),
    (CMD_SHIFT, Key::Z, Cmd::Redo),
    (CMD, Key::S, Cmd::Save),
    (CMD, Key::O, Cmd::Open),
    (CMD, Key::I, Cmd::Import),
    (CMD, Key::E, Cmd::Export),
    (CMD, Key::N, Cmd::NewEmpty),
    (CMD, Key::Y, Cmd::Redo),
    (CMD, Key::Z, Cmd::Undo),
    (CMD, Key::D, Cmd::Duplicate),
    (NONE, Key::Delete, Cmd::Delete),
    (Modifiers::SHIFT, Key::H, Cmd::Isolate),
    (Modifiers::ALT, Key::H, Cmd::ShowAll),
    (NONE, Key::H, Cmd::Hide),
    (NONE, Key::Home, Cmd::FrameAll),
    (NONE, Key::F, Cmd::FrameSelection),
    (NONE, Key::F5, Cmd::RenderToggle),
    (NONE, Key::Num5, Cmd::ToggleOrtho),
    (NONE, Key::F1, Cmd::Manual),
];

/// Tasti del gizmo: solo quando non si sta trascinando.
const GIZMO_KEYS: &[(Key, Cmd)] = &[
    (Key::Q, Cmd::GizmoSelect),
    (Key::W, Cmd::GizmoMove),
    (Key::E, Cmd::GizmoRotate),
    (Key::R, Cmd::GizmoScale),
];

/// Legge la tastiera e restituisce i comandi premuti in questo frame.
pub fn pressed(ctx: &egui::Context, gizmo_free: bool) -> Vec<Cmd> {
    if ctx.egui_wants_keyboard_input() {
        return Vec::new();
    }
    ctx.input_mut(|i| {
        let mut out: Vec<Cmd> = SHORTCUTS
            .iter()
            .filter(|(m, k, _)| i.consume_key(*m, *k))
            .map(|(_, _, c)| *c)
            .collect();
        if gizmo_free {
            out.extend(GIZMO_KEYS.iter().filter(|(k, _)| i.consume_key(NONE, *k)).map(|(_, c)| *c));
        }
        out
    })
}

/// Il comando si può usare adesso?
pub fn enabled(app: &ForgeApp, cmd: Cmd) -> bool {
    match cmd {
        Cmd::Stop => app.render_state().is_active(),
        Cmd::RenderToggle => !app.is_preparing(),
        Cmd::Export => app.session.latest_frame().is_some(),
        Cmd::Duplicate | Cmd::Delete | Cmd::Hide | Cmd::Isolate | Cmd::FrameSelection => !app.selection.is_empty(),
        Cmd::UndoPreset => app.preset_backup.is_some(),
        Cmd::OpenRecent(i) => i < app.prefs.recent_projects.len(),
        _ => true,
    }
}

/// Testo del comando "Avvia render" secondo lo stato (Pausa, Riprendi…).
pub fn render_label(app: &ForgeApp) -> &'static str {
    match app.render_state() {
        SessionState::Rendering => tk("Pausa"),
        SessionState::Paused => tk("Riprendi"),
        SessionState::Cancelled if !app.render_is_stale() => tk("Riprendi"),
        _ => tk("Avvia render"),
    }
}

/// Comandi che buttano via la scena attuale: con modifiche non salvate si chiede prima.
fn discards_scene(cmd: Cmd) -> bool {
    matches!(cmd, Cmd::NewEmpty | Cmd::NewTest | Cmd::Open | Cmd::OpenRecent(_) | Cmd::Exit)
}

/// Esegue un comando (da menu, toolbar o tastiera).
pub fn run(app: &mut ForgeApp, ctx: &egui::Context, cmd: Cmd) {
    if !enabled(app, cmd) {
        return;
    }
    if discards_scene(cmd) && app.unsaved {
        app.confirm_discard = Some(cmd);
        return;
    }
    run_confirmed(app, ctx, cmd);
}

/// Esegue senza chiedere conferma (dopo "Non salvare" o "Salva").
pub fn run_confirmed(app: &mut ForgeApp, ctx: &egui::Context, cmd: Cmd) {
    let msg_before = app.simple_msg.clone();
    match cmd {
        Cmd::NewEmpty => app.new_empty_scene(),
        Cmd::NewTest => app.new_test_scene(),
        Cmd::Open => app.open_project_dialog(),
        Cmd::OpenRecent(i) => {
            if let Some(p) = app.prefs.recent_projects.get(i).cloned() {
                if p.is_file() {
                    app.open_project(&p);
                } else {
                    let mut prefs = app.prefs.clone();
                    prefs.recent_projects.retain(|x| x != &p);
                    app.apply_prefs(prefs, false);
                    app.error = Some(crate::i18n::tf(
                        "Il progetto non esiste più: {}\nTolto dall'elenco dei recenti.",
                        &[p.display().to_string()],
                    ));
                }
            }
        }
        Cmd::Save => app.save_project(false),
        Cmd::SaveAs => app.save_project(true),
        Cmd::Import => app.import_dialog(),
        Cmd::Export => app.export_dialog(),
        Cmd::Exit => {
            app.exit_confirmed = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        Cmd::Undo => app.undo(),
        Cmd::Redo => app.redo(),
        Cmd::Duplicate => app.duplicate_selection(),
        Cmd::Delete => app.delete_selected_copies(),
        Cmd::Hide => app.hide_selection(false),
        Cmd::Isolate => app.hide_selection(true),
        Cmd::ShowAll => app.show_all(),
        Cmd::Preferences => app.prefs_open = true,
        Cmd::ResetPreferences => {
            let keep = crate::prefs::Preferences {
                recent_projects: app.prefs.recent_projects.clone(),
                ..Default::default()
            };
            app.apply_prefs(keep, false);
            app.toast(tr("Impostazioni riportate ai valori iniziali."));
        }
        Cmd::PrepareScene => app.simple_prepare_scene(),
        Cmd::FitScene => app.simple_fit_scene(),
        Cmd::UndoPreset => app.simple_undo_preset(),
        Cmd::DropToBase => app.drop_product_to_base(),
        Cmd::FrameProduct => app.frame_product(),
        Cmd::AddLight(k) => {
            app.add_light(k);
            open_tab(app, InspectorTab::Lights);
        }
        Cmd::FrameAll => app.frame_all(),
        Cmd::FrameSelection => app.frame_selection(),
        Cmd::ToggleOrtho => app.toggle_ortho(),
        Cmd::Isometric => crate::ui::navigator::isometric(app),
        Cmd::StandardView(i) => {
            if let Some((_, dir)) = crate::ui::navigator::STANDARD_VIEWS.get(i) {
                crate::ui::navigator::look_from(app, *dir);
            }
        }
        Cmd::ShowViewport => app.view = View::Viewport,
        Cmd::ShowRender => app.view = View::Render,
        Cmd::GizmoSelect => app.gizmo.mode = crate::ui::gizmo::GizmoMode::Select,
        Cmd::GizmoMove => app.gizmo.mode = crate::ui::gizmo::GizmoMode::Move,
        Cmd::GizmoRotate => app.gizmo.mode = crate::ui::gizmo::GizmoMode::Rotate,
        Cmd::GizmoScale => app.gizmo.mode = crate::ui::gizmo::GizmoMode::Scale,
        Cmd::RenderToggle => app.toggle_pause(),
        Cmd::Stop => app.stop_render(),
        Cmd::Restart => app.start_render(),
        Cmd::AutoRestart => {
            let p = crate::prefs::Preferences {
                auto_restart: !app.auto_restart,
                ..app.prefs.clone()
            };
            app.auto_restart = p.auto_restart;
            app.apply_prefs(p, false);
        }
        Cmd::LibraryMine => app.library.open_on(false),
        Cmd::LibraryOnline => app.library.open_on(true),
        Cmd::LibraryFolder => crate::ui::settings::open_paths(app, ctx),
        Cmd::AiControl => {
            let on = app.mcp.is_some();
            crate::mcp::set_enabled(app, ctx, !on);
        }
        Cmd::Tab(t) => open_tab(app, t),
        Cmd::Manual => {
            let target = crate::ui::manual::context_target(app, ctx);
            app.manual.open_at(Some(&target));
        }
        Cmd::Shortcuts => {
            // La pagina del manuale se c'è, altrimenti l'elenco generato dalle tabelle.
            if crate::ui::manual::root().is_some() {
                app.manual.open_at(Some("keys.elenco"));
            } else {
                app.info = Some(shortcuts_text(ctx));
            }
        }
        Cmd::Welcome => app.splash_open = true,
    }
    // In Avanzata il messaggio dei preset non ha un pannello: lo si mostra come avviso.
    if app.simple_msg != msg_before && app.prefs.ui_mode == crate::prefs::UiMode::Advanced {
        if let Some(m) = app.simple_msg.clone() {
            app.toast(m);
        }
    }
}

/// Apre una scheda del pannello Avanzata; dalla Semplice passa da sola all'Avanzata.
pub fn open_tab(app: &mut ForgeApp, t: InspectorTab) {
    app.tab = t;
    if app.prefs.ui_mode == crate::prefs::UiMode::Simple {
        let p = crate::prefs::Preferences {
            ui_mode: crate::prefs::UiMode::Advanced,
            ..app.prefs.clone()
        };
        app.apply_prefs(p, false);
        app.toast(tr("Passato alla modalità Avanzata (si torna alla Semplice in alto a sinistra)."));
    }
}

/// Elenco delle scorciatoie per Guida → Scorciatoie da tastiera (generato dalle tabelle vere).
pub fn shortcuts_text(ctx: &egui::Context) -> String {
    let mut lines: Vec<String> = SHORTCUTS
        .iter()
        .map(|(m, k, c)| (KeyboardShortcut::new(*m, *k), *c))
        .chain(GIZMO_KEYS.iter().map(|(k, c)| (KeyboardShortcut::new(NONE, *k), *c)))
        .map(|(s, c)| format!("{:<16}  {}", ctx.format_shortcut(&s), tr(spec(c).label)))
        .collect();
    for (keys, what) in [
        (tk("Alt + trascina"), tk("Sposta la selezione invece della camera")),
        (
            tk("Maiusc + trascina"),
            tk("Sposta la camera di lato (anche tasto destro o centrale)"),
        ),
        (tk("Ctrl + trascina il gizmo"), tk("Snap a passi fissi")),
        (tk("Esc"), tk("Annulla il trascinamento in corso")),
    ] {
        lines.push(format!("{:<16}  {}", tr(keys), tr(what)));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_shortcuts_match_specs() {
        let all = [
            Cmd::NewEmpty,
            Cmd::NewTest,
            Cmd::Open,
            Cmd::Save,
            Cmd::SaveAs,
            Cmd::Import,
            Cmd::Export,
            Cmd::Exit,
            Cmd::Undo,
            Cmd::Redo,
            Cmd::Duplicate,
            Cmd::Delete,
            Cmd::Hide,
            Cmd::Isolate,
            Cmd::ShowAll,
            Cmd::Preferences,
            Cmd::ResetPreferences,
            Cmd::PrepareScene,
            Cmd::FitScene,
            Cmd::UndoPreset,
            Cmd::DropToBase,
            Cmd::FrameProduct,
            Cmd::FrameAll,
            Cmd::FrameSelection,
            Cmd::ToggleOrtho,
            Cmd::Isometric,
            Cmd::ShowViewport,
            Cmd::ShowRender,
            Cmd::GizmoSelect,
            Cmd::GizmoMove,
            Cmd::GizmoRotate,
            Cmd::GizmoScale,
            Cmd::RenderToggle,
            Cmd::Stop,
            Cmd::Restart,
            Cmd::AutoRestart,
            Cmd::LibraryMine,
            Cmd::LibraryOnline,
            Cmd::LibraryFolder,
            Cmd::AiControl,
            Cmd::Manual,
            Cmd::Shortcuts,
            Cmd::Welcome,
        ];
        let mut ids: Vec<&str> = all.iter().map(|c| spec(*c).id).collect();
        ids.sort();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n, "id duplicati");
        // Ogni scorciatoia della tabella è quella scritta nel menu (o un'alternativa nota).
        for (m, k, c) in SHORTCUTS {
            let alt = matches!((c, k), (Cmd::Redo, Key::Z));
            assert!(alt || spec(*c).shortcut == Some(KeyboardShortcut::new(*m, *k)), "{c:?}");
        }
        for (k, c) in GIZMO_KEYS {
            assert_eq!(spec(*c).shortcut, Some(KeyboardShortcut::new(NONE, *k)));
        }
    }

    #[test]
    fn specific_shortcuts_come_first() {
        // Una scorciatoia con più modificatori deve precedere la stessa lettera con meno.
        for (i, (m, k, _)) in SHORTCUTS.iter().enumerate() {
            for (m2, k2, _) in &SHORTCUTS[i + 1..] {
                // m "cattura" anche m2 se m2 ha gli stessi tasti di m più Maiusc e/o Alt.
                let same_base = m.command == m2.command && m.ctrl == m2.ctrl;
                let superset = (m2.shift || !m.shift) && (m2.alt || !m.alt) && (m2.shift != m.shift || m2.alt != m.alt);
                assert!(!(k == k2 && same_base && superset), "{k:?}: la combinazione più specifica va prima");
            }
        }
    }
}
