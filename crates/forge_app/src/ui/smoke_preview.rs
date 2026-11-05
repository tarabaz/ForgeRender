//! Anteprima del fumo simulato nella viewport: una simulazione grossolana e veloce
//! ([`forge_render_core::smoke::smoke_preview`]) calcolata in un thread a parte ogni volta che
//! cambiano le impostazioni del fumo selezionato, mostrata come una nuvola di punti. La simulazione
//! copre tutta la durata della timeline e si mostra l'istante del cursore: spostandolo o
//! riproducendo non si ricalcola niente. Serve a trovare posizione, sorgente e comportamento di
//! massima senza fare il render; poi si rifinisce con il render.

use crate::app::ForgeApp;
use eframe::egui;
use forge_render_core::smoke::SmokePreview;
use forge_scene::{SmokeCloud, SmokeMode};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::Instant;

/// Stato dell'anteprima (non salvato con il progetto).
pub struct SmokePreviewState {
    /// Anteprima accesa (di serie sì).
    pub enabled: bool,
    /// Il fumo si muove in loop; spento = fermo all'istante di `scrub`.
    pub playing: bool,
    /// Istante mostrato quando è fermo (frazione del tempo simulato, 0–1).
    pub scrub: f32,
    /// Chiave delle impostazioni dell'anteprima pronta o in calcolo.
    key: String,
    /// Indice del fumo e anteprima pronta.
    pub data: Option<(usize, Arc<SmokePreview>)>,
    rx: Option<Receiver<Option<SmokePreview>>>,
    cancel: Arc<AtomicBool>,
    /// In calcolo.
    pub busy: bool,
    clock: Instant,
}

impl Default for SmokePreviewState {
    fn default() -> Self {
        Self {
            enabled: true,
            playing: true,
            scrub: 1.0,
            key: String::new(),
            data: None,
            rx: None,
            cancel: Arc::new(AtomicBool::new(false)),
            busy: false,
            clock: Instant::now(),
        }
    }
}

impl SmokePreviewState {
    /// Istante da mostrare (s) per un fumo con `time` secondi simulati: in loop, con una breve
    /// pausa sull'istante finale, oppure fermo su `scrub`.
    pub fn shown_time(&self, time: f32) -> f32 {
        if !self.playing {
            return self.scrub.clamp(0.0, 1.0) * time;
        }
        // Velocità reale, ma un giro dura almeno 2 s e al massimo 8 s.
        let lap = time.clamp(2.0, 8.0);
        let t = self.clock.elapsed().as_secs_f32() % (lap + 0.8);
        (t / lap).min(1.0) * time
    }
}

/// Il tempo non è nella chiave: lo decide il cursore della timeline. Conta fin dove arriva.
fn key(s: &SmokeCloud, geometry_rev: u64, horizon: f32) -> String {
    let mut s = s.sanitized();
    s.sim.time = horizon;
    format!(
        "{:?}|{:?}|{:?}|{}|{}|{}|{}",
        s.sim, s.center, s.size, s.rotation_deg, s.seed, s.collisions, geometry_rev
    )
}

/// Tiene aggiornata l'anteprima del fumo selezionato (da chiamare a ogni fotogramma).
pub fn update(app: &mut ForgeApp) {
    if let Some(rx) = &app.smoke_preview.rx {
        if let Ok(r) = rx.try_recv() {
            app.smoke_preview.rx = None;
            app.smoke_preview.busy = false;
            if let (Some(p), Some(i)) = (r, app.selected_smoke) {
                app.smoke_preview.data = Some((i, Arc::new(p)));
            }
        }
    }
    let target = app
        .selected_smoke
        .filter(|_| app.smoke_preview.enabled)
        .and_then(|i| app.scene.smokes.get(i).map(|s| (i, s)))
        .filter(|(_, s)| s.enabled && s.mode == SmokeMode::Simulated);
    let Some((i, s)) = target else {
        // Deselezionato (per esempio con un clic nella viewport): l'anteprima pronta resta, così
        // riselezionando lo stesso fumo non si rifà; si ferma solo un calcolo in corso.
        if app.smoke_preview.busy {
            app.smoke_preview.cancel.store(true, Ordering::SeqCst);
            app.smoke_preview.key.clear();
            app.smoke_preview.busy = false;
            app.smoke_preview.rx = None;
        }
        return;
    };
    // Mentre l'animazione gira non si ricalcola: si guarda l'anteprima già pronta.
    if app.timeline_ui.playing && app.smoke_preview.data.is_some() {
        return;
    }
    let tl = &app.scene.timeline;
    let horizon = (tl.duration - tl.start_of(&forge_scene::Target::Smoke(s.n))).max(0.1);
    let k = key(s, app.geometry_rev, horizon);
    if k == app.smoke_preview.key {
        return;
    }
    // Serve la geometria per far girare il fumo attorno agli oggetti: se non c'è ancora si
    // riprova al prossimo fotogramma.
    let Some(geo) = app.current_geometry_shared() else { return };
    let mut s = s.clone();
    s.sim.time = horizon;
    app.smoke_preview.cancel.store(true, Ordering::SeqCst);
    let cancel = Arc::new(AtomicBool::new(false));
    app.smoke_preview.cancel = cancel.clone();
    app.smoke_preview.key = k;
    app.smoke_preview.busy = true;
    if app.smoke_preview.data.as_ref().is_some_and(|d| d.0 != i) {
        app.smoke_preview.data = None;
    }
    let (tx, rx) = std::sync::mpsc::channel();
    app.smoke_preview.rx = Some(rx);
    let _ = std::thread::Builder::new().name("forge-anteprima-fumo".into()).spawn(move || {
        let t0 = Instant::now();
        let p = forge_render_core::smoke::smoke_preview(&s, &geo, &|| cancel.load(Ordering::SeqCst));
        if let Some(p) = &p {
            tracing::debug!(
                "anteprima del fumo: {} fotogrammi, celle di {:.1} cm, {:.0} ms",
                p.frames.len(),
                p.cell * 100.0,
                t0.elapsed().as_secs_f64() * 1e3
            );
        }
        let _ = tx.send(p);
    });
}

/// Disegna i punti dell'anteprima del fumo `i` (se pronta) e chiede il prossimo fotogramma.
pub fn draw(app: &ForgeApp, painter: &egui::Painter, rect: egui::Rect, i: usize) {
    let Some((idx, p)) = &app.smoke_preview.data else { return };
    if *idx != i || !app.smoke_preview.enabled {
        return;
    }
    let Some(s) = app.scene.smokes.get(i) else { return };
    let time = s.sanitized().sim.time;
    let t = if crate::ui::smoke::SHOW_TIME_CONTROLS {
        app.smoke_preview.shown_time(time)
    } else {
        time
    };
    let Some(points) = p.at(t) else { return };
    let cam = &app.view_camera;
    let col = egui::Color32::from_rgb(225, 235, 245);
    for &(q, d) in points {
        if let Some(pos) = crate::viewport::project(cam, rect, q) {
            painter.circle_filled(pos, 1.4 + d * 0.8, col.gamma_multiply(0.12 + 0.45 * d));
        }
    }
    // Istante mostrato: si vede che il giro ricomincia da 0 (non è un nuovo calcolo).
    let mut label = crate::i18n::tf("Anteprima del fumo: {} / {} s", &[format!("{t:.1}"), format!("{time:.1}")]);
    if s.preview_only {
        label.push_str(crate::i18n::tr(" · solo anteprima, il render non lo calcola"));
    }
    painter.text(
        rect.left_bottom() + egui::vec2(10.0, -10.0),
        egui::Align2::LEFT_BOTTOM,
        label,
        egui::FontId::proportional(12.0),
        egui::Color32::from_rgb(225, 235, 245),
    );
    if crate::ui::smoke::SHOW_TIME_CONTROLS && app.smoke_preview.playing {
        painter.ctx().request_repaint();
    }
}

/// Comandi dell'anteprima nel pannello del fumo simulato.
pub fn controls(ui: &mut egui::Ui, st: &mut SmokePreviewState, time: f32) {
    use crate::i18n::{tf, tr};
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut st.enabled, tr("Anteprima in viewport")).on_hover_text(tr(
            "Mostra nella viewport, come una nuvola di punti, dove va il fumo e come si muove fino all'istante scelto (simulazione grossolana e veloce, solo per trovare posizione e impostazioni: il render la rifà fine)",
        ));
        if st.enabled && crate::ui::smoke::SHOW_TIME_CONTROLS {
            let label = if st.playing { tr("Ferma") } else { tr("Riproduci") };
            if ui.small_button(label).clicked() {
                st.playing = !st.playing;
            }
        }
        if st.enabled && st.busy {
            ui.spinner();
        }
    });
    if !crate::ui::smoke::SHOW_TIME_CONTROLS {
        return;
    }
    if st.enabled && !st.playing {
        let mut sec = st.scrub * time;
        if ui
            .add(
                egui::Slider::new(&mut sec, 0.0..=time)
                    .suffix(" s")
                    .max_decimals(2)
                    .text(tr("Tempo dell'anteprima")),
            )
            .changed()
        {
            st.scrub = (sec / time.max(1e-3)).clamp(0.0, 1.0);
        }
    } else if st.enabled {
        ui.label(
            egui::RichText::new(tf("Il fumo si muove da 0 a {} s, poi riparte", &[format!("{time:.1}")]))
                .small()
                .weak(),
        );
    }
}
