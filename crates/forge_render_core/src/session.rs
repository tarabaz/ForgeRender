//! Sessione di rendering progressivo su un thread dedicato.
//!
//! L'interfaccia non viene mai bloccata: invia comandi tramite canale e legge
//! lo stato/l'ultimo frame da memoria condivisa. Ogni passata completata
//! aggiunge un campione reale per pixel all'accumulatore; una passata
//! interrotta (pausa, annullamento, nuova scena) viene scartata per intero,
//! così l'accumulo non contiene mai campioni parziali.

use crate::adaptive::TileMask;
use crate::{Accumulator, DenoiseMode, DenoiseSettings, Denoiser, Guides, RenderBackend, RenderScene, RenderSettings};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq)]
pub enum SessionState {
    Idle,
    Rendering,
    Paused,
    Completed,
    Cancelled,
    Failed(String),
}

impl SessionState {
    pub fn label(&self) -> String {
        match self {
            SessionState::Idle => "In attesa".into(),
            SessionState::Rendering => "Rendering in corso".into(),
            SessionState::Paused => "In pausa".into(),
            SessionState::Completed => "Completato".into(),
            SessionState::Cancelled => "Interrotto".into(),
            SessionState::Failed(e) => format!("Errore: {e}"),
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self, SessionState::Rendering | SessionState::Paused)
    }
}

#[derive(Clone, Debug)]
pub struct Progress {
    pub job_id: u64,
    pub state: SessionState,
    pub samples: u32,
    /// 0 = senza limite.
    pub target: u32,
    pub width: u32,
    pub height: u32,
    /// Tempo di rendering effettivo (le pause sono escluse).
    pub elapsed: Duration,
    pub last_pass: Duration,
    pub backend: String,
    pub device: String,
    /// Stato della riduzione del rumore (filtro, dispositivo, tempo o errore); vuoto se inattiva.
    pub denoiser: String,
    /// Frazione dell'immagine che riceve ancora campioni (1 senza campionamento adattivo).
    pub active_fraction: f32,
    /// Campioni calcolati davvero, sommati su tutti i pixel (col campionamento adattivo
    /// sono meno di `samples × pixel`).
    pub pixel_samples: u64,
}

impl Progress {
    pub fn avg_pass(&self) -> Duration {
        if self.samples == 0 {
            Duration::ZERO
        } else {
            self.elapsed / self.samples
        }
    }

    /// Campioni di pixel calcolati al secondo (milioni).
    pub fn msamples_per_sec(&self) -> f64 {
        let s = self.elapsed.as_secs_f64();
        if s <= 0.0 {
            0.0
        } else {
            // Campioni calcolati davvero (col campionamento adattivo i pixel fermi non contano).
            let n = if self.pixel_samples > 0 {
                self.pixel_samples as f64
            } else {
                self.width as f64 * self.height as f64 * self.samples as f64
            };
            n / s / 1e6
        }
    }

    pub fn fraction(&self) -> f32 {
        if self.target == 0 {
            0.0
        } else {
            (self.samples as f32 / self.target as f32).min(1.0)
        }
    }
}

/// Immagine corrente: media lineare dei campioni accumulati.
#[derive(Clone, Debug)]
pub struct Frame {
    pub job_id: u64,
    pub samples: u32,
    pub width: u32,
    pub height: u32,
    pub linear: Arc<Vec<[f32; 3]>>,
    /// Immagine ripulita dal denoiser, calcolata a `denoised_samples` campioni.
    pub denoised: Option<Arc<Vec<[f32; 3]>>>,
    pub denoised_samples: u32,
    /// Buffer guida usati dal denoiser (per la visualizzazione di controllo).
    pub guides: Option<Arc<Guides>>,
    /// Diagnostica del campionamento adattivo (solo se è acceso).
    pub adaptive: Option<Arc<AdaptiveDiag>>,
}

/// Diagnostica del campionamento adattivo: dove sono finiti i campioni e quanta parte è
/// nella metà "B" usata per stimare l'errore. Non cambia il rendering.
#[derive(Clone, Debug)]
pub struct AdaptiveDiag {
    /// Campioni di ogni pixel (riga per riga).
    pub counts: Vec<u32>,
    pub b_fraction: Option<crate::BFraction>,
}

impl Frame {
    /// Immagine da mostrare/esportare: grezza o miscelata con quella ripulita.
    pub fn display(&self, strength: f32) -> std::borrow::Cow<'_, [[f32; 3]]> {
        match &self.denoised {
            Some(d) if strength >= 1.0 => std::borrow::Cow::Borrowed(d.as_slice()),
            Some(d) if strength > 0.0 => std::borrow::Cow::Owned(crate::mix_denoised(&self.linear, d, strength)),
            _ => std::borrow::Cow::Borrowed(self.linear.as_slice()),
        }
    }
}

enum Cmd {
    Start(Box<Job>),
    Pause,
    Resume,
    Cancel,
    SetTarget(u32),
    /// Soglia di rumore e campioni minimi del campionamento adattivo.
    SetAdaptive(f32, u32),
    SetDenoise(DenoiseSettings),
    SetDenoiser(Option<Box<dyn Denoiser>>),
    Shutdown,
}

struct Job {
    id: u64,
    scene: Arc<RenderScene>,
    settings: RenderSettings,
    backend: Box<dyn RenderBackend>,
    resume: Option<Accumulator>,
}

struct Shared {
    progress: Mutex<Progress>,
    frame: Mutex<Option<Frame>>,
    frame_version: AtomicU64,
    interrupt: AtomicBool,
}

pub struct RenderSession {
    tx: Sender<Cmd>,
    shared: Arc<Shared>,
    next_job: AtomicU64,
    handle: Option<JoinHandle<()>>,
}

impl Default for RenderSession {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderSession {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        let shared = Arc::new(Shared {
            progress: Mutex::new(Progress {
                job_id: 0,
                state: SessionState::Idle,
                samples: 0,
                target: 0,
                width: 0,
                height: 0,
                elapsed: Duration::ZERO,
                last_pass: Duration::ZERO,
                backend: String::new(),
                device: String::new(),
                denoiser: String::new(),
                active_fraction: 1.0,
                pixel_samples: 0,
            }),
            frame: Mutex::new(None),
            frame_version: AtomicU64::new(0),
            interrupt: AtomicBool::new(false),
        });
        let s2 = shared.clone();
        let handle = std::thread::Builder::new()
            .name("forge-render-session".into())
            .spawn(move || worker(rx, s2))
            .expect("impossibile creare il thread di rendering");
        Self {
            tx,
            shared,
            next_job: AtomicU64::new(1),
            handle: Some(handle),
        }
    }

    fn send(&self, cmd: Cmd) {
        // L'ordine conta: prima il comando, poi l'interruzione della passata in corso.
        let _ = self.tx.send(cmd);
        self.shared.interrupt.store(true, Ordering::SeqCst);
    }

    /// Avvia un nuovo rendering scartando qualsiasi accumulo precedente.
    pub fn start(&self, scene: Arc<RenderScene>, settings: RenderSettings, backend: Box<dyn RenderBackend>) -> u64 {
        self.start_with(scene, settings, backend, None)
    }

    /// Avvia riprendendo da un accumulatore salvato (stessa scena e impostazioni).
    pub fn start_with(
        &self,
        scene: Arc<RenderScene>,
        settings: RenderSettings,
        backend: Box<dyn RenderBackend>,
        resume: Option<Accumulator>,
    ) -> u64 {
        let id = self.next_job.fetch_add(1, Ordering::SeqCst);
        self.send(Cmd::Start(Box::new(Job {
            id,
            scene,
            settings: settings.sanitized(),
            backend,
            resume,
        })));
        id
    }

    pub fn pause(&self) {
        self.send(Cmd::Pause);
    }

    pub fn resume(&self) {
        self.send(Cmd::Resume);
    }

    /// Ferma il rendering mantenendo l'immagine corrente.
    pub fn cancel(&self) {
        self.send(Cmd::Cancel);
    }

    /// Cambia il numero di campioni obiettivo senza ripartire (0 = illimitato).
    pub fn set_target(&self, target: u32) {
        let _ = self.tx.send(Cmd::SetTarget(target));
    }

    /// Cambia la soglia del campionamento adattivo senza ripartire (0 = spento).
    pub fn set_adaptive(&self, threshold: f32, min_samples: u32) {
        let _ = self.tx.send(Cmd::SetAdaptive(threshold, min_samples));
    }

    /// Aggiorna le impostazioni del denoiser senza ripartire.
    pub fn set_denoise(&self, settings: DenoiseSettings) {
        let _ = self.tx.send(Cmd::SetDenoise(settings));
    }

    /// Installa (o rimuove) il denoiser usato dalla sessione.
    pub fn set_denoiser(&self, denoiser: Option<Box<dyn Denoiser>>) {
        let _ = self.tx.send(Cmd::SetDenoiser(denoiser));
    }

    pub fn progress(&self) -> Progress {
        self.shared.progress.lock().unwrap().clone()
    }

    pub fn frame_version(&self) -> u64 {
        self.shared.frame_version.load(Ordering::Acquire)
    }

    pub fn latest_frame(&self) -> Option<Frame> {
        self.shared.frame.lock().unwrap().clone()
    }

    /// Attende (bloccando) la fine del job indicato. Usato da CLI e test.
    pub fn wait(&self, job_id: u64, timeout: Duration) -> Progress {
        let t0 = Instant::now();
        loop {
            let p = self.progress();
            if p.job_id == job_id && !matches!(p.state, SessionState::Rendering | SessionState::Idle) {
                return p;
            }
            if t0.elapsed() > timeout {
                return p;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

impl Drop for RenderSession {
    fn drop(&mut self) {
        self.send(Cmd::Shutdown);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

struct Active {
    job: Job,
    accum: Accumulator,
    pass: Vec<[f32; 3]>,
    running_since: Option<Instant>,
    elapsed: Duration,
    last_publish: Instant,
    denoised: Option<Arc<Vec<[f32; 3]>>>,
    /// Campioni e "chiave" (qualità, CUDA, guide) dell'ultima immagine ripulita.
    denoised_at: Option<(u32, DenoiseKey)>,
    /// Soglia della prossima anteprima ripulita.
    next_preview: u32,
    /// Riquadri che ricevono ancora campioni (campionamento adattivo acceso).
    mask: Option<TileMask>,
    /// La prima valutazione della maschera è già stata registrata nel log.
    mask_logged: bool,
}

type DenoiseKey = (crate::DenoiseQuality, bool, bool);

/// Denoiser installato, impostazioni correnti e cache dei buffer guida.
struct DenoiseState {
    denoiser: Option<Box<dyn Denoiser>>,
    settings: DenoiseSettings,
    /// (id della scena, larghezza, altezza) → guide.
    guides: Option<((u64, u32, u32), Arc<Guides>)>,
    status: String,
}

impl DenoiseState {
    fn enabled(&self) -> bool {
        self.denoiser.is_some() && self.settings.mode != DenoiseMode::Off
    }

    fn key(&self, final_image: bool) -> DenoiseKey {
        let q = if final_image {
            self.settings.quality
        } else {
            self.settings.preview_quality
        };
        (q, self.settings.use_cuda, self.settings.use_guides)
    }

    /// Ripulisce l'immagine corrente. Restituisce falso se interrotto da un comando.
    fn run(&mut self, a: &mut Active, final_image: bool, interrupt: &AtomicBool) -> bool {
        if !self.enabled() {
            return true;
        }
        let key = self.key(final_image);
        let Some(denoiser) = self.denoiser.as_mut() else { return true };
        if a.accum.samples == 0 {
            return true;
        }
        let (w, h) = (a.accum.width, a.accum.height);
        let guides = if self.settings.use_guides {
            let gk = (a.job.scene.id, w, h);
            match &self.guides {
                Some((k, g)) if *k == gk => Some(g.clone()),
                _ => {
                    let t0 = Instant::now();
                    let Some(g) = denoiser.guides(&a.job.scene, &a.job.settings, interrupt) else {
                        return false;
                    };
                    tracing::info!(
                        "buffer guida (albedo, normali) calcolati in {:.0} ms",
                        t0.elapsed().as_secs_f64() * 1e3
                    );
                    let g = Arc::new(g);
                    self.guides = Some((gk, g.clone()));
                    Some(g)
                }
            }
        } else {
            None
        };
        let color = a.accum.resolve();
        let t0 = Instant::now();
        match denoiser.denoise(&color, guides.as_deref(), w, h, key.0, key.1) {
            Ok(img) => {
                let ms = t0.elapsed().as_secs_f64() * 1e3;
                self.status = format!(
                    "{} · {} · {:.0} ms a {} campioni",
                    denoiser.name(),
                    key.0.label(),
                    ms,
                    a.accum.samples
                );
                tracing::debug!("denoise: {}", self.status);
                a.denoised = Some(Arc::new(img));
            }
            Err(e) => {
                tracing::warn!("riduzione del rumore fallita: {e:#}");
                self.status = format!("errore del denoiser: {e:#}");
                a.denoised = None;
            }
        }
        a.denoised_at = Some((a.accum.samples, key));
        true
    }

    /// Vero se l'immagine finale ripulita manca o non è aggiornata.
    fn final_pending(&self, a: &Active) -> bool {
        self.enabled() && a.accum.samples > 0 && a.denoised_at != Some((a.accum.samples, self.key(true)))
    }
}

fn worker(rx: Receiver<Cmd>, shared: Arc<Shared>) {
    let mut active: Option<Active> = None;
    let mut state = SessionState::Idle;
    let mut dn = DenoiseState {
        denoiser: None,
        settings: DenoiseSettings::default(),
        guides: None,
        status: String::new(),
    };

    let publish = |a: &Active, dn: &DenoiseState, shared: &Shared| {
        // In modalità "solo finale" l'immagine ripulita si mostra solo se è aggiornata.
        let fresh = a.denoised_at.map(|d| d.0) == Some(a.accum.samples);
        let on = dn.enabled() && a.denoised.is_some() && (fresh || dn.settings.mode == DenoiseMode::Progressive);
        let frame = Frame {
            job_id: a.job.id,
            samples: a.accum.samples,
            width: a.accum.width,
            height: a.accum.height,
            linear: Arc::new(a.accum.resolve()),
            denoised: if on { a.denoised.clone() } else { None },
            denoised_samples: if on { a.denoised_at.map(|d| d.0).unwrap_or(0) } else { 0 },
            guides: if on { dn.guides.as_ref().map(|g| g.1.clone()) } else { None },
            adaptive: a.mask.as_ref().map(|_| {
                Arc::new(AdaptiveDiag {
                    counts: a.accum.counts().to_vec(),
                    b_fraction: a.accum.b_fraction(),
                })
            }),
        };
        *shared.frame.lock().unwrap() = Some(frame);
        shared.frame_version.fetch_add(1, Ordering::Release);
    };
    let update_progress = |a: &Active, dn: &DenoiseState, state: &SessionState, last_pass: Duration, shared: &Shared| {
        let mut p = shared.progress.lock().unwrap();
        p.job_id = a.job.id;
        p.state = state.clone();
        p.samples = a.accum.samples;
        p.target = a.job.settings.target_samples;
        p.width = a.accum.width;
        p.height = a.accum.height;
        p.elapsed = a.elapsed + a.running_since.map(|t| t.elapsed()).unwrap_or_default();
        p.last_pass = last_pass;
        p.backend = a.job.backend.name();
        p.device = a.job.backend.device_info();
        p.denoiser = if dn.enabled() { dn.status.clone() } else { String::new() };
        p.active_fraction = a.mask.as_ref().map(TileMask::active_fraction).unwrap_or(1.0);
        p.pixel_samples = a.accum.total_samples();
    };

    loop {
        let wants_render = state == SessionState::Rendering;
        shared.interrupt.store(false, Ordering::SeqCst);
        // Fermi (completato, in pausa, interrotto): si completa l'immagine finale ripulita.
        if !wants_render {
            if let Some(a) = active.as_mut() {
                if dn.final_pending(a) && dn.run(a, true, &shared.interrupt) {
                    publish(a, &dn, &shared);
                    update_progress(a, &dn, &state, Duration::ZERO, &shared);
                }
            }
            shared.interrupt.store(false, Ordering::SeqCst);
        }
        let cmd = if wants_render {
            rx.try_recv().ok()
        } else {
            match rx.recv() {
                Ok(c) => Some(c),
                Err(_) => return,
            }
        };

        if let Some(cmd) = cmd {
            let stop_clock = |a: &mut Active| {
                if let Some(t) = a.running_since.take() {
                    a.elapsed += t.elapsed();
                }
            };
            match cmd {
                Cmd::Shutdown => return,
                Cmd::Start(job) => {
                    let job = *job;
                    dn.settings = job.settings.denoise.clone();
                    let (w, h) = (job.settings.width, job.settings.height);
                    let accum = match job.resume {
                        Some(ref a) if a.width == w && a.height == h => a.clone(),
                        _ => Accumulator::new(w, h),
                    };
                    let a = Active {
                        pass: vec![[0.0; 3]; accum.pixel_count()],
                        accum,
                        job: Job { resume: None, ..job },
                        running_since: Some(Instant::now()),
                        elapsed: Duration::ZERO,
                        last_publish: Instant::now() - Duration::from_secs(1),
                        denoised: None,
                        denoised_at: None,
                        next_preview: dn.settings.first_preview_samples,
                        mask: None,
                        mask_logged: false,
                    };
                    let mut a = a;
                    if a.job.settings.noise_threshold > 0.0 {
                        a.mask = Some(TileMask::all(w, h));
                    }
                    tracing::info!(
                        "render avviato: {}x{}, {} campioni, backend {}",
                        w,
                        h,
                        if a.job.settings.target_samples == 0 {
                            "∞".to_string()
                        } else {
                            a.job.settings.target_samples.to_string()
                        },
                        a.job.backend.name()
                    );
                    state = if a.done() {
                        SessionState::Completed
                    } else {
                        SessionState::Rendering
                    };
                    update_progress(&a, &dn, &state, Duration::ZERO, &shared);
                    active = Some(a);
                }
                Cmd::Pause => {
                    if let (Some(a), SessionState::Rendering) = (active.as_mut(), &state) {
                        stop_clock(a);
                        state = SessionState::Paused;
                        publish(a, &dn, &shared);
                        update_progress(a, &dn, &state, Duration::ZERO, &shared);
                    }
                }
                Cmd::Resume => {
                    if let Some(a) = active.as_mut() {
                        if matches!(state, SessionState::Paused | SessionState::Cancelled) && !a.done() {
                            a.running_since = Some(Instant::now());
                            state = SessionState::Rendering;
                            update_progress(a, &dn, &state, Duration::ZERO, &shared);
                        }
                    }
                }
                Cmd::Cancel => {
                    if let Some(a) = active.as_mut() {
                        if state.is_active() {
                            stop_clock(a);
                            state = SessionState::Cancelled;
                            publish(a, &dn, &shared);
                            update_progress(a, &dn, &state, Duration::ZERO, &shared);
                            tracing::info!("render interrotto a {} campioni", a.accum.samples);
                        }
                    }
                }
                Cmd::SetDenoise(d) => {
                    dn.settings = d;
                    if let Some(a) = active.as_mut() {
                        if dn.settings.mode == DenoiseMode::Progressive && a.next_preview <= a.accum.samples {
                            a.next_preview = a.accum.samples.max(dn.settings.first_preview_samples);
                        }
                        publish(a, &dn, &shared);
                        update_progress(a, &dn, &state, Duration::ZERO, &shared);
                    }
                }
                Cmd::SetDenoiser(d) => {
                    dn.denoiser = d;
                    dn.guides = None;
                    dn.status.clear();
                    if let Some(a) = active.as_mut() {
                        a.denoised = None;
                        a.denoised_at = None;
                        publish(a, &dn, &shared);
                    }
                }
                Cmd::SetAdaptive(threshold, min) => {
                    if let Some(a) = active.as_mut() {
                        a.job.settings.noise_threshold = threshold;
                        a.job.settings.adaptive_min_samples = min;
                        if threshold <= 0.0 {
                            a.mask = None;
                        } else if a.mask.is_none() {
                            a.mask = Some(TileMask::all(a.accum.width, a.accum.height));
                        }
                        if state == SessionState::Completed && !a.done() {
                            a.running_since = Some(Instant::now());
                            state = SessionState::Rendering;
                        }
                        update_progress(a, &dn, &state, Duration::ZERO, &shared);
                    }
                }
                Cmd::SetTarget(t) => {
                    if let Some(a) = active.as_mut() {
                        a.job.settings.target_samples = t;
                        if state == SessionState::Completed && !a.done() {
                            a.running_since = Some(Instant::now());
                            state = SessionState::Rendering;
                        } else if state == SessionState::Rendering && a.done() {
                            stop_clock(a);
                            state = SessionState::Completed;
                            publish(a, &dn, &shared);
                        }
                        update_progress(a, &dn, &state, Duration::ZERO, &shared);
                    }
                }
            }
            continue;
        }

        // Una passata: uno o più campioni per pixel, senza superare l'obiettivo.
        let Some(a) = active.as_mut() else {
            state = SessionState::Idle;
            continue;
        };
        let t0 = Instant::now();
        let sample_index = a.accum.samples;
        let target = a.job.settings.target_samples;
        let max_samples = if target == 0 {
            u32::MAX
        } else {
            target.saturating_sub(sample_index).max(1)
        };
        let result = match &a.mask {
            Some(m) => a.job.backend.render_pass_masked(
                &a.job.scene,
                &a.job.settings,
                sample_index,
                max_samples,
                &mut a.pass,
                &shared.interrupt,
                m,
            ),
            None => a.job.backend.render_pass(
                &a.job.scene,
                &a.job.settings,
                sample_index,
                max_samples,
                &mut a.pass,
                &shared.interrupt,
            ),
        };
        match result {
            Ok(n) if n > 0 => {
                a.accum.add_samples_masked(&a.pass, n, a.mask.as_ref());
                // Campionamento adattivo: si fermano i riquadri già sotto la soglia di rumore.
                let st = &a.job.settings;
                if let Some(m) = a.mask.as_mut() {
                    if a.accum.samples >= st.adaptive_min_samples {
                        m.update(&a.accum, st.noise_threshold);
                        if !a.mask_logged {
                            a.mask_logged = true;
                            if let Some(f) = a.accum.b_fraction() {
                                tracing::info!(
                                    "adattivo: prima valutazione a {} campioni (passata di {n}); metà B n_B/n media {:.3}, min {:.3}, max {:.3}; riquadri attivi {:.1} %",
                                    a.accum.samples,
                                    f.mean,
                                    f.min,
                                    f.max,
                                    m.active_fraction() * 100.0
                                );
                            }
                        }
                    }
                }
                let dt = t0.elapsed();
                let done = a.done();
                if done {
                    if let Some(t) = a.running_since.take() {
                        a.elapsed += t.elapsed();
                    }
                    state = SessionState::Completed;
                    tracing::info!(
                        "render completato: {} campioni in {:.2} s",
                        a.accum.samples,
                        a.elapsed.as_secs_f64()
                    );
                }
                if done {
                    // Se un comando interrompe il calcolo, si riprova appena la sessione è ferma.
                    dn.run(a, true, &shared.interrupt);
                } else if dn.enabled()
                    && dn.settings.mode == DenoiseMode::Progressive
                    && a.accum.samples >= a.next_preview
                    && dn.run(a, false, &shared.interrupt)
                {
                    a.next_preview = a.accum.samples.saturating_mul(2);
                    a.last_publish = Instant::now() - Duration::from_secs(1);
                }
                // Le prime passate si mostrano subito, poi al massimo ~4 volte al secondo.
                let interval = if a.accum.samples < 8 {
                    Duration::ZERO
                } else {
                    Duration::from_millis(250)
                };
                if done || a.last_publish.elapsed() >= interval {
                    publish(a, &dn, &shared);
                    a.last_publish = Instant::now();
                }
                update_progress(a, &dn, &state, dt, &shared);
            }
            Ok(_) => {} // interrotta: si torna a leggere i comandi
            Err(e) => {
                tracing::error!("errore del backend: {e:#}");
                if let Some(t) = a.running_since.take() {
                    a.elapsed += t.elapsed();
                }
                state = SessionState::Failed(format!("{e:#}"));
                publish(a, &dn, &shared);
                update_progress(a, &dn, &state, Duration::ZERO, &shared);
            }
        }
    }
}

impl Active {
    fn done(&self) -> bool {
        let t = self.job.settings.target_samples;
        (t != 0 && self.accum.samples >= t) || self.mask.as_ref().is_some_and(TileMask::all_done)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_scene::Scene;

    /// Backend di prova: restituisce 1.0 in ogni pixel, con un ritardo opzionale.
    struct ConstBackend(Duration);
    impl RenderBackend for ConstBackend {
        fn name(&self) -> String {
            "test".into()
        }
        fn device_info(&self) -> String {
            String::new()
        }
        fn render_pass(
            &mut self,
            _: &RenderScene,
            _: &RenderSettings,
            _: u32,
            _: u32,
            out: &mut [[f32; 3]],
            cancel: &AtomicBool,
        ) -> anyhow::Result<u32> {
            let t0 = Instant::now();
            while t0.elapsed() < self.0 {
                if cancel.load(Ordering::Relaxed) {
                    return Ok(0);
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            out.fill([1.0; 3]);
            Ok(1)
        }
    }

    /// Denoiser di prova: dimezza l'immagine e registra a quanti campioni è stato chiamato.
    struct HalfDenoiser(Arc<Mutex<Vec<u32>>>);
    impl Denoiser for HalfDenoiser {
        fn name(&self) -> String {
            "metà".into()
        }
        fn guides(&mut self, _: &RenderScene, s: &RenderSettings, _: &AtomicBool) -> Option<Guides> {
            let n = (s.width * s.height) as usize;
            Some(Guides {
                width: s.width,
                height: s.height,
                albedo: vec![[1.0; 3]; n],
                normal: vec![[0.0, 0.0, 1.0]; n],
            })
        }
        fn denoise(
            &mut self,
            color: &[[f32; 3]],
            guides: Option<&Guides>,
            _: u32,
            _: u32,
            _: crate::DenoiseQuality,
            _: bool,
        ) -> anyhow::Result<Vec<[f32; 3]>> {
            assert!(guides.is_some());
            self.0.lock().unwrap().push(0);
            Ok(color.iter().map(|p| p.map(|v| v * 0.5)).collect())
        }
    }

    #[test]
    fn denoise_final_and_progressive() {
        let s = RenderSession::new();
        let calls = Arc::new(Mutex::new(Vec::new()));
        s.set_denoiser(Some(Box::new(HalfDenoiser(calls.clone()))));
        let scene = Arc::new(RenderScene::compile(&Scene::default()));
        // Solo finale: una chiamata, immagine ripulita aggiornata al numero finale di campioni.
        let id = s.start(scene.clone(), settings(10), Box::new(ConstBackend(Duration::ZERO)));
        s.wait(id, Duration::from_secs(10));
        std::thread::sleep(Duration::from_millis(20));
        let f = s.latest_frame().unwrap();
        assert_eq!((f.samples, f.denoised_samples), (10, 10));
        assert!(f.denoised.as_ref().unwrap().iter().all(|p| *p == [0.5; 3]));
        assert_eq!(f.display(0.5)[0], [0.75; 3], "miscela 50 %");
        assert_eq!(calls.lock().unwrap().len(), 1);
        assert!(s.progress().denoiser.contains("metà"));

        // Anteprima: ripulita a 8, 16, 32 e alla fine (40).
        calls.lock().unwrap().clear();
        let mut st = settings(40);
        st.denoise.mode = DenoiseMode::Progressive;
        let id = s.start(scene.clone(), st, Box::new(ConstBackend(Duration::ZERO)));
        s.wait(id, Duration::from_secs(10));
        std::thread::sleep(Duration::from_millis(20));
        assert_eq!(calls.lock().unwrap().len(), 4);
        assert_eq!(s.latest_frame().unwrap().denoised_samples, 40);

        // Disattivato: nessuna immagine ripulita.
        calls.lock().unwrap().clear();
        let mut st = settings(5);
        st.denoise.mode = DenoiseMode::Off;
        let id = s.start(scene.clone(), st, Box::new(ConstBackend(Duration::ZERO)));
        s.wait(id, Duration::from_secs(10));
        std::thread::sleep(Duration::from_millis(20));
        assert!(s.latest_frame().unwrap().denoised.is_none());
        assert!(calls.lock().unwrap().is_empty());

        // Attivato a render finito (cambio d'impostazione dal pannello): si ripulisce subito.
        s.set_denoise(DenoiseSettings::default());
        std::thread::sleep(Duration::from_millis(50));
        let f = s.latest_frame().unwrap();
        assert_eq!(f.denoised_samples, 5);
        assert_eq!(calls.lock().unwrap().len(), 1);
    }

    fn settings(target: u32) -> RenderSettings {
        RenderSettings {
            width: 16,
            height: 16,
            target_samples: target,
            ..Default::default()
        }
    }

    #[test]
    fn renders_exact_sample_count() {
        let s = RenderSession::new();
        let scene = Arc::new(RenderScene::compile(&Scene::default()));
        let id = s.start(scene, settings(10), Box::new(ConstBackend(Duration::ZERO)));
        let p = s.wait(id, Duration::from_secs(10));
        assert_eq!(p.state, SessionState::Completed);
        assert_eq!(p.samples, 10);
        let f = s.latest_frame().unwrap();
        assert_eq!(f.samples, 10);
        assert!(f.linear.iter().all(|p| *p == [1.0; 3]));
    }

    #[test]
    fn pause_resume_cancel_and_retarget() {
        let s = RenderSession::new();
        let scene = Arc::new(RenderScene::compile(&Scene::default()));
        let id = s.start(scene.clone(), settings(0), Box::new(ConstBackend(Duration::from_millis(2))));
        std::thread::sleep(Duration::from_millis(60));
        s.pause();
        // Sotto carico (altri test in parallelo) la pausa può arrivare dopo qualche millisecondo.
        let t0 = std::time::Instant::now();
        while s.progress().state != SessionState::Paused && t0.elapsed() < Duration::from_secs(2) {
            std::thread::sleep(Duration::from_millis(5));
        }
        std::thread::sleep(Duration::from_millis(10));
        let p1 = s.progress();
        assert_eq!(p1.state, SessionState::Paused);
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(s.progress().samples, p1.samples, "in pausa non si accumula");
        // L'immagine pubblicata in pausa corrisponde esattamente allo stato.
        assert_eq!(s.latest_frame().unwrap().samples, p1.samples);
        s.resume();
        // Sotto carico il thread della sessione può partire in ritardo: si aspetta il risultato.
        let wait_for = |ok: &dyn Fn(&Progress) -> bool| {
            let t0 = std::time::Instant::now();
            while !ok(&s.progress()) && t0.elapsed() < Duration::from_secs(2) {
                std::thread::sleep(Duration::from_millis(5));
            }
        };
        wait_for(&|p| p.samples > p1.samples);
        assert!(s.progress().samples > p1.samples);
        s.cancel();
        wait_for(&|p| p.state == SessionState::Cancelled);
        let p2 = s.progress();
        assert_eq!(p2.state, SessionState::Cancelled);
        // Nuovo obiettivo superiore: si riprende con "resume".
        s.set_target(p2.samples + 5);
        s.resume();
        std::thread::sleep(Duration::from_millis(20));
        let p3 = s.wait(id, Duration::from_secs(10));
        assert_eq!(p3.state, SessionState::Completed);
        assert_eq!(p3.samples, p2.samples + 5);
        // Un nuovo job riparte da zero.
        let id2 = s.start(scene, settings(3), Box::new(ConstBackend(Duration::ZERO)));
        let p4 = s.wait(id2, Duration::from_secs(10));
        assert_eq!((p4.job_id, p4.samples), (id2, 3));
    }
}
