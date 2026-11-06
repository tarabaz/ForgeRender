//! Quello che serve a un'esportazione lunga per andare avanti da sola: niente sospensione del PC
//! mentre lavora, carico scelto dall'utente e ripresa dai fotogrammi già fatti.
//!
//! La regola che tiene insieme il resto: il passaggio da un fotogramma all'altro sta in
//! [`eframe::App::logic`], che gira anche con la finestra ridotta a icona; il disegno
//! dell'interfaccia, invece, con la finestra ridotta non viene chiamato.

use crate::app::ForgeApp;
use forge_project::ProjectMeta;
use forge_render_core::RenderSettings;
use forge_scene::Scene;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Quanto può pesare l'esportazione sul PC.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportLoad {
    /// Tutta la potenza: è il valore di partenza, nessun limite.
    #[default]
    Full,
    /// Priorità bassa e pause sulla GPU, per continuare a usare il PC: il render dura di più.
    Shared,
}

/// Con [`ExportLoad::Shared`]: riposo della GPU dopo ogni passata, in frazione del tempo della
/// passata (0,25 = un quinto del tempo fermo).
pub const SHARED_GPU_REST: f32 = 0.25;

/// Stato chiesto a Windows durante un'esportazione. Si aggiorna sempre dallo stesso thread:
/// la richiesta di restare svegli vale per il thread che l'ha fatta.
#[derive(Default)]
pub struct Guard {
    active: bool,
    load: ExportLoad,
}

impl Guard {
    pub fn update(&mut self, active: bool, load: ExportLoad) {
        let load = if active { load } else { ExportLoad::Full };
        if active != self.active {
            self.active = active;
            sys::keep_awake(active);
            tracing::info!(
                "esportazione: sospensione automatica del PC {}",
                if active { "bloccata" } else { "di nuovo normale" }
            );
        }
        if load != self.load {
            self.load = load;
            sys::set_load(load);
            tracing::info!("esportazione: carico {load:?}");
        }
    }
}

#[cfg(windows)]
mod sys {
    use super::ExportLoad;
    use std::sync::atomic::{AtomicU32, Ordering};
    use windows::Win32::System::Power::{SetThreadExecutionState, ES_CONTINUOUS, ES_SYSTEM_REQUIRED};
    use windows::Win32::System::Threading::{
        GetCurrentProcess, GetPriorityClass, SetPriorityClass, BELOW_NORMAL_PRIORITY_CLASS, PROCESS_CREATION_FLAGS,
    };

    /// Lo schermo può spegnersi lo stesso, e lo spegnimento voluto dall'utente non si ferma.
    pub fn keep_awake(on: bool) {
        let flags = if on { ES_CONTINUOUS | ES_SYSTEM_REQUIRED } else { ES_CONTINUOUS };
        // SAFETY: nessun puntatore, cambia solo lo stato del thread chiamante.
        unsafe { SetThreadExecutionState(flags) };
    }

    /// La priorità di prima, da rimettere.
    static BEFORE: AtomicU32 = AtomicU32::new(0);

    /// Vale per tutti i thread del programma e per ffmpeg, che la eredita.
    pub fn set_load(load: ExportLoad) {
        // SAFETY: `GetCurrentProcess` è una pseudo-handle sempre valida, da non chiudere.
        unsafe {
            let me = GetCurrentProcess();
            let class = match load {
                ExportLoad::Shared => {
                    BEFORE.store(GetPriorityClass(me), Ordering::Relaxed);
                    BELOW_NORMAL_PRIORITY_CLASS
                }
                ExportLoad::Full => PROCESS_CREATION_FLAGS(BEFORE.load(Ordering::Relaxed)),
            };
            if class.0 != 0 {
                if let Err(e) = SetPriorityClass(me, class) {
                    tracing::warn!("priorità del programma non cambiata: {e}");
                }
            }
        }
    }
}

#[cfg(not(windows))]
mod sys {
    pub fn keep_awake(_on: bool) {}
    pub fn set_load(_load: super::ExportLoad) {}
}

/// File nella cartella dei fotogrammi: c'è finché l'esportazione non è finita del tutto.
pub const MANIFEST: &str = "esportazione.json";

/// Che cosa si stava esportando: serve a non mescolare fotogrammi di lavori diversi.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub fingerprint: String,
    pub total: usize,
    pub width: u32,
    pub height: u32,
    pub samples: u32,
    pub fps: f32,
    pub app_version: String,
}

impl Manifest {
    pub fn new(fingerprint: String, total: usize, settings: &RenderSettings, fps: f32) -> Self {
        Self {
            fingerprint,
            total,
            width: settings.width,
            height: settings.height,
            samples: settings.target_samples,
            fps,
            app_version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    pub fn write(&self, dir: &Path) -> anyhow::Result<()> {
        forge_image_io::write_atomic(&dir.join(MANIFEST), |tmp| {
            std::fs::write(tmp, serde_json::to_vec_pretty(self)?)?;
            Ok(())
        })
    }

    pub fn read(dir: &Path) -> Option<Self> {
        serde_json::from_slice(&std::fs::read(dir.join(MANIFEST)).ok()?).ok()
    }
}

/// Esportazione finita: non c'è più niente da riprendere. I fotogrammi restano.
pub fn forget(dir: &Path) {
    let _ = std::fs::remove_file(dir.join(MANIFEST));
}

/// FNV-1a: stabile tra una versione e l'altra del compilatore, al contrario dell'hash della
/// libreria standard.
fn fnv64(bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3))
}

/// Impronta del lavoro: progetto, impostazioni di render e `job` (intervallo, fotogrammi, vista).
/// La posizione del cursore della timeline non cambia i fotogrammi e non conta.
pub fn fingerprint(scene: &Scene, meta: &ProjectMeta, settings: &RenderSettings, job: &str) -> String {
    let file = forge_project::to_project_file(scene, meta, settings);
    let mut value = serde_json::to_value(&file).unwrap_or_default();
    value["timeline"]["time"] = serde_json::Value::Null;
    value["app_version"] = serde_json::Value::Null;
    let mut bytes = serde_json::to_vec(&value).unwrap_or_default();
    bytes.extend_from_slice(job.as_bytes());
    format!("{:016x}", fnv64(&bytes))
}

/// Quanti dei fotogrammi previsti sono già pronti, di seguito dal primo: si fermano al primo che
/// manca o che non ha le dimensioni giuste. Un PNG a metà non c'è mai, si scrive come `.tmp` e
/// si rinomina a file completo.
pub fn completed(files: &[PathBuf], width: u32, height: u32) -> usize {
    files
        .iter()
        .take_while(|f| image::image_dimensions(f).is_ok_and(|d| d == (width, height)))
        .count()
}

/// Esportazione interrotta trovata all'avvio.
#[derive(Clone, Debug)]
pub struct Interrupted {
    pub dir: PathBuf,
    pub done: usize,
    pub total: usize,
}

/// Dove si interviene alla scelta dell'utente.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Timeline,
    Sequence,
}

/// "Riprendere o ricominciare?", in attesa di risposta.
pub struct Prompt {
    pub found: Interrupted,
    pub target: Target,
}

/// L'esportazione interrotta di questa cartella, se è lo stesso lavoro.
pub fn interrupted(dir: &Path, fingerprint: &str, files: &[PathBuf]) -> Option<Interrupted> {
    let m = Manifest::read(dir).filter(|m| m.fingerprint == fingerprint && m.total == files.len())?;
    let names: Vec<PathBuf> = files.iter().map(|f| dir.join(f)).collect();
    let done = completed(&names, m.width, m.height);
    (done > 0).then(|| Interrupted {
        dir: dir.to_path_buf(),
        done,
        total: m.total,
    })
}

/// Cerca tra le cartelle `<prefisso>*` la più recente con questa esportazione interrotta.
pub fn find_interrupted(root: &Path, prefix: &str, fingerprint: &str, files: &[PathBuf]) -> Option<Interrupted> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(root)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir() && p.file_name().is_some_and(|n| n.to_string_lossy().starts_with(prefix)))
        .collect();
    dirs.sort();
    dirs.iter().rev().find_map(|d| interrupted(d, fingerprint, files))
}

impl ForgeApp {
    pub fn export_running(&self) -> bool {
        self.sequence.run.is_some() || self.timeline_ui.run.is_some()
    }

    /// Stato verso Windows e riposo della GPU: si legge a ogni giro, così cambiare la scelta a
    /// esportazione in corso vale dal fotogramma dopo.
    pub fn update_export_guard(&mut self) {
        let running = self.export_running();
        let load = self.prefs.video.load;
        self.export_guard.update(running, load);
        if running {
            let gpu = self.settings.backend == forge_render_core::BackendKind::Gpu;
            self.settings.pass_rest = if load == ExportLoad::Shared && gpu { SHARED_GPU_REST } else { 0.0 };
        }
    }

    /// Lascia nel log quando la finestra si nasconde e torna: distingue un arresto vero da
    /// un'interfaccia che semplicemente non si stava aggiornando.
    pub fn log_window_state(&mut self, ctx: &eframe::egui::Context) {
        let minimized = ctx.input(|i| i.viewport().minimized).unwrap_or(false);
        if minimized != self.window_minimized {
            self.window_minimized = minimized;
            if self.export_running() {
                tracing::info!(
                    "esportazione: finestra {}",
                    if minimized { "ridotta a icona" } else { "di nuovo visibile" }
                );
            }
        }
    }

    /// Finestra minuscola all'avvio (salvata da una versione precedente mentre era ridotta a
    /// icona): si riporta alla dimensione normale.
    pub fn repair_window_size(&mut self, ctx: &eframe::egui::Context) {
        if self.frames > 3 {
            return;
        }
        let small = ctx
            .input(|i| i.viewport().inner_rect)
            .is_some_and(|r| r.width() < 400.0 || r.height() < 300.0);
        if small && !self.window_minimized {
            tracing::warn!("finestra salvata troppo piccola: riportata alla dimensione normale");
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::InnerSize(eframe::egui::vec2(1600.0, 960.0)));
            ctx.send_viewport_cmd(eframe::egui::ViewportCommand::OuterPosition(eframe::egui::pos2(40.0, 40.0)));
        }
    }

    /// Interrompe quello che è in corso: i fotogrammi fatti restano e si possono riprendere.
    pub fn stop_exports(&mut self) {
        self.cancel_sequence();
        if let Some(r) = &mut self.timeline_ui.run {
            r.stop();
        }
    }

    /// Riporta la finestra davanti: serve quando si deve chiedere qualcosa all'utente.
    pub fn bring_to_front(ctx: &eframe::egui::Context) {
        ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Focus);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("forge-export-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn png(path: &Path, size: u32) {
        image::RgbaImage::new(size, size).save(path).unwrap();
    }

    fn files(n: usize) -> Vec<PathBuf> {
        (0..n).map(|i| PathBuf::from(format!("f{i}.png"))).collect()
    }

    fn manifest(total: usize) -> Manifest {
        let settings = RenderSettings {
            width: 4,
            height: 4,
            ..Default::default()
        };
        Manifest::new("abc".into(), total, &settings, 24.0)
    }

    #[test]
    fn counts_only_consecutive_frames_of_the_right_size() {
        let d = dir("conta");
        png(&d.join("f0.png"), 4);
        png(&d.join("f1.png"), 4);
        png(&d.join("f3.png"), 4);
        let list: Vec<PathBuf> = files(4).iter().map(|f| d.join(f)).collect();
        assert_eq!(completed(&list, 4, 4), 2, "il buco ferma il conteggio");
        assert_eq!(completed(&list, 8, 8), 0, "dimensioni diverse: lavoro diverso");
        // Un file rovinato non vale come fotogramma.
        std::fs::write(d.join("f1.png"), b"non un png").unwrap();
        assert_eq!(completed(&list, 4, 4), 1);
    }

    #[test]
    fn resumes_only_the_same_job() {
        let d = dir("stesso");
        png(&d.join("f0.png"), 4);
        manifest(3).write(&d).unwrap();
        let found = interrupted(&d, "abc", &files(3)).expect("stesso lavoro");
        assert_eq!((found.done, found.total), (1, 3));
        assert!(interrupted(&d, "xyz", &files(3)).is_none(), "impronta diversa");
        assert!(interrupted(&d, "abc", &files(5)).is_none(), "numero di fotogrammi diverso");
        forget(&d);
        assert!(interrupted(&d, "abc", &files(3)).is_none(), "esportazione finita");
    }

    #[test]
    fn finds_the_newest_folder_with_that_job() {
        let root = dir("radice");
        for (name, key) in [("animazione_001", "abc"), ("animazione_002", "abc"), ("animazione_003", "altro")] {
            let d = root.join(name);
            std::fs::create_dir_all(&d).unwrap();
            png(&d.join("f0.png"), 4);
            let mut m = manifest(2);
            m.fingerprint = key.into();
            m.write(&d).unwrap();
        }
        let found = find_interrupted(&root, "animazione_", "abc", &files(2)).expect("trovata");
        assert!(found.dir.ends_with("animazione_002"));
        assert!(find_interrupted(&root, "anteprima_", "abc", &files(2)).is_none());
    }

    #[test]
    fn the_playhead_does_not_change_the_fingerprint_but_the_settings_do() {
        let mut scene = Scene::default();
        let meta = ProjectMeta::default();
        let settings = RenderSettings::default();
        let a = fingerprint(&scene, &meta, &settings, "1");
        scene.timeline.time = 3.0;
        assert_eq!(a, fingerprint(&scene, &meta, &settings, "1"));
        assert_ne!(a, fingerprint(&scene, &meta, &settings, "2"), "lavoro diverso");
        let other = RenderSettings {
            target_samples: settings.target_samples + 1,
            ..settings.clone()
        };
        assert_ne!(a, fingerprint(&scene, &meta, &other, "1"), "campioni diversi");
    }

    #[test]
    fn guard_can_be_turned_on_and_off_more_than_once() {
        let mut g = Guard::default();
        g.update(true, ExportLoad::Shared);
        g.update(true, ExportLoad::Full);
        g.update(true, ExportLoad::Shared);
        g.update(false, ExportLoad::Shared);
        assert!(!g.active);
        assert_eq!(g.load, ExportLoad::Full);
    }
}
