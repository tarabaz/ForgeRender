//! ForgeRender — applicazione desktop.
//!
//! Opzioni:
//!   forgerender [file.frproject | file.obj | file.stl]
//!     --safe-mode          interfaccia senza GPU (OpenGL/software), viewport 3D disattivata
//!     --render             avvia subito il rendering
//!     --screenshot F.png   salva uno screenshot della finestra ed esce (test automatici)
//!     --screenshot-after N attende N campioni prima dello screenshot
//!     --modalita semplice|avanzata   modalità dell'interfaccia per questa sessione
//!     --mesh               viewport in vista Mesh (wireframe) all'avvio
//!     --integrazione-ai    apre la finestra Integrazione AI (collegamento degli assistenti)
//!     --timeline-demo      animazione di prova sul primo oggetto (screenshot)
//!     --prepara studio|chiaro|scuro|scenografico   "Prepara scena prodotto" all'avvio

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod commands;
mod contact;
mod demos;
mod export_job;
mod i18n;
mod mcp;
mod prefs;
mod presets;
mod sequence;
#[cfg(test)]
mod texts;
mod ui;
mod video;
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
            "--importa-blend" => {
                i += 1;
                o.import_blend = args.get(i).map(PathBuf::from);
            }
            "--settings" => o.settings = true,
            "--iso" => o.iso = true,
            "--mesh" => o.mesh = true,
            "--integrazione-ai" => o.ai_setup = true,
            "--timeline-demo" => o.timeline_demo = true,
            "--controllo-ia" | "--mcp" => o.mcp = true,
            "--prepara" => {
                i += 1;
                o.prepare = match args.get(i).map(String::as_str) {
                    Some("chiaro") => Some(presets::Look::Light),
                    Some("scuro") => Some(presets::Look::Dark),
                    Some("scenografico") => Some(presets::Look::Dramatic),
                    Some("studio") => Some(presets::Look::Studio),
                    _ => None,
                };
            }
            "--modalita" => {
                i += 1;
                o.ui_mode = match args.get(i).map(String::as_str) {
                    Some("semplice") => Some(prefs::UiMode::Simple),
                    Some("avanzata") => Some(prefs::UiMode::Advanced),
                    _ => None,
                };
            }
            "--tab" => {
                i += 1;
                o.tab = args.get(i).cloned();
            }
            "--select" => {
                i += 1;
                o.select = args.get(i).cloned();
            }
            "--light" => {
                i += 1;
                o.light = args.get(i).cloned();
            }
            "--fog" => {
                i += 1;
                o.fog = args.get(i).cloned();
            }
            "--aim" => {
                i += 1;
                o.aim = args.get(i).cloned();
            }
            "--gizmo" => {
                i += 1;
                o.gizmo = args.get(i).cloned();
            }
            "--camera-test" => {
                i += 1;
                o.camera_test = args.get(i).cloned();
            }
            "--sequenze" => o.sequences_window = true,
            "--importa-step" => {
                i += 1;
                o.step_import = args.get(i).map(PathBuf::from);
            }
            "--demo-prisma" => o.prism_demo = true,
            "--scelta-modalita" => o.mode_choice = true,
            "--sequenza" => {
                i += 1;
                o.sequence = args.get(i).map(PathBuf::from);
            }
            "--animazione" => o.animation = true,
            "--riprendi" => o.resume = true,
            "--sequenza-viste" => o.sequence_views = true,
            "--sequenza-passaggi" => o.sequence_passes = true,
            "--sequenza-passo" => {
                i += 1;
                o.sequence_step = args.get(i).and_then(|s| s.parse().ok());
            }
            "--sequenza-campioni" => {
                i += 1;
                o.sequence_samples = args.get(i).and_then(|s| s.parse().ok());
            }
            other if !other.starts_with("--") => o.open = Some(PathBuf::from(other)),
            other => eprintln!("opzione sconosciuta ignorata: {other}"),
        }
        i += 1;
    }
    o
}

/// Icona del programma (`assets/forgerender.ico`, la stessa delle risorse dell'eseguibile).
const ICON_ICO: &[u8] = include_bytes!("../../../assets/forgerender.ico");

/// L'immagine più grande del .ico (PNG o bitmap a 32 bit), come RGBA.
fn ico_largest_rgba(ico: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    let u16_at = |o: usize| Some(u16::from_le_bytes(ico.get(o..o + 2)?.try_into().ok()?));
    let u32_at = |o: usize| Some(u32::from_le_bytes(ico.get(o..o + 4)?.try_into().ok()?));
    let count = u16_at(4)? as usize;
    let side = |e: usize| ico.get(e).map(|&b| if b == 0 { 256 } else { b as u32 });
    let e = (0..count).map(|i| 6 + 16 * i).max_by_key(|&e| side(e).unwrap_or(0))?;
    let (size, offset) = (u32_at(e + 8)? as usize, u32_at(e + 12)? as usize);
    let data = ico.get(offset..offset + size)?;
    if data.starts_with(b"\x89PNG") {
        let img = image::load_from_memory_with_format(data, image::ImageFormat::Png).ok()?.to_rgba8();
        let (w, h) = img.dimensions();
        return Some((img.into_raw(), w, h));
    }
    // Bitmap a 32 bit (BGRA, dal basso in alto, altezza doppia per la maschera).
    let w = u32::from_le_bytes(data.get(4..8)?.try_into().ok()?) as usize;
    let h = (u32::from_le_bytes(data.get(8..12)?.try_into().ok()?) / 2) as usize;
    let header = u32::from_le_bytes(data.get(0..4)?.try_into().ok()?) as usize;
    if u16::from_le_bytes(data.get(14..16)?.try_into().ok()?) != 32 {
        return None;
    }
    let px = data.get(header..header + w * h * 4)?;
    let mut rgba = Vec::with_capacity(w * h * 4);
    for y in (0..h).rev() {
        for p in px[y * w * 4..(y + 1) * w * 4].chunks(4) {
            rgba.extend([p[2], p[1], p[0], p[3]]);
        }
    }
    Some((rgba, w as u32, h as u32))
}

/// Icona della finestra e della barra delle applicazioni: va data a parte, la risorsa
/// dell'eseguibile da sola non basta a winit.
fn window_icon() -> egui::IconData {
    match ico_largest_rgba(ICON_ICO) {
        Some((rgba, width, height)) => egui::IconData { rgba, width, height },
        None => egui::IconData::default(),
    }
}

fn main() -> std::process::ExitCode {
    let log_dir = forge_telemetry::default_log_dir();
    let logs = forge_telemetry::init_logging_to(log_dir.as_deref());
    let startup = parse_args();
    let system = forge_telemetry::SystemInfo::detect();
    tracing::info!("{} — {}", app::APP_TITLE, system.summary());
    if let Some(d) = &log_dir {
        tracing::info!("file di log: {}", d.join("forgerender.log").display());
    }

    let native = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(app::APP_TITLE)
            // Identificativo stabile (archivio delle impostazioni, app id su Wayland):
            // il titolo cambia a ogni build, le impostazioni salvate restano.
            .with_app_id("ForgeRender")
            .with_inner_size([1600.0, 960.0])
            .with_min_inner_size([960.0, 600.0])
            .with_drag_and_drop(true)
            .with_icon(window_icon()),
        renderer: if startup.safe_mode {
            eframe::Renderer::Glow
        } else {
            eframe::Renderer::Wgpu
        },
        ..Default::default()
    };
    let st = startup.clone();
    let result = eframe::run_native(
        app::APP_TITLE,
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

#[cfg(test)]
mod icon_tests {
    use super::*;

    /// L'icona del repository è un .ico valido con tutte le misure di Windows, e la finestra ne
    /// prende la più grande (256 px) con la trasparenza degli angoli.
    #[test]
    fn program_icon_is_complete() {
        let n = u16::from_le_bytes([ICON_ICO[4], ICON_ICO[5]]) as usize;
        let sides: Vec<u32> = (0..n)
            .map(|i| match ICON_ICO[6 + 16 * i] {
                0 => 256,
                b => b as u32,
            })
            .collect();
        for side in [16, 24, 32, 48, 256] {
            assert!(sides.contains(&side), "manca la misura {side}: {sides:?}");
        }
        let (rgba, w, h) = ico_largest_rgba(ICON_ICO).expect("immagine dell'icona");
        assert_eq!((w, h), (256, 256));
        assert_eq!(rgba.len(), 256 * 256 * 4);
        assert!(rgba[3] < 255, "angolo trasparente");
        assert!(rgba.chunks(4).any(|p| p[3] == 255), "logo opaco");
    }
}
