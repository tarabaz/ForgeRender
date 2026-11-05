//! Pannello della timeline (si attiva da Funzioni → Timeline, o da solo quando nella scena c'è un
//! effetto che si calcola nel tempo): sotto, al posto della console (scheda Console | Timeline),
//! oppure staccato in una finestra a parte.
//!
//! - In alto: riproduzione (inizio, chiave prima, play/pausa, chiave dopo, fine, ripeti), tempo,
//!   durata e fotogrammi al secondo, "+ Elemento", zoom, render dell'animazione, stacca.
//! - A sinistra gli elementi con le loro proprietà: rombo per mettere o togliere la chiave
//!   nell'istante del cursore, valore (cambiandolo con le chiavi si registra una chiave), "+" per i
//!   modificatori. "+ Proprietà" aggiunge le proprietà compatibili non di base.
//! - A destra il righello (clic o trascinamento = sposta il cursore) e le chiavi: trascinandole
//!   si spostano (a scatti di fotogramma), clic destro = interpolazione o elimina. Gli effetti
//!   calcolati nel tempo hanno una barra che parte dal loro inizio: trascinandola cambia l'inizio.
//!
//! Spostare un oggetto con il gizmo (o cambiare un valore nei pannelli) quando la sua proprietà
//! ha già delle chiavi registra una chiave nell'istante del cursore.

use crate::app::ForgeApp;
use crate::i18n::{tf, tr};
use crate::ui::{icons, style};
use eframe::egui;
use forge_scene::{Interp, ModSlot, Modifier, Prop, Target};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const ROW: f32 = 20.0;
const LEFT: f32 = 360.0;

/// Stato del pannello e della riproduzione (non salvato nel progetto).
pub struct TimelineUi {
    pub playing: bool,
    last_tick: Option<Instant>,
    pub loop_play: bool,
    /// Zoom del tempo (1 = tutta la durata nella larghezza) e primo secondo in vista.
    zoom: f32,
    t0: f32,
    drag_key: Option<(usize, usize)>,
    drag_start: Option<Target>,
    /// Effetti nel tempo visti l'ultima volta (per accendere la timeline quando ne arriva uno).
    seen_effects: usize,
    pub run: Option<AnimRun>,
    /// Finestra staccata da riportare davanti.
    raise: bool,
    /// Animazione di prova da creare appena c'è la scena (`--timeline-demo`).
    pub demo_pending: bool,
}

impl Default for TimelineUi {
    fn default() -> Self {
        Self {
            playing: false,
            last_tick: None,
            loop_play: true,
            zoom: 1.0,
            t0: 0.0,
            drag_key: None,
            drag_start: None,
            seen_effects: 0,
            run: None,
            raise: false,
            demo_pending: false,
        }
    }
}

/// Render dell'animazione in corso: un'immagine per fotogramma, poi il video.
pub struct AnimRun {
    dir: PathBuf,
    frame: usize,
    total: usize,
    rendering: bool,
    written: Vec<PathBuf>,
    settings: forge_render_core::RenderSettings,
    auto_restart: bool,
    view: crate::app::View,
    time: f32,
    started: Instant,
    waiting_denoise: Option<Instant>,
    make_video: bool,
    video: Option<std::thread::JoinHandle<anyhow::Result<&'static str>>>,
    cancelled: bool,
}

impl AnimRun {
    pub fn progress(&self) -> (usize, usize) {
        (self.frame.min(self.total), self.total)
    }
}

/// Effetti che si calcolano nel tempo nella scena (fumo simulato, ghiaccio secco).
fn time_effects(app: &ForgeApp) -> usize {
    app.scene
        .smokes
        .iter()
        .filter(|s| s.mode == forge_scene::SmokeMode::Simulated)
        .count()
        + app.scene.fogs.len()
}

/// Accende o spegne la funzione Timeline (resta nelle impostazioni).
pub fn set_enabled(app: &mut ForgeApp, on: bool) {
    if app.prefs.timeline_enabled == on {
        return;
    }
    let p = crate::prefs::Preferences {
        timeline_enabled: on,
        ..app.prefs.clone()
    };
    app.apply_prefs(p, false);
    if on {
        app.bottom_tab = crate::app::BottomTab::Timeline;
        // L'istante degli effetti nel tempo resta quello di prima: il cursore va lì.
        let t = app
            .scene
            .smokes
            .iter()
            .filter(|s| s.mode == forge_scene::SmokeMode::Simulated)
            .map(|s| s.sim.time)
            .chain(app.scene.fogs.iter().map(|f| f.sim_time))
            .fold(0.0f32, f32::max);
        if t > 0.0 {
            let tl = &mut app.scene.timeline;
            tl.duration = tl.duration.max(t);
            tl.time = t;
        }
    } else {
        app.timeline_ui.playing = false;
    }
}

/// A ogni fotogramma dell'interfaccia: accensione automatica, riproduzione, applicazione dei
/// valori animati, render dell'animazione.
pub fn tick(app: &mut ForgeApp, ctx: &egui::Context) {
    let effects = time_effects(app);
    if effects > app.timeline_ui.seen_effects && !app.prefs.timeline_enabled {
        set_enabled(app, true);
        app.toast(tr(
            "Timeline attivata: nella scena c'è un effetto che si calcola nel tempo (fumo simulato, ghiaccio secco)",
        ));
    }
    app.timeline_ui.seen_effects = effects;
    if !app.prefs.timeline_enabled {
        app.timeline_ui.last_tick = None;
        return;
    }
    if app.timeline_ui.demo_pending && !app.scene.roots.is_empty() {
        app.timeline_ui.demo_pending = false;
        demo(app);
    }
    if app.timeline_ui.run.is_some() {
        tick_render(app);
        ctx.request_repaint_after(Duration::from_millis(50));
        return;
    }
    let now = Instant::now();
    if app.timeline_ui.playing {
        let dt = app
            .timeline_ui
            .last_tick
            .map(|l| now - l)
            .unwrap_or_default()
            .as_secs_f32()
            .min(0.25);
        let tl = &mut app.scene.timeline;
        let mut t = tl.time + dt;
        if t > tl.duration {
            if app.timeline_ui.loop_play {
                t = 0.0;
            } else {
                t = tl.duration;
                app.timeline_ui.playing = false;
            }
        }
        tl.time = t;
        ctx.request_repaint();
    }
    app.timeline_ui.last_tick = Some(now);
    apply(app);
}

/// Animazione di prova (`--timeline-demo`): il primo oggetto con una mesh sale e gira con
/// interpolazioni diverse e un tremolio, la camera si avvicina.
fn demo(app: &mut ForgeApp) {
    use forge_scene::Key;
    let Some(node) = (0..app.scene.nodes.len()).find(|&i| app.scene.nodes[i].mesh.is_some()) else {
        return;
    };
    let target = Target::Object(app.scene.nodes[node].id.clone());
    let mut tl = std::mem::take(&mut app.scene.timeline);
    tl.duration = 4.0;
    tl.add_element(&app.scene, target.clone());
    tl.add_element(&app.scene, Target::Camera);
    let key = |t: f32, v: f32, interp: Interp| Key { t, v, interp };
    let y = tl.track(&target, Prop::PosY).expect("traccia");
    let base = tl.tracks[y].base;
    tl.tracks[y].keys = vec![
        key(0.0, base, Interp::EaseInOut),
        key(1.5, base + 0.3, Interp::Smooth),
        key(3.0, base + 0.1, Interp::EaseOut),
        key(4.0, base, Interp::Linear),
    ];
    let r = tl.track(&target, Prop::RotY).expect("traccia");
    tl.tracks[r].modifiers.push(ModSlot {
        modifier: Modifier::Ramp { per_second: 90.0 },
        enabled: true,
    });
    let x = tl.track(&target, Prop::PosX).expect("traccia");
    tl.tracks[x].modifiers.push(ModSlot {
        modifier: Modifier::Wiggle {
            amplitude: 0.03,
            frequency: 2.0,
            seed: 3,
        },
        enabled: true,
    });
    let o = tl.track(&target, Prop::Opacity).expect("traccia");
    tl.tracks[o].keys = vec![
        key(0.0, 1.0, Interp::Linear),
        key(2.0, 0.35, Interp::Hold),
        key(3.2, 1.0, Interp::Linear),
    ];
    let cz = tl.track(&Target::Camera, Prop::PosZ).expect("traccia");
    let cb = tl.tracks[cz].base;
    tl.tracks[cz].keys = vec![key(0.0, cb, Interp::EaseInOut), key(4.0, cb * 0.8, Interp::EaseInOut)];
    tl.time = 1.2;
    app.scene.timeline = tl;
}

/// Applica la timeline alla scena e segnala cosa va rifatto (senza segnare il progetto come
/// modificato: cambia solo l'istante mostrato).
pub fn apply(app: &mut ForgeApp) {
    let unsaved = app.unsaved;
    let mut tl = std::mem::take(&mut app.scene.timeline);
    let a = tl.apply(&mut app.scene);
    app.scene.timeline = tl;
    if !a.any() {
        return;
    }
    if a.geometry {
        app.geometry_changed();
    }
    if a.shading || a.effects {
        app.appearance_changed();
    }
    if a.camera {
        if app.scene.active_camera.is_none() {
            app.sync_orbit_from_camera();
        }
        app.camera_changed();
    }
    app.unsaved = unsaved;
}

/// Il pannello: dentro la parte bassa o in una finestra a parte.
pub fn panel(app: &mut ForgeApp, ui: &mut egui::Ui) {
    toolbar(app, ui);
    ui.separator();
    if app.scene.timeline.elements.is_empty() {
        ui.label(
            egui::RichText::new(tr(
                "Nessun elemento: con «+ Elemento» aggiungi gli oggetti selezionati, la camera, una luce o un effetto. \
                 Poi sposta il cursore, cambia un valore e premi il rombo per mettere una chiave.",
            ))
            .weak(),
        );
    }
    let lane_w = (ui.available_width() - LEFT - 8.0).max(80.0);
    let scale = TimeScale::new(&app.scene.timeline, &app.timeline_ui, lane_w);
    ruler(app, ui, &scale);
    egui::ScrollArea::vertical()
        .id_salt("timeline-righe")
        .auto_shrink([false, false])
        .show(ui, |ui| rows(app, ui, &scale));
}

/// Finestra staccata (se la timeline è staccata dal programma).
pub fn window(app: &mut ForgeApp, ctx: &egui::Context) {
    if !app.prefs.timeline_enabled || !app.prefs.timeline_detached {
        return;
    }
    let mut open = true;
    let raise = std::mem::take(&mut app.timeline_ui.raise);
    crate::ui::detachable(
        ctx,
        "win-timeline",
        tr("Timeline"),
        [1200.0, 380.0],
        false,
        raise,
        &mut open,
        |ui| panel(app, ui),
    );
    if !open {
        set_detached(app, false);
    }
}

fn set_detached(app: &mut ForgeApp, detached: bool) {
    let p = crate::prefs::Preferences {
        timeline_detached: detached,
        ..app.prefs.clone()
    };
    app.apply_prefs(p, false);
    app.timeline_ui.raise = detached;
}

/// Da tempo a pixel e ritorno nella colonna delle chiavi.
struct TimeScale {
    x0: f32,
    px_per_s: f32,
    t0: f32,
    width: f32,
}

impl TimeScale {
    fn new(tl: &forge_scene::Timeline, ui_state: &TimelineUi, width: f32) -> Self {
        let dur = tl.duration.max(0.1);
        Self {
            x0: 0.0,
            px_per_s: width / dur * ui_state.zoom.max(1.0),
            t0: ui_state.t0,
            width,
        }
    }
    fn x(&self, t: f32) -> f32 {
        self.x0 + (t - self.t0) * self.px_per_s
    }
    fn t(&self, x: f32) -> f32 {
        self.t0 + (x - self.x0) / self.px_per_s
    }
}

fn fmt_time(t: f32, fps: f32) -> String {
    let s = t.max(0.0);
    let frame = ((s.fract()) * fps).floor() as u32;
    format!("{:02}:{:02}.{:02}", (s / 60.0) as u32, (s % 60.0) as u32, frame)
}

fn toolbar(app: &mut ForgeApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        let keys = app.scene.timeline.key_times();
        let t = app.scene.timeline.time;
        let tl_frame = app.scene.timeline.frame();
        if ui.button("⏮").on_hover_text(tr("All'inizio")).clicked() {
            app.scene.timeline.time = 0.0;
        }
        if ui.button("◀◀").on_hover_text(tr("Chiave prima")).clicked() {
            if let Some(k) = keys.iter().rev().find(|&&k| k < t - tl_frame * 0.5) {
                app.scene.timeline.time = *k;
            }
        }
        let play = if app.timeline_ui.playing { icons::PAUSE } else { icons::RENDER };
        if ui
            .button(egui::RichText::new(play).size(16.0))
            .on_hover_text(tr("Riproduci / pausa"))
            .clicked()
        {
            app.timeline_ui.playing = !app.timeline_ui.playing;
            if app.timeline_ui.playing && app.scene.timeline.time >= app.scene.timeline.duration {
                app.scene.timeline.time = 0.0;
            }
        }
        if ui.button("▶▶").on_hover_text(tr("Chiave dopo")).clicked() {
            if let Some(k) = keys.iter().find(|&&k| k > t + tl_frame * 0.5) {
                app.scene.timeline.time = *k;
            }
        }
        if ui.button("⏭").on_hover_text(tr("Alla fine")).clicked() {
            app.scene.timeline.time = app.scene.timeline.duration;
        }
        ui.toggle_value(&mut app.timeline_ui.loop_play, "⟲")
            .on_hover_text(tr("Ripeti: arrivata alla fine la riproduzione riparte dall'inizio"));
        ui.separator();
        let tl = &mut app.scene.timeline;
        ui.label(
            egui::RichText::new(format!("{} / {}", fmt_time(tl.time, tl.fps), fmt_time(tl.duration, tl.fps)))
                .monospace()
                .strong(),
        );
        ui.separator();
        ui.label(tr("Durata"));
        let mut changed = ui
            .add(egui::DragValue::new(&mut tl.duration).range(0.1..=3600.0).speed(0.05).suffix(" s"))
            .on_hover_text(tr("Quanto dura l'animazione (e fino a quando si calcolano fumi e nebbie)"))
            .changed();
        ui.label(tr("FPS"));
        changed |= ui
            .add(egui::DragValue::new(&mut tl.fps).range(1.0..=120.0).speed(0.2))
            .on_hover_text(tr("Fotogrammi al secondo del render dell'animazione"))
            .changed();
        if changed {
            tl.time = tl.time.min(tl.duration);
            app.unsaved = true;
        }
        ui.separator();
        add_element_menu(app, ui);
        ui.separator();
        ui.label(tr("Zoom"));
        ui.add(
            egui::Slider::new(&mut app.timeline_ui.zoom, 1.0..=20.0)
                .logarithmic(true)
                .show_value(false),
        );
        if app.timeline_ui.zoom > 1.0 {
            let max_t0 = app.scene.timeline.duration * (1.0 - 1.0 / app.timeline_ui.zoom);
            ui.add(egui::Slider::new(&mut app.timeline_ui.t0, 0.0..=max_t0.max(0.0)).show_value(false))
                .on_hover_text(tr("Scorri il tempo"));
        } else {
            app.timeline_ui.t0 = 0.0;
        }
        ui.separator();
        match &app.timeline_ui.run {
            Some(r) => {
                let (f, n) = r.progress();
                ui.label(tf(
                    "Render animazione: fotogramma {} di {}",
                    &[(f + 1).min(n).to_string(), n.to_string()],
                ));
                if ui.button(tr("Interrompi")).clicked() {
                    if let Some(r) = &mut app.timeline_ui.run {
                        r.cancelled = true;
                    }
                }
            }
            None => {
                if ui
                    .button(format!("{}  {}", icons::IMAGE, tr("Render animazione…")))
                    .on_hover_text(tr(
                        "Fa il render di ogni fotogramma della timeline (PNG in una cartella) e poi il video MP4, con i campioni attuali",
                    ))
                    .clicked()
                {
                    start_render(app);
                }
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (label, hint) = if app.prefs.timeline_detached {
                (
                    format!("{}  {}", icons::BACK, tr("Riporta nel programma")),
                    tr("Rimette la timeline in basso, al posto della console"),
                )
            } else {
                (
                    format!("{}  {}", icons::EXPORT, tr("Stacca dal programma")),
                    tr("Apre la timeline in una finestra a parte, da spostare anche su un altro schermo"),
                )
            };
            if ui.button(label).on_hover_text(hint).clicked() {
                let d = !app.prefs.timeline_detached;
                set_detached(app, d);
            }
        });
    });
}

fn add_element_menu(app: &mut ForgeApp, ui: &mut egui::Ui) {
    let mut add: Vec<Target> = Vec::new();
    ui.menu_button(format!("+ {}", tr("Elemento")), |ui| {
        let sel: Vec<Target> = app
            .selection
            .iter()
            .map(|&i| Target::Object(app.scene.nodes[i].id.clone()))
            .collect();
        if ui
            .add_enabled(
                !sel.is_empty(),
                egui::Button::new(tf("Oggetti selezionati ({})", &[sel.len().to_string()])),
            )
            .clicked()
        {
            add.extend(sel);
            ui.close();
        }
        if ui.button(tr("Camera")).clicked() {
            add.push(Target::Camera);
            ui.close();
        }
        ui.menu_button(tr("Luce"), |ui| {
            for l in &app.scene.lights {
                if ui.button(&l.name).clicked() {
                    add.push(Target::Light(l.name.clone()));
                    ui.close();
                }
            }
        });
        if !app.scene.smokes.is_empty() {
            ui.menu_button(tr("Fumo"), |ui| {
                for s in &app.scene.smokes {
                    if ui.button(&s.name).clicked() {
                        add.push(Target::Smoke(s.n));
                        ui.close();
                    }
                }
            });
        }
        if !app.scene.fogs.is_empty() {
            ui.menu_button(tr("Ghiaccio secco"), |ui| {
                for f in &app.scene.fogs {
                    if ui.button(&f.name).clicked() {
                        add.push(Target::Fog(f.name.clone()));
                        ui.close();
                    }
                }
            });
        }
        if ui.button(tr("Foschia d'ambiente")).clicked() {
            add.push(Target::Haze);
            ui.close();
        }
        if ui.button(tr("Polvere in sospensione")).clicked() {
            add.push(Target::Dust);
            ui.close();
        }
        if ui.button(tr("Ambiente")).clicked() {
            add.push(Target::Environment);
            ui.close();
        }
    });
    if !add.is_empty() {
        let mut tl = std::mem::take(&mut app.scene.timeline);
        for t in add {
            tl.add_element(&app.scene, t);
        }
        app.scene.timeline = tl;
        app.unsaved = true;
    }
}

/// Righello del tempo: clic o trascinamento = sposta il cursore.
fn ruler(app: &mut ForgeApp, ui: &mut egui::Ui, scale: &TimeScale) {
    ui.horizontal(|ui| {
        ui.allocate_exact_size(egui::vec2(LEFT, ROW), egui::Sense::hover());
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(scale.width, ROW), egui::Sense::click_and_drag());
        let s = TimeScale { x0: rect.left(), ..*scale };
        let tl = &mut app.scene.timeline;
        if let Some(p) = resp.interact_pointer_pos() {
            tl.time = tl.snap(s.t(p.x));
            app.timeline_ui.playing = false;
        }
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 2.0, ui.visuals().faint_bg_color);
        // Tacche: ogni secondo (o mezzo, o 5 s) secondo lo zoom.
        let step = [0.1f32, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0]
            .into_iter()
            .find(|st| st * s.px_per_s >= 60.0)
            .unwrap_or(60.0);
        let mut k = (s.t0 / step).floor() * step;
        let end = s.t(rect.right());
        while k <= end {
            let x = s.x(k);
            if x >= rect.left() {
                painter.line_segment(
                    [egui::pos2(x, rect.bottom() - 6.0), egui::pos2(x, rect.bottom())],
                    ui.visuals().widgets.noninteractive.fg_stroke,
                );
                painter.text(
                    egui::pos2(x + 2.0, rect.top() + 1.0),
                    egui::Align2::LEFT_TOP,
                    format!("{k:.1}s"),
                    egui::FontId::proportional(10.0),
                    ui.visuals().weak_text_color(),
                );
            }
            k += step;
        }
        // Fine della durata.
        let xe = s.x(tl.duration);
        if xe < rect.right() {
            painter.rect_filled(
                egui::Rect::from_min_max(egui::pos2(xe, rect.top()), rect.max),
                0.0,
                egui::Color32::from_black_alpha(90),
            );
        }
        playhead(&painter, &s, tl.time, rect);
    });
}

fn playhead(painter: &egui::Painter, s: &TimeScale, t: f32, rect: egui::Rect) {
    let x = s.x(t);
    if x >= rect.left() - 1.0 && x <= rect.right() + 1.0 {
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(1.5, style::ACCENT),
        );
    }
}

/// Azioni raccolte durante il disegno (applicate dopo, quando la scena è libera).
enum Action {
    RemoveElement(Target),
    AddProp(Target, Prop),
    RemoveTrack(usize),
    ToggleKey(usize),
    SetValue(usize, f32),
    AddModifier(usize, Modifier),
    RemoveModifier(usize, usize),
    DeleteKey(usize, usize),
    SetInterp(usize, usize, Interp),
    MoveKey(usize, usize, f32),
    SetStart(Target, f32),
}

fn rows(app: &mut ForgeApp, ui: &mut egui::Ui, scale: &TimeScale) {
    let mut actions: Vec<Action> = Vec::new();
    let elements = app.scene.timeline.elements.clone();
    for target in &elements {
        element_rows(app, ui, scale, target, &mut actions);
    }
    if actions.is_empty() {
        return;
    }
    let time = app.scene.timeline.time;
    let mut tl = std::mem::take(&mut app.scene.timeline);
    let tol = tl.frame() * 0.5;
    let mut push_scene = Vec::new();
    for a in actions {
        match a {
            Action::RemoveElement(t) => tl.remove_element(&t),
            Action::AddProp(t, p) => {
                tl.add_track(&app.scene, t, p);
            }
            Action::RemoveTrack(i) => {
                tl.tracks.remove(i);
            }
            Action::ToggleKey(i) => {
                let tr = &mut tl.tracks[i];
                match tr.key_near(time, tol) {
                    Some(k) => {
                        tr.keys.remove(k);
                        if tr.keys.is_empty() {
                            // Senza chiavi resta il valore attuale.
                            tr.base = forge_scene::timeline::get(&app.scene, &tr.target, tr.prop).unwrap_or(tr.base);
                        }
                    }
                    None => {
                        let v = forge_scene::timeline::get(&app.scene, &tr.target, tr.prop).unwrap_or(tr.base);
                        tr.set_key(time, v, tol);
                    }
                }
            }
            Action::SetValue(i, v) => {
                let tr = &mut tl.tracks[i];
                if tr.keys.is_empty() {
                    tr.base = v;
                } else {
                    tr.set_key(time, v, tol);
                }
                push_scene.push((tr.target.clone(), tr.prop, v));
                tr.last = None;
            }
            Action::AddModifier(i, m) => tl.tracks[i].modifiers.push(ModSlot {
                modifier: m,
                enabled: true,
            }),
            Action::RemoveModifier(i, k) => {
                tl.tracks[i].modifiers.remove(k);
            }
            Action::DeleteKey(i, k) => {
                tl.tracks[i].keys.remove(k);
            }
            Action::SetInterp(i, k, interp) => tl.tracks[i].keys[k].interp = interp,
            Action::MoveKey(i, k, t) => {
                let t = tl.snap(t);
                tl.tracks[i].keys[k].t = t;
                tl.tracks[i].sort();
            }
            Action::SetStart(t, s) => tl.set_start(&t, tl.snap(s.max(0.0))),
        }
    }
    for (t, p, v) in push_scene {
        forge_scene::timeline::set(&mut app.scene, &t, p, v);
    }
    app.scene.timeline = tl;
    app.unsaved = true;
    // Rimette i valori dell'istante attuale (e segnala cosa va rifatto).
    for tr in &mut app.scene.timeline.tracks {
        tr.last = None;
    }
    apply(app);
    app.unsaved = true;
}

fn element_rows(app: &mut ForgeApp, ui: &mut egui::Ui, scale: &TimeScale, target: &Target, actions: &mut Vec<Action>) {
    let name = target.name(&app.scene);
    let id = ui.make_persistent_id(("timeline-elemento", format!("{target:?}")));
    let mut open = ui.ctx().data(|d| d.get_temp::<bool>(id)).unwrap_or(true);
    let is_effect = matches!(target, Target::Smoke(_) | Target::Fog(_))
        && match target {
            Target::Smoke(n) => app
                .scene
                .smokes
                .iter()
                .any(|s| s.n == *n && s.mode == forge_scene::SmokeMode::Simulated),
            _ => true,
        };
    // Riga dell'elemento.
    ui.horizontal(|ui| {
        let (left, _) = ui.allocate_exact_size(egui::vec2(LEFT, ROW), egui::Sense::hover());
        let mut lui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(left)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        if collapse_button(&mut lui, open).clicked() {
            open = !open;
            lui.ctx().data_mut(|d| d.insert_temp(id, open));
        }
        lui.label(egui::RichText::new(&name).strong());
        lui.menu_button("+", |ui| {
            ui.label(egui::RichText::new(tr("Aggiungi una proprietà")).weak());
            for &p in target.all_props() {
                let has = app.scene.timeline.track(target, p).is_some();
                if ui.add_enabled(!has, egui::Button::new(tr(p.label()))).clicked() {
                    actions.push(Action::AddProp(target.clone(), p));
                    ui.close();
                }
            }
        })
        .response
        .on_hover_text(tr("Aggiungi una proprietà da animare"));
        if lui
            .small_button(icons::DELETE)
            .on_hover_text(tr("Togli dalla timeline (le chiavi si perdono)"))
            .clicked()
        {
            actions.push(Action::RemoveElement(target.clone()));
        }
        if is_effect {
            let mut start = app.scene.timeline.start_of(target);
            if lui
                .add(
                    egui::DragValue::new(&mut start)
                        .range(0.0..=3600.0)
                        .speed(0.02)
                        .prefix(tr("inizio "))
                        .suffix(" s"),
                )
                .on_hover_text(tr("Quando parte l'effetto: prima non c'è, poi si calcola da zero"))
                .changed()
            {
                actions.push(Action::SetStart(target.clone(), start));
            }
        }
        // Colonna del tempo: barra dell'effetto (dal suo inizio) o fascia dell'elemento.
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(scale.width, ROW), egui::Sense::click_and_drag());
        let s = TimeScale { x0: rect.left(), ..*scale };
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, ui.visuals().faint_bg_color);
        if is_effect {
            let x0 = s.x(app.scene.timeline.start_of(target)).max(rect.left());
            let x1 = s.x(app.scene.timeline.duration).min(rect.right());
            if x1 > x0 {
                let bar = egui::Rect::from_min_max(egui::pos2(x0, rect.top() + 3.0), egui::pos2(x1, rect.bottom() - 3.0));
                painter.rect_filled(bar, 3.0, style::ACCENT.gamma_multiply(0.45));
                painter.text(
                    bar.left_center() + egui::vec2(4.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    tr("calcolo nel tempo"),
                    egui::FontId::proportional(10.0),
                    egui::Color32::WHITE,
                );
            }
            if resp.drag_started() {
                app.timeline_ui.drag_start = Some(target.clone());
            }
            if app.timeline_ui.drag_start.as_ref() == Some(target) {
                if let Some(p) = resp.interact_pointer_pos() {
                    actions.push(Action::SetStart(target.clone(), s.t(p.x)));
                }
                if resp.drag_stopped() {
                    app.timeline_ui.drag_start = None;
                }
            }
            resp.on_hover_text(tr("Trascina per cambiare quando parte l'effetto"));
        }
        playhead(&painter, &s, app.scene.timeline.time, rect);
    });
    if !open {
        return;
    }
    // Proprietà: prima quelle di base, poi le aggiunte, nell'ordine dell'elenco compatibile.
    let tracks: Vec<usize> = target
        .all_props()
        .iter()
        .filter_map(|&p| app.scene.timeline.track(target, p))
        .collect();
    for i in tracks {
        prop_row(app, ui, scale, i, actions);
        let mods = app.scene.timeline.tracks[i].modifiers.len();
        for k in 0..mods {
            modifier_row(app, ui, scale, i, k, actions);
        }
    }
}

fn prop_row(app: &mut ForgeApp, ui: &mut egui::Ui, scale: &TimeScale, i: usize, actions: &mut Vec<Action>) {
    let time = app.scene.timeline.time;
    let tol = app.scene.timeline.frame() * 0.5;
    let tr_ = app.scene.timeline.tracks[i].clone();
    let base = tr_.target.base_props().contains(&tr_.prop);
    ui.horizontal(|ui| {
        let (left, _) = ui.allocate_exact_size(egui::vec2(LEFT, ROW), egui::Sense::hover());
        let mut lui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(left)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        lui.add_space(18.0);
        let on_key = tr_.key_near(time, tol).is_some();
        let color = if tr_.keys.is_empty() && !on_key {
            lui.visuals().weak_text_color()
        } else {
            style::ACCENT
        };
        if key_button(&mut lui, on_key, color)
            .on_hover_text(if on_key {
                tr("Togli la chiave in questo istante")
            } else {
                tr("Metti una chiave in questo istante con il valore attuale")
            })
            .clicked()
        {
            actions.push(Action::ToggleKey(i));
        }
        lui.add_sized(egui::vec2(130.0, ROW), egui::Label::new(tr(tr_.prop.label())).truncate());
        let mut v = forge_scene::timeline::get(&app.scene, &tr_.target, tr_.prop).unwrap_or(tr_.base);
        let r = lui.add(
            egui::DragValue::new(&mut v)
                .speed(tr_.prop.speed())
                .max_decimals(3)
                .suffix(tr_.prop.unit()),
        );
        if r.changed() {
            actions.push(Action::SetValue(i, v));
        }
        lui.menu_button("ƒ+", |ui| {
            ui.label(egui::RichText::new(tr("Aggiungi un modificatore")).weak());
            for m in Modifier::all_defaults() {
                if ui.button(tr(m.label())).on_hover_text(tr(m.hint())).clicked() {
                    actions.push(Action::AddModifier(i, m));
                    ui.close();
                }
            }
        })
        .response
        .on_hover_text(tr("Modificatori: tremolio, oscillazione, rampa, ripeti, avanti e indietro..."));
        if !base
            && lui
                .small_button(icons::CLOSE)
                .on_hover_text(tr("Togli questa proprietà dalla timeline"))
                .clicked()
        {
            actions.push(Action::RemoveTrack(i));
        }
        // Chiavi.
        let (rect, _) = ui.allocate_exact_size(egui::vec2(scale.width, ROW), egui::Sense::hover());
        let s = TimeScale { x0: rect.left(), ..*scale };
        let painter = ui.painter_at(rect);
        painter.line_segment(
            [egui::pos2(rect.left(), rect.center().y), egui::pos2(rect.right(), rect.center().y)],
            egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
        );
        // Tratti animati tra le chiavi.
        for w in tr_.keys.windows(2) {
            let (a, b) = (s.x(w[0].t), s.x(w[1].t));
            let col = if w[0].interp == Interp::Hold {
                ui.visuals().weak_text_color()
            } else {
                style::ACCENT.gamma_multiply(0.5)
            };
            painter.line_segment(
                [egui::pos2(a, rect.center().y), egui::pos2(b, rect.center().y)],
                egui::Stroke::new(2.0, col),
            );
        }
        for (k, key) in tr_.keys.iter().enumerate() {
            let x = s.x(key.t);
            if x < rect.left() - 6.0 || x > rect.right() + 6.0 {
                continue;
            }
            let kr = egui::Rect::from_center_size(egui::pos2(x, rect.center().y), egui::vec2(12.0, 14.0));
            let resp = ui.interact(kr, ui.make_persistent_id(("chiave", i, k)), egui::Sense::click_and_drag());
            let fill = if resp.hovered() || app.timeline_ui.drag_key == Some((i, k)) {
                egui::Color32::WHITE
            } else {
                style::ACCENT
            };
            let c = kr.center();
            painter.add(egui::Shape::convex_polygon(
                vec![
                    c + egui::vec2(0.0, -6.0),
                    c + egui::vec2(5.0, 0.0),
                    c + egui::vec2(0.0, 6.0),
                    c + egui::vec2(-5.0, 0.0),
                ],
                fill,
                egui::Stroke::new(1.0, egui::Color32::BLACK),
            ));
            if resp.drag_started() {
                app.timeline_ui.drag_key = Some((i, k));
            }
            if app.timeline_ui.drag_key == Some((i, k)) {
                if let Some(p) = resp.interact_pointer_pos() {
                    if resp.dragged() {
                        actions.push(Action::MoveKey(i, k, s.t(p.x).clamp(0.0, app.scene.timeline.duration)));
                    }
                }
                if resp.drag_stopped() {
                    app.timeline_ui.drag_key = None;
                }
            }
            if resp.clicked() {
                app.scene.timeline.time = key.t;
            }
            let interp_now = key.interp;
            let resp = resp.on_hover_text(tf(
                "{} s · {} · {}",
                &[format!("{:.2}", key.t), format!("{:.3}", key.v), tr(interp_now.label()).to_string()],
            ));
            resp.context_menu(|ui| {
                ui.label(egui::RichText::new(tr("Interpolazione verso la chiave dopo")).weak());
                for it in Interp::ALL {
                    if ui.add(egui::Button::selectable(interp_now == it, tr(it.label()))).clicked() {
                        actions.push(Action::SetInterp(i, k, it));
                        ui.close();
                    }
                }
                ui.separator();
                if ui.button(format!("{}  {}", icons::DELETE, tr("Elimina la chiave"))).clicked() {
                    actions.push(Action::DeleteKey(i, k));
                    ui.close();
                }
            });
        }
        playhead(&painter, &s, time, rect);
    });
}

/// Triangolo per aprire o chiudere l'elemento (giù = aperto).
fn collapse_button(ui: &mut egui::Ui, open: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(16.0, ROW), egui::Sense::click());
    let c = rect.center();
    let col = if resp.hovered() {
        egui::Color32::WHITE
    } else {
        ui.visuals().text_color()
    };
    let pts = if open {
        vec![c + egui::vec2(-4.5, -2.5), c + egui::vec2(4.5, -2.5), c + egui::vec2(0.0, 3.5)]
    } else {
        vec![c + egui::vec2(-2.5, -4.5), c + egui::vec2(3.5, 0.0), c + egui::vec2(-2.5, 4.5)]
    };
    ui.painter().add(egui::Shape::convex_polygon(pts, col, egui::Stroke::NONE));
    resp
}

/// Rombo della chiave: pieno se c'è una chiave nell'istante del cursore, vuoto se no.
fn key_button(ui: &mut egui::Ui, filled: bool, color: egui::Color32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(16.0, ROW), egui::Sense::click());
    let c = rect.center();
    let col = if resp.hovered() { egui::Color32::WHITE } else { color };
    let pts = vec![
        c + egui::vec2(0.0, -5.5),
        c + egui::vec2(5.0, 0.0),
        c + egui::vec2(0.0, 5.5),
        c + egui::vec2(-5.0, 0.0),
    ];
    let fill = if filled { col } else { egui::Color32::TRANSPARENT };
    ui.painter()
        .add(egui::Shape::convex_polygon(pts, fill, egui::Stroke::new(1.3, col)));
    resp
}

fn modifier_row(app: &mut ForgeApp, ui: &mut egui::Ui, scale: &TimeScale, i: usize, k: usize, actions: &mut Vec<Action>) {
    let time = app.scene.timeline.time;
    let mut changed = false;
    ui.horizontal(|ui| {
        let (left, _) = ui.allocate_exact_size(egui::vec2(LEFT, ROW), egui::Sense::hover());
        let mut lui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(left)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        lui.add_space(36.0);
        let slot = &mut app.scene.timeline.tracks[i].modifiers[k];
        changed |= lui.checkbox(&mut slot.enabled, "").on_hover_text(tr("Acceso / spento")).changed();
        lui.label(egui::RichText::new(format!("ƒ {}", tr(slot.modifier.label()))))
            .on_hover_text(tr(slot.modifier.hint()));
        let dv = |ui: &mut egui::Ui, v: &mut f32, speed: f64, hint: &str| {
            ui.add(egui::DragValue::new(v).speed(speed).max_decimals(3))
                .on_hover_text(hint)
                .changed()
        };
        match &mut slot.modifier {
            Modifier::Wiggle {
                amplitude,
                frequency,
                seed,
            } => {
                changed |= dv(&mut lui, amplitude, 0.01, tr("Ampiezza"));
                changed |= dv(&mut lui, frequency, 0.05, tr("Volte al secondo"));
                changed |= lui
                    .add(egui::DragValue::new(seed).range(0..=9999))
                    .on_hover_text(tr("Seme"))
                    .changed();
            }
            Modifier::Oscillate {
                amplitude,
                frequency,
                phase_deg,
            } => {
                changed |= dv(&mut lui, amplitude, 0.01, tr("Ampiezza"));
                changed |= dv(&mut lui, frequency, 0.02, tr("Volte al secondo"));
                changed |= dv(&mut lui, phase_deg, 1.0, tr("Fase (gradi)"));
            }
            Modifier::Ramp { per_second } => changed |= dv(&mut lui, per_second, 0.5, tr("Unità al secondo")),
            Modifier::Offset { value } => changed |= dv(&mut lui, value, 0.01, tr("Valore aggiunto")),
            Modifier::Multiply { factor } => changed |= dv(&mut lui, factor, 0.01, tr("Fattore")),
            Modifier::Clamp { min, max } => {
                changed |= dv(&mut lui, min, 0.01, tr("Minimo"));
                changed |= dv(&mut lui, max, 0.01, tr("Massimo"));
            }
            Modifier::Steps { per_second } => changed |= dv(&mut lui, per_second, 0.2, tr("Scatti al secondo")),
            Modifier::Loop | Modifier::PingPong => {}
        }
        if lui.small_button(icons::CLOSE).on_hover_text(tr("Togli il modificatore")).clicked() {
            actions.push(Action::RemoveModifier(i, k));
        }
        let (rect, _) = ui.allocate_exact_size(egui::vec2(scale.width, ROW), egui::Sense::hover());
        let s = TimeScale { x0: rect.left(), ..*scale };
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect.shrink2(egui::vec2(0.0, 6.0)), 2.0, style::ACCENT.gamma_multiply(0.12));
        playhead(&painter, &s, time, rect);
    });
    if changed {
        for tr in &mut app.scene.timeline.tracks {
            tr.last = None;
        }
        apply(app);
        app.unsaved = true;
    }
}

// ------------------------------------------------------------------------------------------
// Render dell'animazione
// ------------------------------------------------------------------------------------------

fn start_render(app: &mut ForgeApp) {
    let Some(dir) = rfd::FileDialog::new().set_title(tr("Cartella dei fotogrammi")).pick_folder() else {
        return;
    };
    let tl = &app.scene.timeline;
    let total = ((tl.duration * tl.fps).round() as usize).max(1);
    let run = AnimRun {
        dir,
        frame: 0,
        total,
        rendering: false,
        written: Vec::new(),
        settings: app.settings.clone(),
        auto_restart: app.auto_restart,
        view: app.view,
        time: tl.time,
        started: Instant::now(),
        waiting_denoise: None,
        make_video: true,
        video: None,
        cancelled: false,
    };
    app.settings.noise_threshold = 0.0;
    if app.settings.denoise.mode != forge_render_core::DenoiseMode::Off {
        app.settings.denoise.mode = forge_render_core::DenoiseMode::Final;
    }
    app.auto_restart = false;
    app.timeline_ui.playing = false;
    tracing::info!("render dell'animazione: {total} fotogrammi in {}", run.dir.display());
    app.timeline_ui.run = Some(run);
}

fn tick_render(app: &mut ForgeApp) {
    let Some(run) = &app.timeline_ui.run else { return };
    if run.video.is_some() {
        return finish_video(app);
    }
    if run.cancelled || run.frame >= run.total {
        return finish_render(app);
    }
    if !run.rendering {
        let fps = app.scene.timeline.fps.max(1.0);
        app.scene.timeline.time = run.frame as f32 / fps;
        apply(app);
        app.view = crate::app::View::Render;
        app.start_render();
        if let Some(run) = &mut app.timeline_ui.run {
            run.rendering = true;
            run.waiting_denoise = None;
        }
        return;
    }
    if app.is_preparing() {
        return;
    }
    let Some(job) = app.job_id() else {
        app.start_render();
        return;
    };
    let p = app.session.progress();
    if p.job_id != job {
        return;
    }
    match p.state {
        forge_render_core::SessionState::Completed => {}
        forge_render_core::SessionState::Failed(_) | forge_render_core::SessionState::Cancelled => {
            if let Some(r) = &mut app.timeline_ui.run {
                r.cancelled = true;
            }
            return;
        }
        _ => return,
    }
    let Some(frame) = app.session.latest_frame() else { return };
    if frame.job_id != job || frame.samples < app.settings.target_samples {
        return;
    }
    let want_denoise = app.settings.denoise.mode != forge_render_core::DenoiseMode::Off && app.denoiser_info.is_ok();
    if want_denoise && frame.denoised_samples != frame.samples {
        let run = app.timeline_ui.run.as_mut().expect("render dell'animazione");
        let since = *run.waiting_denoise.get_or_insert_with(Instant::now);
        if since.elapsed() < Duration::from_secs(30) {
            return;
        }
    }
    let (tone, ev, strength) = (app.settings.tone_mapping, app.settings.exposure_ev, app.settings.denoise.strength);
    let run = app.timeline_ui.run.as_mut().expect("render dell'animazione");
    let path = run.dir.join(format!("fotogramma_{:05}.png", run.frame + 1));
    match forge_image_io::save_frame_png(&path, &frame, tone, ev, strength) {
        Ok(()) => run.written.push(path),
        Err(e) => {
            tracing::error!("fotogramma non salvato: {e:#}");
            run.cancelled = true;
            return;
        }
    }
    run.frame += 1;
    run.rendering = false;
}

fn finish_render(app: &mut ForgeApp) {
    let Some(run) = &mut app.timeline_ui.run else { return };
    app.settings = run.settings.clone();
    app.auto_restart = run.auto_restart;
    app.view = run.view;
    app.scene.timeline.time = run.time;
    if !run.cancelled && run.make_video && run.written.len() > 1 {
        let frames = run.written.clone();
        let out = run.dir.join("animazione.mp4");
        let fps = app.scene.timeline.fps.round().max(1.0) as u32;
        run.video = Some(std::thread::spawn(move || crate::video::encode_mp4(&frames, &out, fps, 1)));
        return;
    }
    let run = app.timeline_ui.run.take().expect("render dell'animazione");
    let text = tf(
        "Animazione: {} fotogrammi in {} salvati in\n{}",
        &[
            run.written.len().to_string(),
            format!("{:.0} s", run.started.elapsed().as_secs_f32()),
            run.dir.display().to_string(),
        ],
    );
    tracing::info!("{}", text.replace('\n', " "));
    app.info = Some(text);
    apply(app);
}

fn finish_video(app: &mut ForgeApp) {
    let Some(run) = &mut app.timeline_ui.run else { return };
    if run.video.as_ref().is_some_and(|v| !v.is_finished()) {
        return;
    }
    let video = run.video.take().map(|v| v.join());
    let run = app.timeline_ui.run.take().expect("render dell'animazione");
    let mut text = tf(
        "Animazione: {} fotogrammi in {} salvati in\n{}",
        &[
            run.written.len().to_string(),
            format!("{:.0} s", run.started.elapsed().as_secs_f32()),
            run.dir.display().to_string(),
        ],
    );
    match video {
        Some(Ok(Ok(enc))) => {
            text = format!(
                "{text}\n{}",
                tf(
                    "Video: {} ({})",
                    &[run.dir.join("animazione.mp4").display().to_string(), enc.to_string()]
                )
            )
        }
        Some(Ok(Err(e))) => text = format!("{text}\n{}", tf("video non creato: {e:#}", &[format!("{e:#}")])),
        _ => {}
    }
    tracing::info!("{}", text.replace('\n', " "));
    app.info = Some(text);
    apply(app);
}
