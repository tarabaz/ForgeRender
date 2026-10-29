//! Impostazioni dell'app (menu Impostazioni), salvate in `impostazioni.json`:
//! accanto al programma nella versione portable, altrimenti in `%LOCALAPPDATA%\ForgeRender`
//! (Windows) o `$XDG_CONFIG_HOME/forgerender` (`~/.config/forgerender`).
//!
//! Ogni campo comanda una funzione reale. File mancante: valori predefiniti. File non valido:
//! valori predefiniti, il file rovinato viene conservato come `impostazioni.json.non-valido`.
//! Campi nuovi o sconosciuti non rompono la lettura (`#[serde(default)]`).

use crate::i18n::tf;
use forge_library::store::ImportDisplacement;
use forge_render_core::{BackendKind, RenderSettings};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const FILE_NAME: &str = "impostazioni.json";

/// Modalità dell'interfaccia: Semplice (pannello guidato) o Avanzata (tutte le schede).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiMode {
    #[default]
    Simple,
    Advanced,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    /// Modalità dell'interfaccia (selettore in alto).
    pub ui_mode: UiMode,
    /// Lingua dell'interfaccia (`code` del file `lang/Lan_*.json`: "it", "en"…).
    pub language: String,
    /// Schermata di benvenuto all'avvio.
    pub show_splash: bool,
    // --- Libreria e import dei materiali ---
    /// Risoluzione proposta per i download (indice in `online::RESOLUTIONS`: 0 = 1K).
    pub download_resolution: usize,
    /// Sito proposto all'apertura della scheda Online: "ambientcg" o "polyhaven".
    pub download_provider: String,
    /// Displacement dei materiali importati.
    pub import_displacement: ImportDisplacement,
    /// Lato delle anteprime nella finestra Libreria (pixel dell'interfaccia, 64–512).
    pub library_card_size: u32,
    // --- Percorsi ---
    /// Cartella della libreria scelta dall'utente (`None` = predefinita).
    pub library_dir: Option<PathBuf>,
    // --- Rendering (nuovi progetti) ---
    pub new_backend: BackendKind,
    /// Campioni per pixel dei nuovi progetti.
    pub new_samples: u32,
    /// Thread CPU dei nuovi progetti (0 = automatico).
    pub new_threads: usize,
    /// Riavvio automatico del render quando cambia la scena.
    pub auto_restart: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        let r = RenderSettings::default();
        Self {
            ui_mode: UiMode::Simple,
            language: crate::i18n::SOURCE_CODE.into(),
            show_splash: true,
            download_resolution: 0,
            download_provider: "ambientcg".into(),
            import_displacement: ImportDisplacement::Off,
            library_card_size: 112,
            library_dir: None,
            new_backend: r.backend,
            new_samples: r.target_samples,
            new_threads: r.threads,
            auto_restart: true,
        }
    }
}

impl Preferences {
    /// Valori fuori intervallo riportati a quelli validi.
    pub fn sanitized(mut self) -> Self {
        if self.language.trim().is_empty() {
            self.language = crate::i18n::SOURCE_CODE.into();
        }
        let max_res = forge_library::online::RESOLUTIONS.len() - 1;
        self.download_resolution = self.download_resolution.min(max_res);
        if !["ambientcg", "polyhaven"].contains(&self.download_provider.as_str()) {
            self.download_provider = "ambientcg".into();
        }
        self.library_card_size = self.library_card_size.clamp(64, 512);
        self.new_samples = self.new_samples.min(1 << 20);
        self.new_threads = self.new_threads.min(1024);
        if self.library_dir.as_ref().is_some_and(|p| p.as_os_str().is_empty()) {
            self.library_dir = None;
        }
        self
    }

    /// Impostazioni di render di un nuovo progetto.
    pub fn new_project_settings(&self) -> RenderSettings {
        RenderSettings {
            backend: self.new_backend,
            target_samples: self.new_samples,
            threads: self.new_threads,
            ..Default::default()
        }
    }

    /// Applica le impostazioni che valgono per tutto il processo (libreria, import).
    pub fn apply_global(&self) {
        forge_library::paths::set_custom_root(self.library_dir.clone());
        forge_library::store::set_import_displacement(self.import_displacement);
    }

    pub fn load_from(path: &Path) -> (Self, Option<String>) {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (Self::default(), None),
            Err(e) => {
                return (
                    Self::default(),
                    Some(tf("impostazioni non lette ({e}): uso i valori predefiniti", &[format!("{}", e)])),
                )
            }
        };
        match serde_json::from_str::<Self>(&text) {
            Ok(p) => (p.sanitized(), None),
            Err(e) => {
                let bad = path.with_extension("json.non-valido");
                let _ = std::fs::copy(path, &bad);
                (
                    Self::default(),
                    Some(tf(
                        "impostazioni non valide ({e}): uso i valori predefiniti; il file è stato conservato come {}",
                        &[format!("{}", e), format!("{}", bad.display())],
                    )),
                )
            }
        }
    }

    /// Salvataggio atomico (file temporaneo + rinomina).
    pub fn save_to(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }
}

/// Percorso del file delle impostazioni.
pub fn file_path() -> PathBuf {
    if let Some(p) = std::env::var_os("FORGE_SETTINGS_FILE") {
        return PathBuf::from(p);
    }
    if let Some(exe_dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)) {
        if exe_dir.join("portable.txt").exists() {
            return exe_dir.join(FILE_NAME);
        }
    }
    let base = std::env::var_os("LOCALAPPDATA")
        .map(|p| PathBuf::from(p).join("ForgeRender"))
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(|p| PathBuf::from(p).join("forgerender")))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config/forgerender")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join(FILE_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("forge_prefs_{}_{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d.join(FILE_NAME)
    }

    #[test]
    fn round_trip_and_defaults() {
        let p = tmp("rt");
        let (d, err) = Preferences::load_from(&p);
        assert_eq!(d, Preferences::default());
        assert!(err.is_none(), "file mancante: nessun errore");
        let custom = Preferences {
            ui_mode: UiMode::Advanced,
            language: "en".into(),
            show_splash: false,
            download_resolution: 2,
            download_provider: "polyhaven".into(),
            import_displacement: ImportDisplacement::FromMaterialX,
            library_card_size: 256,
            library_dir: Some(PathBuf::from("/x/Libreria")),
            new_backend: BackendKind::Gpu,
            new_samples: 256,
            new_threads: 8,
            auto_restart: false,
        };
        custom.save_to(&p).unwrap();
        let (back, err) = Preferences::load_from(&p);
        assert!(err.is_none());
        assert_eq!(back, custom);
        let s = back.new_project_settings();
        assert_eq!((s.backend, s.target_samples, s.threads), (BackendKind::Gpu, 256, 8));
    }

    #[test]
    fn invalid_or_partial_files() {
        let p = tmp("bad");
        std::fs::write(&p, "{ non è json").unwrap();
        let (d, err) = Preferences::load_from(&p);
        assert_eq!(d, Preferences::default());
        assert!(err.unwrap().contains("non valide"));
        assert!(p.with_extension("json.non-valido").is_file(), "file rovinato conservato");
        // Campi mancanti o sconosciuti, valori fuori intervallo.
        std::fs::write(
            &p,
            r#"{"download_resolution": 99, "download_provider": "boh", "library_card_size": 5000, "campo_futuro": 1}"#,
        )
        .unwrap();
        let (d, err) = Preferences::load_from(&p);
        assert!(err.is_none());
        assert_eq!(d.library_card_size, 512);
        assert_eq!(d.download_resolution, forge_library::online::RESOLUTIONS.len() - 1);
        assert_eq!(d.download_provider, "ambientcg");
        assert!(d.auto_restart);
    }
}
