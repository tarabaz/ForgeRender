//! ForgeRender — applicazione desktop.
//!
//! Opzioni:
//!   forgerender [file.frproject | file.obj | file.stl]
//!     --safe-mode          interfaccia senza GPU (OpenGL/software), viewport 3D disattivata
//!     --render             avvia subito il rendering
//!     --screenshot F.png   salva uno screenshot della finestra ed esce (test automatici)
//!     --screenshot-after N attende N campioni prima dello screenshot

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod ui;
mod viewport;

use app::{ForgeApp, StartupOptions};
use eframe::egui;
use std::path::PathBuf;

fn parse_args() -> StartupOptions {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut o = StartupOptions::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--safe-mode" => o.safe_mode = true,
            "--render" => o.auto_render = true,
            "--screenshot" => {
                i += 1;
                o.screenshot = args.get(i).map(PathBuf::from);
            }
            "--screenshot-after" => {
                i += 1;
                o.screenshot_after_samples = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(0);
            }
            "--library" => o.library = true,
            "--tab" => {
                i += 1;
                o.tab = args.get(i).cloned();
            }
            "--select" => {
                i += 1;
                o.select = args.get(i).cloned();
            }
            "--gizmo" => {
                i += 1;
                o.gizmo = args.get(i).cloned();
            }
            "--camera-test" => {
                i += 1;
                o.camera_test = args.get(i).cloned();
            }
            other if !other.starts_with("--") => o.open = Some(PathBuf::from(other)),
            other => eprintln!("opzione sconosciuta ignorata: {other}"),
        }
        i += 1;
    }
    o
}

fn main() -> std::process::ExitCode {
    let log_dir = forge_telemetry::default_log_dir();
    let logs = forge_telemetry::init_logging_to(log_dir.as_deref());
    let startup = parse_args();
    let system = forge_telemetry::SystemInfo::detect();
    tracing::info!("ForgeRender {} — {}", env!("CARGO_PKG_VERSION"), system.summary());
    if let Some(d) = &log_dir {
        tracing::info!("file di log: {}", d.join("forgerender.log").display());
    }

    let native = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("ForgeRender")
            .with_inner_size([1600.0, 960.0])
            .with_min_inner_size([960.0, 600.0])
            .with_drag_and_drop(true),
        renderer: if startup.safe_mode {
            eframe::Renderer::Glow
        } else {
            eframe::Renderer::Wgpu
        },
        ..Default::default()
    };
    let st = startup.clone();
    let result = eframe::run_native(
        "ForgeRender",
        native,
        Box::new(move |cc| Ok(Box::new(ForgeApp::new(cc, logs, system, st)))),
    );
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) if !startup.safe_mode => {
            // Il ciclo eventi della finestra non può essere ricreato nello stesso processo:
            // si rilancia l'eseguibile in modalità sicura.
            tracing::error!("avvio con accelerazione GPU fallito: {e}. Riavvio in modalità sicura.");
            let relaunched = std::env::current_exe().ok().and_then(|exe| {
                std::process::Command::new(exe)
                    .args(std::env::args().skip(1))
                    .arg("--safe-mode")
                    .status()
                    .ok()
            });
            match relaunched {
                Some(s) if s.success() => std::process::ExitCode::SUCCESS,
                _ => std::process::ExitCode::FAILURE,
            }
        }
        Err(e) => {
            tracing::error!("avvio fallito anche in modalità sicura: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
