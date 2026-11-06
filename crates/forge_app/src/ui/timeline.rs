//! Pannello della timeline: sempre attiva, in basso nella scheda Timeline accanto a Console, oppure
//! staccata in una finestra a parte.
//!
//! - In alto: riproduzione (inizio, chiave prima, play/pausa, chiave dopo, fine, ripeti), tempo,
//!   durata e fotogrammi al secondo, "+ Elemento", zoom, salvataggio, stacca.
//! - A sinistra gli elementi con le loro righe: posizione, rotazione, bersaglio e colore sono una
//!   riga sola con i tre valori. Rombo = chiave nell'istante del cursore su tutta la riga; cambiando
//!   un valore con le chiavi si registra una chiave; "ƒ+" aggiunge i modificatori e "+" le righe non
//!   di base.
//! - A destra il righello (clic o trascinamento = sposta il cursore) e le chiavi, come in After
//!   Effects: clic seleziona, Ctrl o Maiusc + clic aggiunge o toglie, trascinando sul vuoto si
//!   disegna un rettangolo di selezione, trascinando una chiave si spostano tutte le selezionate,
//!   clic destro = interpolazione o elimina, Canc elimina, Ctrl+C e Ctrl+V copiano nell'istante del
//!   cursore, F9 = morbida in entrata e uscita, Esc toglie la selezione. Gli effetti calcolati nel
//!   tempo hanno una barra che parte dal loro inizio: trascinandola cambia l'inizio.
//!
//! Spostare un oggetto con il gizmo (o cambiare un valore nei pannelli) quando la sua riga ha già
//! delle chiavi registra una chiave nell'istante del cursore.

use crate::app::ForgeApp;
use crate::i18n::{tf, tr};
use crate::ui::{icons, style};
use eframe::egui;
use forge_scene::{Interp, Key, ModSlot, Modifier, Prop, Target};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const ROW: f32 = 20.0;
const LEFT: f32 = 400.0;

/// Stato del pannello e della riproduzione (non salvato nel progetto).
pub struct TimelineUi {
    /// Ultimo istante delle lucciole mandato al render.
    fireflies_time: f32,
    pub playing: bool,
    last_tick: Option<Instant>,
    pub loop_play: bool,
    /// Zoom del tempo (1 = tutta la durata nella larghezza) e primo secondo in vista.
    zoom: f32,
    t0: f32,
    /// Chiavi selezionate: (traccia, tempo). Le chiavi di una riga si selezionano insieme.
    selection: Vec<(usize, f32)>,
    /// Chiavi in trascinamento.
    drag: Option<KeyDrag>,
    /// Inizio del rettangolo di selezione.
    marquee: Option<egui::Pos2>,
    /// Chiavi copiate.
    clipboard: Vec<(usize, Key)>,
    /// Resto del trascinamento della barra dell'area di lavoro (sotto un fotogramma).
    work_carry: f32,
    drag_start: Option<Target>,
    /// Effetti nel tempo visti l'ultima volta (quando ne arriva uno il cursore va sul suo istante).
    seen_effects: usize,
    pub run: Option<AnimRun>,
    /// Play nella vista Render: campioni per fotogramma, e il render corto in corso.
    pub play_samples: u32,
    render_play: Option<RenderPlay>,
    /// Che cosa salva "Salva animazione".
    pub save_kind: SaveKind,
    /// Da dove: None = la vista della viewport, Some(i) = la camera di rendering i.
    pub save_view: Option<usize>,
    pub make_video: bool,
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
            selection: Vec::new(),
            drag: None,
            marquee: None,
            clipboard: Vec::new(),
            work_carry: 0.0,
            drag_start: None,
            seen_effects: 0,
            fireflies_time: 0.0,
            run: None,
            play_samples: 20,
            render_play: None,
            save_kind: SaveKind::Render,
            save_view: None,
            make_video: true,
            raise: false,
            demo_pending: false,
        }
    }
}

/// Play nella vista Render: ogni fotogramma si calcola con pochi campioni e si passa al successivo
/// (più lento del tempo reale, ma si vede l'animazione renderizzata).
struct RenderPlay {
    settings: forge_render_core::RenderSettings,
    auto_restart: bool,
    /// Render del fotogramma in corso.
    waiting: bool,
    /// Il fotogramma finito è già passato per lo schermo.
    shown: bool,
}

/// Che cosa salva "Salva animazione".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveKind {
    /// Il render di ogni fotogramma.
    Render,
    /// I fotogrammi della viewport: anteprima veloce del movimento.
    Preview,
}

/// Trascinamento delle chiavi selezionate.
struct KeyDrag {
    /// Tempo sotto il mouse all'inizio.
    grab: f32,
    /// Spostamento già fatto.
    moved: f32,
    /// Le chiavi com'erano (per non perderle se ne incontrano altre).
    before: Vec<(usize, Key)>,
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
    /// Istanti da salvare (area di lavoro).
    times: Vec<f32>,
    kind: SaveKind,
    /// Da dove si salva, e la camera attiva di prima da rimettere.
    view_cam: Option<usize>,
    active_before: Option<usize>,
    make_video: bool,
    video_settings: crate::video::VideoSettings,
    video_out: Option<PathBuf>,
    video: Option<std::thread::JoinHandle<anyhow::Result<crate::video::Report>>>,
    /// Avanzamento della codifica, condiviso con il suo thread, e da quando è iniziata.
    encode: Option<(std::sync::Arc<crate::video::Progress>, Instant)>,
    cancelled: bool,
}

impl AnimRun {
    pub fn progress(&self) -> (usize, usize) {
        (self.frame.min(self.total), self.total)
    }

    pub fn started(&self) -> Instant {
        self.started
    }

    pub fn encoding(&self) -> Option<(std::sync::Arc<crate::video::Progress>, Instant)> {
        self.encode.clone()
    }

    fn stop(&mut self) {
        self.cancelled = true;
        if let Some((p, _)) = &self.encode {
            p.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
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

/// A ogni fotogramma dell'interfaccia: riproduzione, applicazione dei valori animati, render
/// dell'animazione.
pub fn tick(app: &mut ForgeApp, ctx: &egui::Context) {
    let effects = time_effects(app);
    if effects > app.timeline_ui.seen_effects {
        // Effetto nuovo: compare nella timeline e, se non c'è ancora un'animazione, il cursore va
        // sul suo istante, così non riparte vuoto.
        let mut tl = std::mem::take(&mut app.scene.timeline);
        tl.adopt_effects(&app.scene);
        app.scene.timeline = tl;
        app.timeline_settled();
    }
    if effects < app.timeline_ui.seen_effects {
        // Effetto tolto: le sue righe non servono più.
        let mut tl = std::mem::take(&mut app.scene.timeline);
        tl.prune(&app.scene);
        app.scene.timeline = tl;
        app.timeline_ui.selection.clear();
        app.timeline_settled();
    }
    app.timeline_ui.seen_effects = effects;
    if app.timeline_ui.demo_pending && !app.scene.roots.is_empty() {
        app.timeline_ui.demo_pending = false;
        demo(app);
        app.timeline_settled();
    }
    if app.timeline_ui.run.is_some() {
        tick_render(app);
        ctx.request_repaint_after(Duration::from_millis(50));
        return;
    }
    if app.timeline_ui.playing && app.view == crate::app::View::Render {
        render_play_step(app);
        ctx.request_repaint_after(Duration::from_millis(16));
        return;
    }
    restore_render_play(app);
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
        // Si riproduce l'area di lavoro.
        let (a, b) = tl.work_area();
        let mut t = tl.time + dt;
        if t > b {
            if app.timeline_ui.loop_play {
                t = a;
            } else {
                t = b;
                app.timeline_ui.playing = false;
            }
        }
        tl.time = t;
        ctx.request_repaint();
    }
    app.timeline_ui.last_tick = Some(now);
    apply(app);
}

/// Un passo del Play nella vista Render: render corto del fotogramma, poi il successivo.
/// Finito il Play nella vista Render tornano le impostazioni di prima.
pub fn restore_render_play(app: &mut ForgeApp) {
    if let Some(r) = app.timeline_ui.render_play.take() {
        app.settings = r.settings;
        app.auto_restart = r.auto_restart;
    }
}

/// Il Play nella vista Render è in corso: l'immagine si aggiorna solo a fotogramma finito.
pub fn render_play_running(app: &ForgeApp) -> bool {
    app.timeline_ui.render_play.is_some()
}

/// Scena sostituita: stato legato a quella vecchia via, Play nel Render compreso.
/// Si sta trascinando qualcosa nella timeline (anche nella finestra staccata).
pub fn dragging(app: &ForgeApp) -> bool {
    let ui = &app.timeline_ui;
    ui.drag.is_some() || ui.marquee.is_some()
}

/// Annulla o Ripeti hanno rimesso un'animazione: le chiavi selezionate possono non esserci più.
pub fn restored(app: &mut ForgeApp) {
    app.timeline_ui.selection.clear();
    app.timeline_ui.seen_effects = time_effects(app);
    apply(app);
}

pub fn scene_replaced(app: &mut ForgeApp) {
    let effects = time_effects(app);
    let ui = &mut app.timeline_ui;
    ui.playing = false;
    ui.selection.clear();
    ui.clipboard.clear();
    ui.drag = None;
    ui.marquee = None;
    ui.drag_start = None;
    ui.seen_effects = effects;
}

fn render_play_step(app: &mut ForgeApp) {
    if app.timeline_ui.render_play.is_none() {
        app.timeline_ui.render_play = Some(RenderPlay {
            settings: app.settings.clone(),
            auto_restart: app.auto_restart,
            waiting: false,
            shown: false,
        });
        app.auto_restart = false;
        app.settings.noise_threshold = 0.0;
        app.settings.denoise.mode = forge_render_core::DenoiseMode::Off;
    }
    app.settings.target_samples = app.timeline_ui.play_samples.max(1);
    let waiting = app.timeline_ui.render_play.as_ref().is_some_and(|r| r.waiting);
    if waiting {
        if app.is_preparing() {
            return;
        }
        let Some(job) = app.job_id() else { return };
        let p = app.session.progress();
        let done = p.job_id == job && !matches!(p.state, forge_render_core::SessionState::Rendering);
        if !done {
            return;
        }
        // Un giro dell'interfaccia per mostrare il fotogramma finito, prima del prossimo render.
        if let Some(r) = &mut app.timeline_ui.render_play {
            if !r.shown {
                r.shown = true;
                return;
            }
            r.shown = false;
        }
        // Fotogramma successivo nell'area di lavoro.
        let tl = &mut app.scene.timeline;
        let (a, b) = tl.work_area();
        let mut t = tl.time + tl.frame();
        if t > b {
            if app.timeline_ui.loop_play {
                t = a;
            } else {
                app.timeline_ui.playing = false;
                return;
            }
        }
        tl.time = t;
    }
    apply(app);
    app.start_render();
    if let Some(r) = &mut app.timeline_ui.render_play {
        r.waiting = true;
    }
}

/// Tastiera sulle chiavi selezionate (prima delle scorciatoie del programma: Canc qui toglie le
/// chiavi, non l'oggetto selezionato).
pub fn keyboard(app: &mut ForgeApp, ctx: &egui::Context) {
    if ctx.egui_wants_keyboard_input() {
        return;
    }
    let none = egui::Modifiers::NONE;
    let has_sel = !app.timeline_ui.selection.is_empty();
    let mut changed = false;
    if has_sel {
        if ctx.input_mut(|i| i.consume_key(none, egui::Key::Delete) || i.consume_key(none, egui::Key::Backspace)) {
            let sel = std::mem::take(&mut app.timeline_ui.selection);
            let mut tl = std::mem::take(&mut app.scene.timeline);
            tl.delete_keys(&app.scene, &sel);
            app.scene.timeline = tl;
            changed = true;
        }
        if ctx.input_mut(|i| i.consume_key(none, egui::Key::Escape)) {
            app.timeline_ui.selection.clear();
        }
        if ctx.input_mut(|i| i.consume_key(none, egui::Key::F9)) {
            let sel = app.timeline_ui.selection.clone();
            app.scene.timeline.set_keys_interp(&sel, Interp::EaseInOut);
            changed = true;
        }
        // Ctrl+C e Ctrl+V non arrivano come tasti: egui li trasforma negli eventi Copy e Paste.
        if ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Copy))) {
            app.timeline_ui.clipboard = app.scene.timeline.copy_keys(&app.timeline_ui.selection);
            // Paste arriva solo se negli appunti di sistema c'è qualcosa.
            ctx.copy_text(format!("ForgeRender: {} keyframe", app.timeline_ui.clipboard.len()));
        }
    }
    let paste = |e: &egui::Event| matches!(e, egui::Event::Paste(_) | egui::Event::PasteImage(_));
    if !app.timeline_ui.clipboard.is_empty() && ctx.input(|i| i.events.iter().any(paste)) {
        let keys = app.timeline_ui.clipboard.clone();
        let at = app.scene.timeline.time;
        app.timeline_ui.selection = app.scene.timeline.paste_keys(&keys, at);
        changed = true;
    }
    if changed {
        refresh(app);
    }
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
    for (k, axis) in [Prop::PosX, Prop::PosY, Prop::PosZ].into_iter().enumerate() {
        let x = tl.track(&target, axis).expect("traccia");
        tl.tracks[x].modifiers.push(ModSlot {
            modifier: Modifier::Wiggle {
                amplitude: 0.03,
                frequency: 2.0,
                seed: 3 + k as u32,
            },
            enabled: true,
        });
    }
    let o = tl.track(&target, Prop::Opacity).expect("traccia");
    tl.tracks[o].keys = vec![
        key(0.0, 1.0, Interp::Linear),
        key(2.0, 0.35, Interp::Hold),
        key(3.2, 1.0, Interp::Linear),
    ];
    let cz = tl.track(&Target::Camera, Prop::PosZ).expect("traccia");
    let cb = tl.tracks[cz].base;
    tl.tracks[cz].keys = vec![key(0.0, cb, Interp::EaseInOut), key(4.0, cb * 0.8, Interp::EaseInOut)];
    let times: Vec<f32> = tl.tracks[y].keys.iter().map(|k| k.t).collect();
    for ax in [Prop::PosX, Prop::PosZ] {
        let i = tl.track(&target, ax).expect("traccia");
        let v = tl.tracks[i].base;
        tl.tracks[i].keys = times.iter().map(|&t| key(t, v, Interp::EaseInOut)).collect();
    }
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
    effects_follow(app);
    app.unsaved = unsaved;
    if !a.any() {
        return;
    }
    if a.geometry {
        app.geometry_changed();
    } else if a.transforms {
        app.transforms_changed();
    }
    if a.shading || a.effects {
        app.shading_changed();
    }
    if a.camera {
        if app.scene.active_camera.is_none() {
            app.sync_orbit_from_camera();
        }
        app.camera_changed();
    }
    app.unsaved = unsaved;
}

/// Gli effetti che dipendono solo dal tempo seguono il cursore. La polvere è geometria: nella
/// viewport si aggiorna da dieci volte al secondo di timeline a poco più di una, in proporzione ai
/// granelli, per non rifare tutto a ogni fotogramma (il moto è lento); nel render ogni fotogramma è
/// esatto.
fn effects_follow(app: &mut ForgeApp) {
    let t = app.scene.timeline.time;
    // Le lucciole non hanno geometria: nel render cambiano luci e alone, nella viewport i punti.
    if app.scene.fireflies.enabled && t != app.timeline_ui.fireflies_time {
        app.timeline_ui.fireflies_time = t;
        if app.timeline_ui.run.is_some() || app.view == crate::app::View::Render {
            app.shading_changed();
        }
    }
    let d = &app.scene.haze_dust;
    if !d.enabled || (d.motion <= 0.0 && d.air_response <= 0.0) {
        return;
    }
    let exact = app.timeline_ui.run.is_some() || app.view == crate::app::View::Render;
    let grains = d.region.map_or(forge_scene::effects::DUST_MAX as f32, |(_, s)| {
        (s.x * s.y * s.z * d.per_m3).min(forge_scene::effects::DUST_MAX as f32)
    });
    let step = if exact { 0.0 } else { (grains / 20_000.0).clamp(1.0, 7.5) * 0.1 };
    if t == app.scene.dust_time || (t - app.scene.dust_time).abs() < step {
        return;
    }
    app.scene.dust_time = t;
    if app.scene.sync_effects() {
        app.geometry_changed();
    }
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
    let x0 = ruler(app, ui, &scale);
    let scale = TimeScale { x0, ..scale };
    egui::ScrollArea::vertical()
        .id_salt("timeline-righe")
        .scroll_source(egui::scroll_area::ScrollSource {
            drag: egui::scroll_area::DragScroll::Never,
            ..egui::scroll_area::ScrollSource::ALL
        })
        .auto_shrink([false, false])
        .show(ui, |ui| rows(app, ui, &scale));
}

/// Finestra staccata (se la timeline è staccata dal programma).
pub fn window(app: &mut ForgeApp, ctx: &egui::Context) {
    if !app.prefs.timeline_detached {
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
    // Va a capo da sola: in una finestra stretta nessun comando esce dal pannello.
    ui.horizontal_wrapped(|ui| {
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
            let (a, b) = app.scene.timeline.work_area();
            if app.timeline_ui.playing && !(a..b).contains(&app.scene.timeline.time) {
                app.scene.timeline.time = a;
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
        ui.add(
            egui::DragValue::new(&mut app.timeline_ui.play_samples)
                .range(1..=512)
                .suffix(tr(" campioni")),
        )
        .on_hover_text(tr(
            "Play nella vista Render: ogni fotogramma si calcola con questi campioni e si passa al successivo. Va più lento del tempo reale, ma si vede l'animazione renderizzata",
        ));
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
                match r.encoding().map(|(p, _)| p.get()) {
                    Some((f, n)) => ui.label(tf("Codifica del video: fotogramma {} di {}", &[f.to_string(), n.to_string()])),
                    None => ui.label(tf(
                        "Render animazione: fotogramma {} di {}",
                        &[(f + 1).min(n).to_string(), n.to_string()],
                    )),
                };
                if ui.button(tr("Interrompi")).clicked() {
                    if let Some(r) = &mut app.timeline_ui.run {
                        r.stop();
                    }
                }
            }
            None => save_menu(app, ui),
        }
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
}

/// "Salva animazione": che cosa (render di ogni fotogramma o anteprima della viewport) e da dove
/// (la vista o una camera). Salva l'area di lavoro nella cartella Rendering.
fn save_menu(app: &mut ForgeApp, ui: &mut egui::Ui) {
    // Il menu resta aperto mentre si scelgono le opzioni: si chiude con Salva o cliccando fuori.
    let config = egui::containers::menu::MenuConfig::new().close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
    let button = egui::containers::menu::MenuButton::new(format!("{}  {}", icons::IMAGE, tr("Salva animazione"))).config(config);
    button.ui(ui, |ui| {
        let frames = app.scene.timeline.work_frames().len();
        let (a, b) = app.scene.timeline.work_area();
        ui.label(
            egui::RichText::new(tf(
                "Area di lavoro: da {} a {} s, {} fotogrammi",
                &[format!("{a:.2}"), format!("{b:.2}"), frames.to_string()],
            ))
            .weak(),
        );
        ui.separator();
        ui.radio_value(&mut app.timeline_ui.save_kind, SaveKind::Render, tr("Render di ogni fotogramma"))
            .on_hover_text(tr("Il render fotorealistico di ogni fotogramma, con i campioni attuali"));
        ui.radio_value(&mut app.timeline_ui.save_kind, SaveKind::Preview, tr("Anteprima (viewport)"))
            .on_hover_text(tr(
                "I fotogrammi come si vedono nella viewport: pochi secondi, per controllare il movimento. Fumo e nebbie non ci sono",
            ));
        ui.separator();
        ui.label(tr("Vista"));
        let cams: Vec<String> = app.scene.render_cameras.iter().map(|c| c.name.clone()).collect();
        if app.timeline_ui.save_view.is_some_and(|i| i >= cams.len()) {
            app.timeline_ui.save_view = None;
        }
        ui.radio_value(&mut app.timeline_ui.save_view, None, tr("Viewport"));
        for (i, name) in cams.iter().enumerate() {
            ui.radio_value(&mut app.timeline_ui.save_view, Some(i), name);
        }
        ui.separator();
        ui.checkbox(&mut app.timeline_ui.make_video, tr("Anche il video"));
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(crate::ui::video_settings::summary(&app.prefs.video)).weak());
            if ui.small_button(icons::SETTINGS).on_hover_text(tr("Impostazioni video…")).clicked() {
                app.video_open = true;
            }
        });
        if ui
            .add(egui::Button::new(egui::RichText::new(tr("Salva")).strong().color(egui::Color32::WHITE)).fill(style::ACCENT))
            .on_hover_text(tr("Salva nella cartella Rendering, in una cartella nuova per ogni animazione"))
            .clicked()
        {
            start_render(app);
            ui.close();
        }
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
                    add.push(Target::Light(l.id));
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
                        add.push(Target::Fog(f.id));
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

/// Righello del tempo: clic o trascinamento = sposta il cursore. Restituisce il bordo sinistro
/// della colonna delle chiavi.
fn ruler(app: &mut ForgeApp, ui: &mut egui::Ui, scale: &TimeScale) -> f32 {
    work_bar(app, ui, scale);
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
                    // Con le tacche da 0.25 s una cifra sola darebbe 0.2, 0.5, 0.8.
                    if step < 0.5 { format!("{k:.2}s") } else { format!("{k:.1}s") },
                    egui::FontId::proportional(10.0),
                    ui.visuals().weak_text_color(),
                );
            }
            k += step;
        }
        // Fuori dall'area di lavoro.
        let (wa, wb) = tl.work_area();
        let shade = egui::Color32::from_black_alpha(90);
        let xa = s.x(wa).clamp(rect.left(), rect.right());
        let xb = s.x(wb).clamp(rect.left(), rect.right());
        painter.rect_filled(egui::Rect::from_min_max(rect.min, egui::pos2(xa, rect.bottom())), 0.0, shade);
        painter.rect_filled(egui::Rect::from_min_max(egui::pos2(xb, rect.top()), rect.max), 0.0, shade);
        playhead(&painter, &s, tl.time, rect);
        rect.left()
    })
    .inner
}

/// Barra dell'area di lavoro sopra il righello, come in After Effects: si trascinano le due
/// estremità, o il centro per spostarla tutta. È la parte che si riproduce e si salva.
fn work_bar(app: &mut ForgeApp, ui: &mut egui::Ui, scale: &TimeScale) {
    ui.horizontal(|ui| {
        ui.allocate_exact_size(egui::vec2(LEFT, 10.0), egui::Sense::hover());
        let (rect, _) = ui.allocate_exact_size(egui::vec2(scale.width, 10.0), egui::Sense::hover());
        let s = TimeScale { x0: rect.left(), ..*scale };
        let tl = &app.scene.timeline;
        let (a, b) = tl.work_area();
        let (xa, xb) = (s.x(a), s.x(b));
        let bar = egui::Rect::from_min_max(egui::pos2(xa, rect.top() + 1.0), egui::pos2(xb, rect.bottom() - 1.0));
        let grip = |x: f32| egui::Rect::from_center_size(egui::pos2(x, rect.center().y), egui::vec2(10.0, 10.0));
        let middle = ui.interact(
            bar.shrink2(egui::vec2(6.0, 0.0)),
            ui.make_persistent_id("area-lavoro"),
            egui::Sense::drag(),
        );
        let left = ui.interact(grip(xa), ui.make_persistent_id("area-lavoro-inizio"), egui::Sense::drag());
        let right = ui.interact(grip(xb), ui.make_persistent_id("area-lavoro-fine"), egui::Sense::drag());
        let cursor = egui::CursorIcon::ResizeHorizontal;
        let left = left
            .on_hover_cursor(cursor)
            .on_hover_text(tr("Inizio dell'area da riprodurre e salvare"));
        let right = right
            .on_hover_cursor(cursor)
            .on_hover_text(tr("Fine dell'area da riprodurre e salvare"));
        let middle = middle.on_hover_text(tr("Trascina per spostare l'area da riprodurre e salvare"));
        let tl = &mut app.scene.timeline;
        let frame = tl.frame();
        let mut changed = false;
        if left.dragged() {
            if let Some(p) = left.interact_pointer_pos() {
                tl.work_in = tl.snap(s.t(p.x)).clamp(0.0, b - frame);
                tl.work_out = b;
                changed = true;
            }
        }
        if right.dragged() {
            if let Some(p) = right.interact_pointer_pos() {
                tl.work_out = tl.snap(s.t(p.x)).clamp(a + frame, tl.duration);
                tl.work_in = a;
                changed = true;
            }
        }
        if middle.dragged() {
            let dt = ((middle.drag_delta().x / s.px_per_s + app.timeline_ui.work_carry) / frame).trunc() * frame;
            app.timeline_ui.work_carry += middle.drag_delta().x / s.px_per_s - dt;
            let dt = dt.clamp(-a, tl.duration - b);
            tl.work_in = a + dt;
            tl.work_out = b + dt;
            changed = true;
        }
        if changed {
            app.unsaved = true;
        }
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);
        painter.rect_filled(bar, 2.0, style::ACCENT.gamma_multiply(0.55));
        for g in [xa, xb] {
            painter.rect_filled(
                egui::Rect::from_center_size(egui::pos2(g, rect.center().y), egui::vec2(4.0, 10.0)),
                1.0,
                style::ACCENT,
            );
        }
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
    AddProps(Target, Vec<Prop>),
    RemoveTracks(Vec<usize>),
    /// Mette o toglie la chiave nell'istante del cursore su tutte le tracce della riga.
    ToggleKey(Vec<usize>),
    /// Valore di una traccia: (tracce della riga, traccia, valore).
    SetValue(Vec<usize>, usize, f32),
    AddModifier(Vec<usize>, Modifier),
    RemoveModifier(Vec<usize>, usize),
    DeleteSelected,
    SetInterp(Interp),
    SetStart(Target, f32),
    SetStop(Target, Option<f32>),
}

/// Posizione di una chiave sullo schermo, per il rettangolo di selezione.
struct KeyHit {
    pos: egui::Pos2,
    t: f32,
    tracks: Vec<usize>,
}

fn rows(app: &mut ForgeApp, ui: &mut egui::Ui, scale: &TimeScale) {
    let mut actions: Vec<Action> = Vec::new();
    let mut hits: Vec<KeyHit> = Vec::new();
    let elements = app.scene.timeline.elements.clone();
    for target in &elements {
        element_rows(app, ui, scale, target, &mut actions, &mut hits);
    }
    selection_gestures(app, ui, scale, &hits);
    if actions.is_empty() {
        return;
    }
    let time = app.scene.timeline.time;
    let mut tl = std::mem::take(&mut app.scene.timeline);
    let tol = tl.frame() * 0.5;
    let mut push_scene = Vec::new();
    for a in actions {
        match a {
            Action::RemoveElement(t) => {
                tl.remove_element(&t);
                app.timeline_ui.selection.clear();
            }
            Action::AddProps(t, ps) => {
                for p in ps {
                    tl.add_track(&app.scene, t.clone(), p);
                }
            }
            Action::RemoveTracks(mut ts) => {
                ts.sort_unstable();
                for i in ts.into_iter().rev() {
                    tl.tracks.remove(i);
                }
                app.timeline_ui.selection.clear();
            }
            Action::ToggleKey(ts) => {
                let on_key = ts.iter().any(|&i| tl.tracks[i].key_near(time, tol).is_some());
                for i in ts {
                    let tr = &mut tl.tracks[i];
                    let v = forge_scene::timeline::get(&app.scene, &tr.target, tr.prop).unwrap_or(tr.base);
                    if on_key {
                        if let Some(k) = tr.key_near(time, tol) {
                            tr.keys.remove(k);
                        }
                        if tr.keys.is_empty() {
                            tr.base = v;
                        }
                    } else {
                        tr.set_key(time, v, tol);
                    }
                }
            }
            Action::SetValue(ts, i, v) => {
                // Con le chiavi la riga prende una chiave su tutti i suoi assi.
                let keyed = ts.iter().any(|&k| !tl.tracks[k].keys.is_empty());
                for k in ts {
                    let tr = &mut tl.tracks[k];
                    let cur = if k == i {
                        v
                    } else {
                        forge_scene::timeline::get(&app.scene, &tr.target, tr.prop).unwrap_or(tr.base)
                    };
                    if keyed {
                        tr.set_key(time, cur, tol);
                    } else {
                        tr.base = cur;
                    }
                    if k == i {
                        push_scene.push((tr.target.clone(), tr.prop, v));
                    }
                }
            }
            Action::AddModifier(ts, m) => {
                // Di serie una rampa sulla rotazione gira attorno alla verticale; gli assi si
                // accendono e spengono dalla sua riga.
                let only_y = matches!(m, Modifier::Ramp { .. }) && ts.len() == 3 && tl.tracks[ts[0]].prop == Prop::RotX;
                for (n, i) in ts.into_iter().enumerate() {
                    let mut m = m.clone();
                    if let Modifier::Wiggle { seed, .. } = &mut m {
                        *seed += n as u32;
                    }
                    tl.tracks[i].modifiers.push(ModSlot {
                        modifier: m,
                        enabled: !only_y || n == 1,
                    });
                }
            }
            Action::RemoveModifier(ts, k) => {
                for i in ts {
                    if k < tl.tracks[i].modifiers.len() {
                        tl.tracks[i].modifiers.remove(k);
                    }
                }
            }
            Action::DeleteSelected => {
                let sel = std::mem::take(&mut app.timeline_ui.selection);
                tl.delete_keys(&app.scene, &sel);
            }
            Action::SetInterp(interp) => tl.set_keys_interp(&app.timeline_ui.selection, interp),
            Action::SetStart(t, s) => tl.set_start(&t, tl.snap(s.max(0.0))),
            Action::SetStop(t, s) => {
                let s = s.map(|v| tl.snap(v));
                tl.set_stop(&t, s);
            }
        }
    }
    for (t, p, v) in push_scene {
        forge_scene::timeline::set(&mut app.scene, &t, p, v);
    }
    app.scene.timeline = tl;
    refresh(app);
}

/// Dopo una modifica delle chiavi: rimette i valori dell'istante attuale.
fn refresh(app: &mut ForgeApp) {
    for tr in &mut app.scene.timeline.tracks {
        tr.last = None;
    }
    apply(app);
    app.unsaved = true;
}

/// Trascinamento delle chiavi selezionate e rettangolo di selezione: continuano finché il tasto è
/// premuto, anche fuori dalla riga da cui sono partiti.
fn selection_gestures(app: &mut ForgeApp, ui: &mut egui::Ui, scale: &TimeScale, hits: &[KeyHit]) {
    let (pos, down, mods) = ui.input(|i| (i.pointer.latest_pos(), i.pointer.primary_down(), i.modifiers));
    if let Some(mut d) = app.timeline_ui.drag.take() {
        if let Some(p) = pos {
            let tl = &mut app.scene.timeline;
            let frame = tl.frame();
            let want = ((scale.t(p.x) - d.grab) / frame).round() * frame;
            let step = want - d.moved;
            if step.abs() > 1e-5 {
                let mut sel = std::mem::take(&mut app.timeline_ui.selection);
                d.moved += tl.shift_keys(&mut sel, step);
                app.timeline_ui.selection = sel;
            }
        }
        if down {
            app.timeline_ui.drag = Some(d);
        } else {
            app.scene.timeline.settle_keys(&d.before);
            refresh(app);
        }
        return;
    }
    let Some(start) = app.timeline_ui.marquee else { return };
    let Some(p) = pos else { return };
    let rect = egui::Rect::from_two_pos(start, p);
    let painter = ui
        .ctx()
        .layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::NULL.with("riquadro-chiavi")));
    painter.rect_filled(rect, 0.0, style::ACCENT.gamma_multiply(0.12));
    painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, style::ACCENT), egui::StrokeKind::Inside);
    if down {
        return;
    }
    app.timeline_ui.marquee = None;
    if !(mods.ctrl || mods.shift || mods.command) {
        app.timeline_ui.selection.clear();
    }
    for h in hits.iter().filter(|h| rect.contains(h.pos)) {
        for &tr in &h.tracks {
            if !is_selected(&app.timeline_ui.selection, tr, h.t) {
                app.timeline_ui.selection.push((tr, h.t));
            }
        }
    }
}

fn is_selected(sel: &[(usize, f32)], track: usize, t: f32) -> bool {
    sel.iter().any(|&(i, k)| i == track && (k - t).abs() < 1e-4)
}

/// Colonna del tempo di una riga: il vuoto risponde al trascinamento (rettangolo di selezione) e al
/// clic (toglie la selezione).
fn lane(app: &mut ForgeApp, ui: &mut egui::Ui, scale: &TimeScale, sense_effect: bool) -> (egui::Rect, egui::Response) {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(scale.width, ROW), egui::Sense::click_and_drag());
    if !sense_effect {
        if resp.drag_started() {
            app.timeline_ui.marquee = resp.interact_pointer_pos();
        }
        if resp.clicked() && !ui.input(|i| i.modifiers.ctrl || i.modifiers.shift) {
            app.timeline_ui.selection.clear();
        }
    }
    (rect, resp)
}

fn element_rows(
    app: &mut ForgeApp,
    ui: &mut egui::Ui,
    scale: &TimeScale,
    target: &Target,
    actions: &mut Vec<Action>,
    hits: &mut Vec<KeyHit>,
) {
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
            for (label, props) in target.rows() {
                let has = props.iter().all(|&p| app.scene.timeline.track(target, p).is_some());
                if ui.add_enabled(!has, egui::Button::new(tr(label))).clicked() {
                    actions.push(Action::AddProps(target.clone(), props));
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
            // Fine dell'emissione: 0 = emette sempre.
            let mut stop = app.scene.timeline.stop_of(target).unwrap_or(0.0);
            if lui
                .add(
                    egui::DragValue::new(&mut stop)
                        .range(0.0..=3600.0)
                        .speed(0.02)
                        .prefix(tr("stop "))
                        .suffix(" s"),
                )
                .on_hover_text(tr(
                    "Quando la sorgente smette di emettere (0 = mai): quello che è uscito non sparisce, continua a muoversi e a dissolversi da solo",
                ))
                .changed()
            {
                actions.push(Action::SetStop(target.clone(), (stop > 0.0).then_some(stop)));
            }
        }
        // Colonna del tempo: barra dell'effetto (dal suo inizio) o fascia dell'elemento.
        let (rect, resp) = lane(app, ui, scale, is_effect);
        let s = TimeScale { x0: rect.left(), ..*scale };
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, ui.visuals().faint_bg_color);
        if is_effect {
            let x0 = s.x(app.scene.timeline.start_of(target)).max(rect.left());
            let x1 = s.x(app.scene.timeline.duration).min(rect.right());
            if x1 > x0 {
                let bar = egui::Rect::from_min_max(egui::pos2(x0, rect.top() + 3.0), egui::pos2(x1, rect.bottom() - 3.0));
                painter.rect_filled(bar, 3.0, style::ACCENT.gamma_multiply(0.2));
                // Parte piena: la sorgente emette. Dopo lo stop il fumo vive da solo.
                let stop_x = app.scene.timeline.stop_of(target).map(|v| s.x(v).clamp(x0, x1)).unwrap_or(x1);
                let emitting = egui::Rect::from_min_max(bar.min, egui::pos2(stop_x, bar.max.y));
                painter.rect_filled(emitting, 3.0, style::ACCENT.gamma_multiply(0.55));
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
    // Righe: gli assi di un gruppo insieme, nell'ordine dell'elenco compatibile.
    for (label, props) in target.rows() {
        let tracks: Vec<usize> = props.iter().filter_map(|&p| app.scene.timeline.track(target, p)).collect();
        if tracks.is_empty() {
            continue;
        }
        prop_row(app, ui, scale, label, &tracks, actions, hits);
        let mods = tracks
            .iter()
            .map(|&i| app.scene.timeline.tracks[i].modifiers.len())
            .max()
            .unwrap_or(0);
        for k in 0..mods {
            modifier_row(app, ui, scale, &tracks, k, actions);
        }
    }
}

/// Chiavi di una riga: gli istanti in cui almeno una traccia ha una chiave.
fn row_keys(app: &ForgeApp, tracks: &[usize]) -> Vec<(f32, Interp, f32)> {
    let mut keys: Vec<(f32, Interp, f32)> = tracks
        .iter()
        .flat_map(|&i| app.scene.timeline.tracks[i].keys.iter().map(|k| (k.t, k.interp, k.v)))
        .collect();
    keys.sort_by(|a, b| a.0.total_cmp(&b.0));
    keys.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-4);
    keys
}

fn prop_row(
    app: &mut ForgeApp,
    ui: &mut egui::Ui,
    scale: &TimeScale,
    label: &'static str,
    tracks: &[usize],
    actions: &mut Vec<Action>,
    hits: &mut Vec<KeyHit>,
) {
    let time = app.scene.timeline.time;
    let tol = app.scene.timeline.frame() * 0.5;
    let first = app.scene.timeline.tracks[tracks[0]].clone();
    let base = first.target.is_base(first.prop);
    let keys = row_keys(app, tracks);
    ui.horizontal(|ui| {
        let (left, _) = ui.allocate_exact_size(egui::vec2(LEFT, ROW), egui::Sense::hover());
        let mut lui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(left)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        lui.add_space(18.0);
        let on_key = keys.iter().any(|k| (k.0 - time).abs() <= tol);
        let color = if keys.is_empty() {
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
            actions.push(Action::ToggleKey(tracks.to_vec()));
        }
        lui.add_sized(egui::vec2(92.0, ROW), egui::Label::new(tr(label)).truncate());
        for &i in tracks {
            let tr_ = &app.scene.timeline.tracks[i];
            let mut v = forge_scene::timeline::get(&app.scene, &tr_.target, tr_.prop).unwrap_or(tr_.base);
            let r = lui.add(
                egui::DragValue::new(&mut v)
                    .speed(tr_.prop.speed())
                    .max_decimals(3)
                    .suffix(tr_.prop.unit()),
            );
            if r.changed() {
                actions.push(Action::SetValue(tracks.to_vec(), i, v));
            }
        }
        lui.menu_button("ƒ+", |ui| {
            ui.label(egui::RichText::new(tr("Aggiungi un modificatore")).weak());
            for m in Modifier::all_defaults() {
                if ui.button(tr(m.label())).on_hover_text(tr(m.hint())).clicked() {
                    actions.push(Action::AddModifier(tracks.to_vec(), m));
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
            actions.push(Action::RemoveTracks(tracks.to_vec()));
        }
        // Chiavi.
        let (rect, _) = lane(app, ui, scale, false);
        let s = TimeScale { x0: rect.left(), ..*scale };
        let painter = ui.painter_at(rect);
        painter.line_segment(
            [egui::pos2(rect.left(), rect.center().y), egui::pos2(rect.right(), rect.center().y)],
            egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
        );
        // Tratti animati tra le chiavi.
        for w in keys.windows(2) {
            let (a, b) = (s.x(w[0].0), s.x(w[1].0));
            let col = if w[0].1 == Interp::Hold {
                ui.visuals().weak_text_color()
            } else {
                style::ACCENT.gamma_multiply(0.5)
            };
            painter.line_segment(
                [egui::pos2(a, rect.center().y), egui::pos2(b, rect.center().y)],
                egui::Stroke::new(2.0, col),
            );
        }
        for (n, &(t, interp, v)) in keys.iter().enumerate() {
            let x = s.x(t);
            if x < rect.left() - 6.0 || x > rect.right() + 6.0 {
                continue;
            }
            let c = egui::pos2(x, rect.center().y);
            let members: Vec<usize> = tracks
                .iter()
                .copied()
                .filter(|&i| app.scene.timeline.tracks[i].key_near(t, 1e-4).is_some())
                .collect();
            hits.push(KeyHit {
                pos: c,
                t,
                tracks: members.clone(),
            });
            let selected = members.iter().any(|&i| is_selected(&app.timeline_ui.selection, i, t));
            let kr = egui::Rect::from_center_size(c, egui::vec2(12.0, 14.0));
            let resp = ui.interact(kr, ui.make_persistent_id(("chiave", tracks[0], n)), egui::Sense::click_and_drag());
            let (fill, stroke) = if selected {
                (egui::Color32::WHITE, egui::Stroke::new(1.5, style::ACCENT))
            } else if resp.hovered() {
                (style::ACCENT.gamma_multiply(1.3), egui::Stroke::new(1.0, egui::Color32::WHITE))
            } else {
                (style::ACCENT, egui::Stroke::new(1.0, egui::Color32::BLACK))
            };
            painter.add(egui::Shape::convex_polygon(
                vec![
                    c + egui::vec2(0.0, -6.0),
                    c + egui::vec2(5.0, 0.0),
                    c + egui::vec2(0.0, 6.0),
                    c + egui::vec2(-5.0, 0.0),
                ],
                fill,
                stroke,
            ));
            let add = ui.input(|i| i.modifiers.ctrl || i.modifiers.shift || i.modifiers.command);
            let sel = &mut app.timeline_ui.selection;
            let choose = |sel: &mut Vec<(usize, f32)>| {
                let new: Vec<(usize, f32)> = members.iter().filter(|&&i| !is_selected(sel, i, t)).map(|&i| (i, t)).collect();
                sel.extend(new);
            };
            if resp.clicked() {
                if add && selected {
                    sel.retain(|&(i, k)| !(members.contains(&i) && (k - t).abs() < 1e-4));
                } else {
                    if !add {
                        sel.clear();
                    }
                    choose(sel);
                }
            }
            if resp.secondary_clicked() && !selected {
                sel.clear();
                choose(sel);
            }
            if resp.drag_started() {
                if !selected {
                    if !add {
                        sel.clear();
                    }
                    choose(sel);
                }
                if let Some(p) = resp.interact_pointer_pos() {
                    app.timeline_ui.drag = Some(KeyDrag {
                        grab: s.t(p.x),
                        moved: 0.0,
                        before: app.scene.timeline.copy_keys(&app.timeline_ui.selection),
                    });
                }
            }
            let resp = resp.on_hover_text(tf(
                "{} s · {} · {}",
                &[format!("{t:.2}"), format!("{v:.3}"), tr(interp.label()).to_string()],
            ));
            resp.context_menu(|ui| {
                ui.label(egui::RichText::new(tr("Interpolazione verso la chiave dopo")).weak());
                for it in Interp::ALL {
                    if ui.add(egui::Button::selectable(interp == it, tr(it.label()))).clicked() {
                        actions.push(Action::SetInterp(it));
                        ui.close();
                    }
                }
                ui.separator();
                if ui.button(format!("{}  {}", icons::DELETE, tr("Elimina la chiave"))).clicked() {
                    actions.push(Action::DeleteSelected);
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

/// Riga di un modificatore: vale per tutti gli assi della riga, con le lettere X Y Z per scegliere
/// su quali agisce (ogni asse ha il suo interruttore).
fn modifier_row(app: &mut ForgeApp, ui: &mut egui::Ui, scale: &TimeScale, tracks: &[usize], k: usize, actions: &mut Vec<Action>) {
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
        // Interruttori degli assi (uno solo se la riga non ha assi).
        let owners: Vec<usize> = tracks
            .iter()
            .copied()
            .filter(|&i| k < app.scene.timeline.tracks[i].modifiers.len())
            .collect();
        for (n, &i) in owners.iter().enumerate() {
            let slot = &mut app.scene.timeline.tracks[i].modifiers[k];
            if owners.len() == 3 {
                if lui
                    .selectable_label(slot.enabled, ["X", "Y", "Z"][n])
                    .on_hover_text(tr("Su quali assi agisce il modificatore"))
                    .clicked()
                {
                    slot.enabled = !slot.enabled;
                    changed = true;
                }
            } else {
                changed |= lui.checkbox(&mut slot.enabled, "").on_hover_text(tr("Acceso / spento")).changed();
            }
        }
        let tl = &app.scene.timeline;
        let lead = owners
            .iter()
            .copied()
            .find(|&i| tl.tracks[i].modifiers[k].enabled)
            .or(owners.first().copied());
        if let Some(lead) = lead {
            let mut m = tl.tracks[lead].modifiers[k].modifier.clone();
            lui.label(egui::RichText::new(format!("ƒ {}", tr(m.label()))))
                .on_hover_text(tr(m.hint()));
            let dv = |ui: &mut egui::Ui, v: &mut f32, speed: f64, hint: &str| {
                ui.add(egui::DragValue::new(v).speed(speed).max_decimals(3))
                    .on_hover_text(hint)
                    .changed()
            };
            let mut edited = false;
            match &mut m {
                Modifier::Wiggle {
                    amplitude,
                    frequency,
                    seed,
                } => {
                    edited |= dv(&mut lui, amplitude, 0.01, tr("Ampiezza"));
                    edited |= dv(&mut lui, frequency, 0.05, tr("Volte al secondo"));
                    edited |= lui
                        .add(egui::DragValue::new(seed).range(0..=9999))
                        .on_hover_text(tr("Seme"))
                        .changed();
                }
                Modifier::Oscillate {
                    amplitude,
                    frequency,
                    phase_deg,
                } => {
                    edited |= dv(&mut lui, amplitude, 0.01, tr("Ampiezza"));
                    edited |= dv(&mut lui, frequency, 0.02, tr("Volte al secondo"));
                    edited |= dv(&mut lui, phase_deg, 1.0, tr("Fase (gradi)"));
                }
                Modifier::Ramp { per_second } => edited |= dv(&mut lui, per_second, 0.5, tr("Unità al secondo")),
                Modifier::Offset { value } => edited |= dv(&mut lui, value, 0.01, tr("Valore aggiunto")),
                Modifier::Multiply { factor } => edited |= dv(&mut lui, factor, 0.01, tr("Fattore")),
                Modifier::Clamp { min, max } => {
                    edited |= dv(&mut lui, min, 0.01, tr("Minimo"));
                    edited |= dv(&mut lui, max, 0.01, tr("Massimo"));
                }
                Modifier::Steps { per_second } => edited |= dv(&mut lui, per_second, 0.2, tr("Scatti al secondo")),
                Modifier::Loop | Modifier::PingPong => {}
            }
            if edited {
                // Stessi valori su tutti gli assi; il tremolio tiene un seme diverso per asse.
                for (n, &i) in owners.iter().enumerate() {
                    let mut m = m.clone();
                    if let Modifier::Wiggle { seed, .. } = &mut m {
                        *seed += n as u32;
                    }
                    app.scene.timeline.tracks[i].modifiers[k].modifier = m;
                }
                changed = true;
            }
        }
        if lui.small_button(icons::CLOSE).on_hover_text(tr("Togli il modificatore")).clicked() {
            actions.push(Action::RemoveModifier(tracks.to_vec(), k));
        }
        let (rect, _) = lane(app, ui, scale, false);
        let s = TimeScale { x0: rect.left(), ..*scale };
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect.shrink2(egui::vec2(0.0, 6.0)), 2.0, style::ACCENT.gamma_multiply(0.12));
        playhead(&painter, &s, time, rect);
    });
    if changed {
        refresh(app);
    }
}

// ------------------------------------------------------------------------------------------
// Render dell'animazione
// ------------------------------------------------------------------------------------------

fn start_render(app: &mut ForgeApp) {
    let kind = app.timeline_ui.save_kind;
    let dir = crate::prefs::new_rendering_subdir(if kind == SaveKind::Preview { "anteprima" } else { "animazione" });
    let tl = &app.scene.timeline;
    let times = tl.work_frames();
    let total = times.len();
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
        times,
        kind,
        view_cam: app.timeline_ui.save_view,
        active_before: app.scene.active_camera,
        make_video: app.timeline_ui.make_video,
        video_settings: app.prefs.video.clone(),
        video_out: app.video_out.clone(),
        video: None,
        encode: None,
        cancelled: false,
    };
    // Il render guarda dalla vista scelta (l'anteprima sceglie la camera a ogni fotogramma).
    if kind == SaveKind::Render {
        app.scene.active_camera = app.timeline_ui.save_view;
        app.settings.noise_threshold = 0.0;
        if app.settings.denoise.mode != forge_render_core::DenoiseMode::Off {
            app.settings.denoise.mode = forge_render_core::DenoiseMode::Final;
        }
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
    if run.kind == SaveKind::Preview {
        return tick_preview(app);
    }
    if !run.rendering {
        app.scene.timeline.time = run.times[run.frame];
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

/// Un fotogramma dell'anteprima: la viewport catturata alla risoluzione del render.
fn tick_preview(app: &mut ForgeApp) {
    let Some(run) = &app.timeline_ui.run else { return };
    let (t, view, frame_no, dir) = (run.times[run.frame], run.view_cam, run.frame, run.dir.clone());
    app.scene.timeline.time = t;
    apply(app);
    let camera = match view.and_then(|i| app.scene.render_cameras.get(i)) {
        Some(c) => c.camera.clone(),
        None => app.scene.camera.clone(),
    };
    let (w, h) = (app.settings.width.max(2), app.settings.height.max(2));
    let path = dir.join(format!("fotogramma_{:05}.png", frame_no + 1));
    let saved = app
        .capture_viewport(&camera, w, h)
        .and_then(|rgba| image::RgbaImage::from_raw(w, h, rgba))
        .map(|img| img.save(&path));
    let run = app.timeline_ui.run.as_mut().expect("anteprima");
    match saved {
        Some(Ok(())) => {
            run.written.push(path);
            run.frame += 1;
        }
        Some(Err(e)) => {
            tracing::error!("fotogramma non salvato: {e:#}");
            run.cancelled = true;
        }
        None => {
            tracing::error!("anteprima non disponibile: la viewport non ha la GPU");
            run.cancelled = true;
        }
    }
}

fn finish_render(app: &mut ForgeApp) {
    let Some(run) = &mut app.timeline_ui.run else { return };
    app.scene.active_camera = run.active_before;
    app.settings = run.settings.clone();
    app.auto_restart = run.auto_restart;
    app.view = run.view;
    app.scene.timeline.time = run.time;
    if !run.cancelled && run.make_video && run.written.len() > 1 {
        let job = crate::video::Job {
            frames: run.written.clone(),
            out: run.video_out.clone().unwrap_or_else(|| run.dir.join("animazione.mp4")),
            settings: run.video_settings.clone(),
            fps: app.scene.timeline.fps.round().max(1.0) as u32,
            loops: 1,
        };
        let progress = std::sync::Arc::new(crate::video::Progress::default());
        run.encode = Some((progress.clone(), Instant::now()));
        run.video = Some(std::thread::spawn(move || crate::video::encode(&job, &progress)));
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
    app.notify_saved(text, Some(run.dir.clone()));
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
        Some(Ok(Ok(report))) => {
            text = format!(
                "{text}\n{}",
                tf("Video: {} ({})", &[report.path.display().to_string(), report.describe()])
            )
        }
        Some(Ok(Err(e))) => text = format!("{text}\n{}", tf("video non creato: {e:#}", &[format!("{e:#}")])),
        _ => {}
    }
    tracing::info!("{}", text.replace('\n', " "));
    app.notify_saved(text, Some(run.dir.clone()));
    apply(app);
}
