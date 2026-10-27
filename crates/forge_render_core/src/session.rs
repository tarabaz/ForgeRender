//! Sessione di rendering progressivo su un thread dedicato.
//!
//! L'interfaccia non viene mai bloccata: invia comandi tramite canale e legge
//! lo stato/l'ultimo frame da memoria condivisa. Ogni passata completata
//! aggiunge un campione reale per pixel all'accumulatore; una passata
//! interrotta (pausa, annullamento, nuova scena) viene scartata per intero,
//! così l'accumulo non contiene mai campioni parziali.

use crate::{Accumulator, RenderBackend, RenderScene, RenderSettings};
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
            self.width as f64 * self.height as f64 * self.samples as f64 / s / 1e6
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
}

enum Cmd {
    Start(Box<Job>),
    Pause,
    Resume,
    Cancel,
    SetTarget(u32),
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
}

fn worker(rx: Receiver<Cmd>, shared: Arc<Shared>) {
    let mut active: Option<Active> = None;
    let mut state = SessionState::Idle;

    let publish = |a: &Active, shared: &Shared| {
        let frame = Frame {
            job_id: a.job.id,
            samples: a.accum.samples,
            width: a.accum.width,
            height: a.accum.height,
            linear: Arc::new(a.accum.resolve()),
        };
        *shared.frame.lock().unwrap() = Some(frame);
        shared.frame_version.fetch_add(1, Ordering::Release);
    };
    let update_progress = |a: &Active, state: &SessionState, last_pass: Duration, shared: &Shared| {
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
    };

    loop {
        let wants_render = state == SessionState::Rendering;
        shared.interrupt.store(false, Ordering::SeqCst);
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
                    };
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
                    update_progress(&a, &state, Duration::ZERO, &shared);
                    active = Some(a);
                }
                Cmd::Pause => {
                    if let (Some(a), SessionState::Rendering) = (active.as_mut(), &state) {
                        stop_clock(a);
                        state = SessionState::Paused;
                        publish(a, &shared);
                        update_progress(a, &state, Duration::ZERO, &shared);
                    }
                }
                Cmd::Resume => {
                    if let Some(a) = active.as_mut() {
                        if matches!(state, SessionState::Paused | SessionState::Cancelled) && !a.done() {
                            a.running_since = Some(Instant::now());
                            state = SessionState::Rendering;
                            update_progress(a, &state, Duration::ZERO, &shared);
                        }
                    }
                }
                Cmd::Cancel => {
                    if let Some(a) = active.as_mut() {
                        if state.is_active() {
                            stop_clock(a);
                            state = SessionState::Cancelled;
                            publish(a, &shared);
                            update_progress(a, &state, Duration::ZERO, &shared);
                            tracing::info!("render interrotto a {} campioni", a.accum.samples);
                        }
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
                            publish(a, &shared);
                        }
                        update_progress(a, &state, Duration::ZERO, &shared);
                    }
                }
            }
            continue;
        }

        // Una passata da 1 campione per pixel.
        let Some(a) = active.as_mut() else {
            state = SessionState::Idle;
            continue;
        };
        let t0 = Instant::now();
        let sample_index = a.accum.samples;
        let result = a
            .job
            .backend
            .render_pass(&a.job.scene, &a.job.settings, sample_index, &mut a.pass, &shared.interrupt);
        match result {
            Ok(true) => {
                a.accum.add_pass(&a.pass);
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
                // Le prime passate si mostrano subito, poi al massimo ~4 volte al secondo.
                let interval = if a.accum.samples < 8 {
                    Duration::ZERO
                } else {
                    Duration::from_millis(250)
                };
                if done || a.last_publish.elapsed() >= interval {
                    publish(a, &shared);
                    a.last_publish = Instant::now();
                }
                update_progress(a, &state, dt, &shared);
            }
            Ok(false) => {} // interrotta: si torna a leggere i comandi
            Err(e) => {
                tracing::error!("errore del backend: {e:#}");
                if let Some(t) = a.running_since.take() {
                    a.elapsed += t.elapsed();
                }
                state = SessionState::Failed(format!("{e:#}"));
                publish(a, &shared);
                update_progress(a, &state, Duration::ZERO, &shared);
            }
        }
    }
}

impl Active {
    fn done(&self) -> bool {
        let t = self.job.settings.target_samples;
        t != 0 && self.accum.samples >= t
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
            out: &mut [[f32; 3]],
            cancel: &AtomicBool,
        ) -> anyhow::Result<bool> {
            let t0 = Instant::now();
            while t0.elapsed() < self.0 {
                if cancel.load(Ordering::Relaxed) {
                    return Ok(false);
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            out.fill([1.0; 3]);
            Ok(true)
        }
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
        std::thread::sleep(Duration::from_millis(30));
        let p1 = s.progress();
        assert_eq!(p1.state, SessionState::Paused);
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(s.progress().samples, p1.samples, "in pausa non si accumula");
        // L'immagine pubblicata in pausa corrisponde esattamente allo stato.
        assert_eq!(s.latest_frame().unwrap().samples, p1.samples);
        s.resume();
        std::thread::sleep(Duration::from_millis(40));
        assert!(s.progress().samples > p1.samples);
        s.cancel();
        std::thread::sleep(Duration::from_millis(20));
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
