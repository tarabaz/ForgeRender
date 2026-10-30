//! Diagnostica: informazioni di sistema, utilizzo CPU/RAM e log consultabili dalla UI.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

#[derive(Clone, Debug)]
pub struct SystemInfo {
    pub os: String,
    pub cpu_brand: String,
    pub physical_cores: Option<usize>,
    pub logical_threads: usize,
    pub total_ram_bytes: u64,
    pub available_ram_bytes: u64,
}

impl SystemInfo {
    pub fn detect() -> Self {
        let sys = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::nothing())
                .with_memory(MemoryRefreshKind::nothing().with_ram()),
        );
        let cpu_brand = sys
            .cpus()
            .first()
            .map(|c| c.brand().trim().to_string())
            .unwrap_or_else(|| "sconosciuta".into());
        Self {
            os: format!(
                "{} {}",
                System::name().unwrap_or_else(|| std::env::consts::OS.into()),
                System::os_version().unwrap_or_default()
            ),
            cpu_brand,
            physical_cores: System::physical_core_count(),
            logical_threads: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1),
            total_ram_bytes: sys.total_memory(),
            available_ram_bytes: sys.available_memory(),
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "{} | CPU: {} ({} core, {} thread) | RAM: {:.1} GB ({:.1} GB liberi)",
            self.os,
            self.cpu_brand,
            self.physical_cores.map(|c| c.to_string()).unwrap_or_else(|| "?".into()),
            self.logical_threads,
            gib(self.total_ram_bytes),
            gib(self.available_ram_bytes)
        )
    }
}

pub fn gib(b: u64) -> f64 {
    b as f64 / (1024.0 * 1024.0 * 1024.0)
}

/// Campionatore periodico dell'utilizzo di CPU e memoria del processo/sistema.
pub struct ResourceMonitor {
    sys: System,
    last: Instant,
    pub cpu_usage_percent: f32,
    pub used_ram_bytes: u64,
    pub process_ram_bytes: u64,
}

impl Default for ResourceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceMonitor {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_usage();
        Self {
            sys,
            last: Instant::now() - Duration::from_secs(10),
            cpu_usage_percent: 0.0,
            used_ram_bytes: 0,
            process_ram_bytes: 0,
        }
    }

    /// Aggiorna al massimo una volta al secondo (la misura della CPU richiede un intervallo).
    pub fn tick(&mut self) {
        if self.last.elapsed() < Duration::from_secs(1) {
            return;
        }
        self.last = Instant::now();
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.cpu_usage_percent = self.sys.global_cpu_usage();
        self.used_ram_bytes = self.sys.used_memory();
        if let Ok(pid) = sysinfo::get_current_pid() {
            self.sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
            self.process_ram_bytes = self.sys.process(pid).map(|p| p.memory()).unwrap_or(0);
        }
    }
}

#[derive(Clone, Debug)]
pub struct LogLine {
    pub level: tracing::Level,
    pub message: String,
    pub time: std::time::SystemTime,
}

/// Buffer circolare dei log, condiviso con l'interfaccia.
#[derive(Clone, Default)]
pub struct LogBuffer(Arc<Mutex<VecDeque<LogLine>>>);

impl LogBuffer {
    const CAPACITY: usize = 2000;

    pub fn lines(&self) -> Vec<LogLine> {
        self.0.lock().map(|l| l.iter().cloned().collect()).unwrap_or_default()
    }

    pub fn len(&self) -> usize {
        self.0.lock().map(|l| l.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn push(&self, line: LogLine) {
        if let Ok(mut l) = self.0.lock() {
            if l.len() >= Self::CAPACITY {
                l.pop_front();
            }
            l.push_back(line);
        }
    }
}

struct BufferLayer(LogBuffer);

struct MessageVisitor(String);

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name().starts_with("log.") {
            return; // metadati aggiunti dal ponte `log` → `tracing`
        }
        if field.name() == "message" {
            self.0 = format!("{value:?}");
        } else {
            self.0.push_str(&format!(" {}={value:?}", field.name()));
        }
    }
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name().starts_with("log.") {
            return;
        }
        if field.name() == "message" {
            self.0 = value.to_string();
        } else {
            self.0.push_str(&format!(" {}={value}", field.name()));
        }
    }
}

impl<S: tracing::Subscriber> Layer<S> for BufferLayer {
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let level = *event.metadata().level();
        if level > tracing::Level::INFO {
            return;
        }
        let mut v = MessageVisitor(String::new());
        event.record(&mut v);
        self.0.push(LogLine {
            level,
            message: v.0,
            time: std::time::SystemTime::now(),
        });
    }
}

const DEFAULT_FILTER: &str = "info,wgpu_core=warn,wgpu_hal=warn,naga=warn,eframe=warn,egui_wgpu=warn";

fn env_filter() -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER))
}

/// Inizializza il logging: console su stderr (filtrabile con `RUST_LOG`; stdout resta libero
/// per i risultati della CLI e per il protocollo MCP) e buffer per la UI.
pub fn init_logging() -> LogBuffer {
    init_logging_to(None)
}

/// Come [`init_logging`], scrivendo anche su `<dir>/forgerender.log`
/// (il file precedente viene conservato come `forgerender.prev.log`).
pub fn init_logging_to(dir: Option<&std::path::Path>) -> LogBuffer {
    let buf = LogBuffer::default();
    let file = dir.and_then(|d| {
        std::fs::create_dir_all(d).ok()?;
        let path = d.join("forgerender.log");
        if path.exists() {
            let _ = std::fs::rename(&path, d.join("forgerender.prev.log"));
        }
        std::fs::File::create(path).ok()
    });
    let file_layer = file.map(|f| {
        tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_target(false)
            .with_writer(Mutex::new(f))
            .with_filter(env_filter())
    });
    let _ = tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_writer(std::io::stderr)
                .with_filter(env_filter()),
        )
        .with(file_layer)
        .with(BufferLayer(buf.clone()))
        .try_init();
    buf
}

/// Cartella dei log: `logs` accanto all'eseguibile in modalità portable (file
/// `portable.txt` presente), altrimenti `%LOCALAPPDATA%\ForgeRender\logs` su
/// Windows o `$XDG_STATE_HOME`/`~/.local/state/forgerender/logs` altrove.
pub fn default_log_dir() -> Option<std::path::PathBuf> {
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    if exe_dir.join("portable.txt").exists() {
        return Some(exe_dir.join("logs"));
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        return Some(std::path::PathBuf::from(local).join("ForgeRender").join("logs"));
    }
    if let Some(state) = std::env::var_os("XDG_STATE_HOME") {
        return Some(std::path::PathBuf::from(state).join("forgerender").join("logs"));
    }
    std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/state/forgerender/logs"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_something() {
        let s = SystemInfo::detect();
        assert!(s.logical_threads >= 1);
        assert!(s.total_ram_bytes > 0);
        assert!(!s.summary().is_empty());
    }

    #[test]
    fn log_buffer_captures_events() {
        let buf = init_logging();
        tracing::info!("messaggio di prova {}", 42);
        assert!(buf.lines().iter().any(|l| l.message.contains("messaggio di prova 42")));
    }
}
