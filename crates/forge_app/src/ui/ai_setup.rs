//! Finestra "Integrazione AI" (Strumenti → Integrazione AI…, e al primo clic su "Controllo IA"
//! se nessun assistente è collegato): stato del server MCP di ForgeRender e, per ogni
//! assistente, se è installato, collegato o da aggiornare, con i pulsanti per collegarlo o
//! scollegarlo. Niente terminale né percorsi da copiare: la logica è in `mcp::setup`.

use crate::app::ForgeApp;
use crate::i18n::{tf, tr};
use crate::mcp::setup::{self, Assistant, Env, State};
use crate::ui::style;
use eframe::egui;
use std::path::PathBuf;

pub struct AiSetupUi {
    pub open: bool,
    /// Aperta dal primo clic su "Controllo IA": al primo collegamento riuscito il controllo si accende.
    first_use: bool,
    env: Option<Env>,
    /// Ponte nella cartella fissa (o perché non c'è).
    bridge: Result<PathBuf, String>,
    states: Vec<(Assistant, State)>,
    /// Ultimo esito (testo, errore?).
    message: Option<(String, bool)>,
    /// Claude Desktop appena collegato: va riavviato una volta.
    restart_desktop: bool,
    confirm_restart: bool,
}

impl Default for AiSetupUi {
    fn default() -> Self {
        Self::new()
    }
}

impl AiSetupUi {
    /// All'avvio: copia (o aggiorna) il ponte nella cartella fissa.
    pub fn new() -> Self {
        let env = Env::current();
        let bridge = match (&env, setup::bridge_source()) {
            (Some(env), Some(src)) => setup::install_bridge(env, &src),
            (Some(env), None) if env.bridge().is_file() => Ok(env.bridge()),
            (None, _) => Err("cartella dell'utente non trovata".into()),
            (Some(_), None) => Err("forgerender-mcp non è accanto al programma".into()),
        };
        match &bridge {
            Ok(p) => tracing::info!("ponte MCP: {}", p.display()),
            Err(e) => tracing::warn!("ponte MCP: {e}"),
        }
        Self {
            open: false,
            first_use: false,
            env,
            bridge,
            states: Vec::new(),
            message: None,
            restart_desktop: false,
            confirm_restart: false,
        }
    }

    pub fn open(&mut self) {
        self.open = true;
        self.refresh();
    }

    pub fn open_for_first_use(&mut self) {
        self.first_use = true;
        self.message = Some((
            tr("Prima collega l'assistente che vuoi usare: un clic, poi il Controllo IA si accende da solo.").into(),
            false,
        ));
        self.open();
    }

    pub fn refresh(&mut self) {
        self.states = match &self.env {
            Some(env) => Assistant::ALL.iter().map(|&a| (a, setup::state(env, a))).collect(),
            None => Vec::new(),
        };
    }

    /// C'è almeno un assistente collegato al ponte della cartella fissa?
    pub fn any_configured(&mut self) -> bool {
        if self.states.is_empty() {
            self.refresh();
        }
        self.states.iter().any(|(_, s)| *s == State::Configured)
    }
}

fn badge(ui: &mut egui::Ui, text: &str, color: egui::Color32) {
    egui::Frame::new()
        .fill(color.gamma_multiply(0.18))
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(7, 2))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).color(color).small().strong());
        });
}

const GREEN: egui::Color32 = egui::Color32::from_rgb(110, 200, 120);
const GREY: egui::Color32 = egui::Color32::from_rgb(160, 160, 160);
const RED: egui::Color32 = egui::Color32::from_rgb(230, 110, 100);

/// Azione chiesta da un pulsante (si esegue dopo il disegno).
enum Action {
    Connect(Assistant),
    Disconnect(Assistant),
    Refresh,
    ToggleControl,
    RestartDesktop,
}

pub fn window(app: &mut ForgeApp, ctx: &egui::Context) {
    if !app.ai_setup.open {
        return;
    }
    let mut open = true;
    let mut action = None;
    // Assistenti collegati adesso (nomi con cui si sono presentati al ponte).
    let connected: Vec<Option<Assistant>> = app
        .mcp
        .as_ref()
        .map(|s| s.assistants.lock().map(|a| a.clone()).unwrap_or_default())
        .unwrap_or_default()
        .iter()
        .map(|n| Assistant::from_client_name(n))
        .collect();
    let control_on = app.mcp.is_some();
    egui::Window::new(tr("Integrazione AI"))
        .id(egui::Id::NULL.with("win-integrazione-ai"))
        .open(&mut open)
        .resizable(false)
        .default_width(460.0)
        .show(ctx, |ui| {
            let s = &app.ai_setup;
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.strong(tr("Server MCP di ForgeRender"));
                    ui.label(
                        egui::RichText::new(tr("Permette agli assistenti AI di controllare ForgeRender"))
                            .small()
                            .weak(),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let label = if control_on { tr("Spegni") } else { tr("Accendi") };
                    if ui
                        .button(label)
                        .on_hover_text(tr("Controllo IA: quando è spento nessun assistente può toccare la scena"))
                        .clicked()
                    {
                        action = Some(Action::ToggleControl);
                    }
                    if control_on {
                        badge(ui, tr("Attivo"), GREEN);
                    } else {
                        badge(ui, tr("Spento"), GREY);
                    }
                });
            });
            match &s.bridge {
                Ok(p) => ui.label(egui::RichText::new(tf("Ponte: {}", &[p.display().to_string()])).small().weak()),
                Err(e) => ui.colored_label(RED, tf("Ponte non installato: {}", std::slice::from_ref(e))),
            };
            ui.separator();
            for (a, state) in &s.states {
                let a = *a;
                let is_connected = connected.contains(&Some(a));
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(a.name()).size(15.0));
                    let (text, color, hint) = match state {
                        State::NotInstalled => (tr("Non installato"), GREY, tr("L'assistente non è su questo PC")),
                        State::NotConfigured => (tr("Non configurato"), GREY, ""),
                        State::Outdated(_) => (
                            tr("Da aggiornare"),
                            style::WARN,
                            tr("Registrato con un ponte di un'altra cartella (versione vecchia): premi Aggiorna"),
                        ),
                        State::Configured => (tr("Configurato"), GREEN, ""),
                        State::Unsupported => (
                            tr("Non supportato"),
                            GREY,
                            tr("ChatGPT collega solo server remoti su internet, non programmi del PC. Per OpenAI usa Codex"),
                        ),
                        State::Broken(_) => (
                            tr("File illeggibile"),
                            RED,
                            tr("La configurazione dell'assistente non è un JSON valido: non la modifico"),
                        ),
                    };
                    if is_connected {
                        badge(ui, tr("Connesso"), GREEN);
                    } else {
                        badge(ui, text, color);
                    }
                    if !hint.is_empty() {
                        ui.label(egui::RichText::new(crate::ui::icons::INFO).weak()).on_hover_text(hint);
                    }
                    // Pulsanti da destra: Scollega (se registrato), poi Collega o Aggiorna.
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let registered = matches!(state, State::Configured | State::Outdated(_));
                        if registered && ui.button(tr("Scollega")).clicked() {
                            action = Some(Action::Disconnect(a));
                        }
                        let connect = match state {
                            State::NotConfigured => Some(ui.button(tr("Collega"))),
                            State::Outdated(old) => Some(ui.button(tr("Aggiorna")).on_hover_text(tf("Ora: {}", std::slice::from_ref(old)))),
                            _ => None,
                        };
                        if connect.is_some_and(|r| r.clicked()) {
                            action = Some(Action::Connect(a));
                        }
                    });
                });
                if let State::Broken(e) = state {
                    ui.label(egui::RichText::new(e).small().color(RED));
                }
            }
            if s.restart_desktop {
                ui.separator();
                if s.confirm_restart {
                    ui.label(tr(
                        "Claude Desktop si chiude e si riapre: le chat in corso si interrompono (restano salvate). Procedo?",
                    ));
                    ui.horizontal(|ui| {
                        if ui.button(tr("Riavvia ora")).clicked() {
                            action = Some(Action::RestartDesktop);
                        }
                        if ui.button(tr("Lo faccio dopo")).clicked() {
                            app.ai_setup.confirm_restart = false;
                            app.ai_setup.restart_desktop = false;
                        }
                    });
                } else if ui
                    .button(tr("Riavvia Claude Desktop"))
                    .on_hover_text(tr("Serve solo questa volta: Claude Desktop legge i collegamenti quando parte"))
                    .clicked()
                {
                    app.ai_setup.confirm_restart = true;
                }
            }
            let s = &app.ai_setup;
            if let Some((msg, err)) = &s.message {
                ui.separator();
                let color = if *err { RED } else { ui.visuals().text_color() };
                ui.label(egui::RichText::new(msg).color(color));
            }
            ui.separator();
            if ui.button(tr("Controlla di nuovo")).clicked() {
                action = Some(Action::Refresh);
            }
        });
    app.ai_setup.open = open;
    if !open {
        app.ai_setup.first_use = false;
    }
    let Some(action) = action else { return };
    let s = &mut app.ai_setup;
    match action {
        Action::Refresh => {
            s.refresh();
            s.message = None;
        }
        Action::ToggleControl => crate::mcp::set_enabled(app, ctx, !control_on),
        Action::Connect(a) => {
            let Some(env) = s.env.clone() else { return };
            match setup::connect(&env, a) {
                Ok(next) => {
                    s.message = Some((
                        tf("{} collegato. {}", &[a.name().to_string(), crate::i18n::tr_dyn(&next).to_string()]),
                        false,
                    ));
                    s.restart_desktop = a == Assistant::ClaudeDesktop && setup::claude_desktop_running();
                    s.confirm_restart = false;
                    s.refresh();
                    // Prima volta: il controllo si accende da solo.
                    if s.first_use && app.mcp.is_none() {
                        app.ai_setup.first_use = false;
                        crate::mcp::set_enabled(app, ctx, true);
                    }
                }
                Err(e) => {
                    s.message = Some((tf("{}: {}", &[a.name().to_string(), e]), true));
                    s.refresh();
                }
            }
        }
        Action::Disconnect(a) => {
            let Some(env) = s.env.clone() else { return };
            s.message = Some(match setup::disconnect(&env, a) {
                Ok(()) => (tf("{} scollegato.", &[a.name().to_string()]), false),
                Err(e) => (tf("{}: {}", &[a.name().to_string(), e]), true),
            });
            s.refresh();
        }
        Action::RestartDesktop => {
            s.confirm_restart = false;
            s.restart_desktop = false;
            s.message = Some(match setup::restart_claude_desktop() {
                Ok(()) => (tr("Claude Desktop riavviato: ogni chat vede ForgeRender.").into(), false),
                Err(e) => (e, true),
            });
        }
    }
}
