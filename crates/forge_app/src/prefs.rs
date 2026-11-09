//! Impostazioni dell'app (menu Impostazioni), salvate in `impostazioni.json`:
//! accanto al programma nella versione portable, altrimenti in `%LOCALAPPDATA%\ForgeRender`
//! (Windows) o `$XDG_CONFIG_HOME/forgerender` (`~/.config/forgerender`).
//!
//! Ogni campo comanda una funzione reale. File mancante: valori predefiniti. File non valido:
//! valori predefiniti, il file rovinato viene conservato come `impostazioni.json.non-valido`.
//! Campi nuovi o sconosciuti non rompono la lettura (`#[serde(default)]`).

use crate::i18n::{tf, tk};
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
    /// Finestra di scelta della modalità (Semplice o Avanzata) all'avvio.
    pub ask_ui_mode: bool,
    // --- Libreria e import dei materiali ---
    /// Risoluzione proposta per i download (indice in `online::RESOLUTIONS`: 0 = 1K).
    pub download_resolution: usize,
    /// Sito proposto all'apertura della scheda Online: "ambientcg" o "polyhaven".
    pub download_provider: String,
    /// Displacement dei materiali importati.
    pub import_displacement: ImportDisplacement,
    /// Lato delle anteprime nella finestra Libreria (pixel dell'interfaccia, 64–512).
    pub library_card_size: u32,
    /// Campioni per pixel delle anteprime dei materiali (4–4096).
    pub preview_samples: u32,
    /// Dimensione apparente dell'oggetto delle anteprime (cm, 0 = reale, circa 9 cm).
    pub preview_object_cm: u32,
    /// Denoiser (Open Image Denoise) sulle anteprime dei materiali.
    pub preview_denoise: bool,
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
    /// Ultimi progetti aperti o salvati (il più recente per primo, al massimo [`MAX_RECENT`]).
    pub recent_projects: Vec<PathBuf>,
    /// Timeline in una finestra a parte invece che in basso.
    pub timeline_detached: bool,
    /// Formato e qualità del video (l'ultima configurazione).
    pub video: crate::video::VideoSettings,
    /// Contatori scelti per la barra di stato, al massimo [`MAX_STATUS_ITEMS`].
    #[serde(deserialize_with = "known_status_items")]
    pub status_items: Vec<StatusItem>,
    /// Sfondo della viewport: uno dei colori pronti oppure Personalizzato. Solo la viewport e
    /// l'anteprima STEP: il render finale non cambia. Il colore da usare lo dà
    /// [`Preferences::viewport_background_color`].
    pub viewport_background_mode: ViewportBackground,
    /// Colore personalizzato (sRGB a 8 bit), conservato anche mentre è scelto un colore pronto.
    /// `None` finché non si è mai scelto Personalizzato.
    pub viewport_background_custom: Option<[u8; 3]>,
    /// Colore unico delle versioni 0.51–0.52: letto una volta e convertito in modalità e colore
    /// personalizzato da [`Preferences::sanitized`], non si scrive più.
    #[serde(rename = "viewport_background", skip_serializing)]
    pub legacy_viewport_background: Option<[u8; 3]>,
    /// Griglia metrica della viewport e dell'anteprima STEP.
    pub viewport_grid: ViewportGrid,
}

/// Scelta dello sfondo della viewport.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewportBackground {
    #[default]
    Antracite,
    GrigioBlender,
    ScuroTecnico,
    GrigioChiaro,
    Personalizzato,
}

/// Unità con cui si mostra e si scrive la misura della cella della griglia. La scena è in metri.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GridUnit {
    Mm,
    #[default]
    Cm,
    M,
}

impl GridUnit {
    pub const ALL: [GridUnit; 3] = [GridUnit::Mm, GridUnit::Cm, GridUnit::M];
    /// Metri in un'unità.
    pub fn meters(self) -> f32 {
        match self {
            GridUnit::Mm => 0.001,
            GridUnit::Cm => 0.01,
            GridUnit::M => 1.0,
        }
    }
    pub fn symbol(self) -> &'static str {
        match self {
            GridUnit::Mm => "mm",
            GridUnit::Cm => "cm",
            GridUnit::M => "m",
        }
    }
}

/// Griglia metrica sul pavimento (y = 0) della viewport: celle di misura reale nel mondo.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ViewportGrid {
    pub show: bool,
    /// Unità con cui si mostra la cella.
    pub unit: GridUnit,
    /// Lato della cella in metri.
    pub cell_m: f32,
    /// Una linea principale ogni quante celle.
    pub major_every: u32,
    /// Quanto si vedono le linee (1 = normale).
    pub intensity: f32,
}

impl Default for ViewportGrid {
    fn default() -> Self {
        Self {
            show: true,
            unit: GridUnit::Cm,
            cell_m: 0.01,
            major_every: 10,
            intensity: 1.0,
        }
    }
}

/// Misure pronte della cella, in metri: 1, 5, 10, 20, 50, 100 mm, poi 50 cm e 1 m.
pub const GRID_CELLS_M: [f32; 8] = [0.001, 0.005, 0.01, 0.02, 0.05, 0.1, 0.5, 1.0];
/// Intervalli pronti delle linee principali.
pub const GRID_MAJOR_EVERY: [u32; 3] = [5, 10, 20];

impl ViewportGrid {
    pub fn sanitized(mut self) -> Self {
        self.cell_m = if self.cell_m.is_finite() {
            self.cell_m.clamp(1e-4, 100.0)
        } else {
            0.01
        };
        self.major_every = self.major_every.clamp(2, 100);
        self.intensity = if self.intensity.is_finite() {
            self.intensity.clamp(0.1, 3.0)
        } else {
            1.0
        };
        self
    }
}

/// Sfondo predefinito della viewport: Antracite, grigio leggermente freddo che tiene il
/// contrasto anche con oggetti neri o molto scuri.
pub const VIEWPORT_BACKGROUND_DEFAULT: [u8; 3] = [40, 44, 52];

/// Colori pronti per lo sfondo della viewport (scelta, nome da tradurre, sRGB). Il primo è il
/// predefinito.
pub const VIEWPORT_BACKGROUNDS: [(ViewportBackground, &str, [u8; 3]); 4] = [
    (ViewportBackground::Antracite, tk("Antracite"), VIEWPORT_BACKGROUND_DEFAULT),
    (ViewportBackground::GrigioBlender, tk("Grigio Blender-like"), [48, 52, 59]),
    (ViewportBackground::ScuroTecnico, tk("Scuro tecnico"), [34, 40, 49]),
    (ViewportBackground::GrigioChiaro, tk("Grigio chiaro"), [53, 58, 66]),
];

/// Il colore pronto uguale a `c`, `None` se non è nessuno di quelli.
pub fn viewport_background_preset(c: [u8; 3]) -> Option<ViewportBackground> {
    VIEWPORT_BACKGROUNDS.iter().find(|(_, _, v)| *v == c).map(|(m, _, _)| *m)
}

/// Contatori che si possono scegliere; Backend e processore restano sempre a destra.
pub const MAX_STATUS_ITEMS: usize = 9;

/// Un contatore della barra di stato.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatusItem {
    Elapsed,
    Remaining,
    Frames,
    ViewportFps,
    VideoFps,
    RenderSpeed,
    Cpu,
    Gpu,
    Vram,
    RamSystem,
    AveragePass,
    LastPass,
    Resolution,
    Denoiser,
}

impl StatusItem {
    pub const ALL: [StatusItem; 14] = [
        StatusItem::Elapsed,
        StatusItem::Remaining,
        StatusItem::Frames,
        StatusItem::ViewportFps,
        StatusItem::VideoFps,
        StatusItem::RenderSpeed,
        StatusItem::Cpu,
        StatusItem::Gpu,
        StatusItem::Vram,
        StatusItem::RamSystem,
        StatusItem::AveragePass,
        StatusItem::LastPass,
        StatusItem::Resolution,
        StatusItem::Denoiser,
    ];

    pub fn defaults() -> Vec<StatusItem> {
        vec![
            StatusItem::Elapsed,
            StatusItem::Remaining,
            StatusItem::RenderSpeed,
            StatusItem::Cpu,
            StatusItem::Gpu,
            StatusItem::Vram,
        ]
    }
}

/// Le voci che non esistono più si saltano: una voce tolta dal programma non deve far perdere
/// tutte le impostazioni.
fn known_status_items<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<StatusItem>, D::Error> {
    let items = Vec::<serde_json::Value>::deserialize(d)?;
    Ok(items.into_iter().filter_map(|v| serde_json::from_value(v).ok()).collect())
}

/// Elenco utilizzabile: niente doppioni e non oltre il limite, anche con un file scritto a mano.
pub fn clean_status_items(items: &mut Vec<StatusItem>) {
    let mut seen = Vec::new();
    items.retain(|i| {
        let new = !seen.contains(i);
        seen.push(*i);
        new
    });
    items.truncate(MAX_STATUS_ITEMS);
}

/// Cartella predefinita di quello che si salva o si esporta: `Rendering` accanto al programma, o
/// in Documenti\ForgeRender se lì non si può scrivere (programma installato). La crea se manca.
pub fn rendering_dir() -> PathBuf {
    if let Some(exe_dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)) {
        let dir = exe_dir.join("Rendering");
        let probe = dir.join(".scrivibile");
        if std::fs::create_dir_all(&dir).is_ok() && std::fs::write(&probe, b"").is_ok() {
            let _ = std::fs::remove_file(&probe);
            return dir;
        }
    }
    let dir = forge_library::paths::default_root()
        .parent()
        .map(|p| p.join("Rendering"))
        .unwrap_or_else(|| PathBuf::from("Rendering"));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Prima cartella libera `<nome>_001`, `<nome>_002`... dentro la cartella Rendering (creata).
pub fn new_rendering_subdir(name: &str) -> PathBuf {
    let root = rendering_dir();
    let mut n = 1;
    loop {
        let dir = root.join(format!("{name}_{n:03}"));
        if !dir.exists() {
            let _ = std::fs::create_dir_all(&dir);
            return dir;
        }
        n += 1;
    }
}

/// Quanti progetti tiene File → Progetti recenti.
pub const MAX_RECENT: usize = 10;

impl Preferences {
    /// Mette `path` in cima ai recenti (senza doppioni). Vero se l'elenco è cambiato.
    pub fn push_recent(&mut self, path: &Path) -> bool {
        let before = self.recent_projects.clone();
        self.recent_projects.retain(|p| p != path);
        self.recent_projects.insert(0, path.to_path_buf());
        self.recent_projects.truncate(MAX_RECENT);
        self.recent_projects != before
    }
}

impl Default for Preferences {
    fn default() -> Self {
        let r = RenderSettings::default();
        Self {
            ui_mode: UiMode::Simple,
            language: crate::i18n::SOURCE_CODE.into(),
            show_splash: true,
            ask_ui_mode: true,
            download_resolution: 0,
            download_provider: "ambientcg".into(),
            import_displacement: ImportDisplacement::Off,
            library_card_size: 112,
            preview_samples: 64,
            preview_object_cm: 30,
            preview_denoise: true,
            library_dir: None,
            new_backend: BackendKind::Gpu,
            new_samples: r.target_samples,
            new_threads: r.threads,
            auto_restart: true,
            recent_projects: Vec::new(),
            timeline_detached: false,
            video: Default::default(),
            status_items: StatusItem::defaults(),
            viewport_background_mode: ViewportBackground::Antracite,
            viewport_background_custom: None,
            legacy_viewport_background: None,
            viewport_grid: ViewportGrid::default(),
        }
    }
}

impl Preferences {
    /// Colore dello sfondo della viewport (sRGB a 8 bit): l'unica sorgente per viewport e
    /// anteprima STEP.
    pub fn viewport_background_color(&self) -> [u8; 3] {
        match self.viewport_background_mode {
            ViewportBackground::Personalizzato => self.viewport_background_custom.unwrap_or(VIEWPORT_BACKGROUND_DEFAULT),
            mode => VIEWPORT_BACKGROUNDS
                .iter()
                .find(|(m, _, _)| *m == mode)
                .map_or(VIEWPORT_BACKGROUND_DEFAULT, |(_, _, c)| *c),
        }
    }

    /// Sceglie Personalizzato con un clic: tiene il colore personalizzato già scelto; la prima
    /// volta parte dal colore che si vede adesso.
    pub fn choose_custom_background(&mut self) {
        if self.viewport_background_custom.is_none() {
            self.viewport_background_custom = Some(self.viewport_background_color());
        }
        self.viewport_background_mode = ViewportBackground::Personalizzato;
    }

    /// Colore personalizzato cambiato dal selettore: diventa anche la scelta attiva.
    pub fn set_custom_background(&mut self, c: [u8; 3]) {
        self.viewport_background_custom = Some(c);
        self.viewport_background_mode = ViewportBackground::Personalizzato;
    }
}

impl Preferences {
    /// Valori fuori intervallo riportati a quelli validi.
    pub fn sanitized(mut self) -> Self {
        clean_status_items(&mut self.status_items);
        if self.language.trim().is_empty() {
            self.language = crate::i18n::SOURCE_CODE.into();
        }
        let max_res = forge_library::online::RESOLUTIONS.len() - 1;
        self.download_resolution = self.download_resolution.min(max_res);
        if !["ambientcg", "polyhaven"].contains(&self.download_provider.as_str()) {
            self.download_provider = "ambientcg".into();
        }
        self.library_card_size = self.library_card_size.clamp(64, 512);
        self.preview_samples = self.preview_samples.clamp(4, 4096);
        self.preview_object_cm = self.preview_object_cm.min(1000);
        self.new_samples = self.new_samples.min(1 << 20);
        self.new_threads = self.new_threads.min(1024);
        if self.library_dir.as_ref().is_some_and(|p| p.as_os_str().is_empty()) {
            self.library_dir = None;
        }
        // File delle 0.51–0.52: un colore solo. Se è uno dei pronti diventa quella scelta,
        // altrimenti Personalizzato con quel colore.
        if let Some(c) = self.legacy_viewport_background.take() {
            match viewport_background_preset(c) {
                Some(mode) => self.viewport_background_mode = mode,
                None => {
                    self.viewport_background_mode = ViewportBackground::Personalizzato;
                    self.viewport_background_custom = Some(c);
                }
            }
        }
        self.viewport_grid = self.viewport_grid.sanitized();
        self
    }

    /// Impostazioni di render di un nuovo progetto.
    pub fn new_project_settings(&self) -> RenderSettings {
        RenderSettings {
            backend: self.new_backend,
            target_samples: self.new_samples,
            threads: self.new_threads,
            // Arresto a soglia di rumore 0,03 (come la modalità Semplice); denoiser al 100 %.
            noise_threshold: 0.03,
            ..Default::default()
        }
    }

    /// Applica le impostazioni che valgono per tutto il processo (libreria, import).
    pub fn apply_global(&self) {
        forge_library::paths::set_custom_root(self.library_dir.clone());
        forge_library::store::set_import_displacement(self.import_displacement);
        crate::ui::central::set_preview_samples(self.preview_samples);
        crate::ui::central::set_preview_object_cm(self.preview_object_cm);
        crate::ui::central::set_preview_denoise(self.preview_denoise);
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
    fn removed_status_item_keeps_the_other_settings() {
        let p = tmp("voce-tolta");
        std::fs::write(&p, r#"{"auto_restart": false, "status_items": ["Cpu", "RamProcess", "Gpu"]}"#).unwrap();
        let (loaded, err) = Preferences::load_from(&p);
        assert!(err.is_none(), "{err:?}");
        assert!(!loaded.auto_restart);
        assert_eq!(loaded.status_items, vec![StatusItem::Cpu, StatusItem::Gpu]);
    }

    #[test]
    fn round_trip_and_defaults() {
        let p = tmp("rt");
        let (d, err) = Preferences::load_from(&p);
        assert_eq!(d, Preferences::default());
        assert!(err.is_none(), "file mancante: nessun errore");
        let custom = Preferences {
            ui_mode: UiMode::Advanced,
            status_items: vec![StatusItem::Frames, StatusItem::ViewportFps],
            video: crate::video::VideoSettings {
                crf: 20,
                ..Default::default()
            },
            language: "en".into(),
            show_splash: false,
            ask_ui_mode: false,
            download_resolution: 2,
            download_provider: "polyhaven".into(),
            import_displacement: ImportDisplacement::FromMaterialX,
            library_card_size: 256,
            preview_samples: 512,
            preview_object_cm: 100,
            preview_denoise: false,
            library_dir: Some(PathBuf::from("/x/Libreria")),
            new_backend: BackendKind::Gpu,
            new_samples: 256,
            new_threads: 8,
            auto_restart: false,
            recent_projects: vec![PathBuf::from("/x/a.frproject")],
            timeline_detached: true,
            viewport_background_mode: ViewportBackground::ScuroTecnico,
            viewport_background_custom: Some([12, 200, 99]),
            legacy_viewport_background: None,
            viewport_grid: ViewportGrid {
                show: false,
                unit: GridUnit::Mm,
                cell_m: 0.005,
                major_every: 20,
                intensity: 1.5,
            },
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
            r#"{"download_resolution": 99, "download_provider": "boh", "library_card_size": 5000, "preview_samples": 0, "preview_resolution": 300, "campo_futuro": 1}"#,
        )
        .unwrap();
        let (d, err) = Preferences::load_from(&p);
        assert!(err.is_none());
        assert_eq!(d.library_card_size, 512);
        assert_eq!(d.preview_samples, 4);
        // Preferenze di versioni precedenti alla 0.15.0: dimensione delle anteprime predefinita.
        assert_eq!(d.preview_object_cm, 30);
        assert_eq!(d.download_resolution, forge_library::online::RESOLUTIONS.len() - 1);
        assert_eq!(d.download_provider, "ambientcg");
        assert!(d.auto_restart);
    }

    /// Sfondo della viewport: Antracite (40, 44, 52) per le installazioni nuove e per i file
    /// scritti prima che esistesse; un preset scelto o un colore personalizzato restano dopo il
    /// riavvio; i preset hanno i valori chiesti e si riconoscono per nome.
    #[test]
    fn viewport_background_default_presets_and_persistence() {
        assert_eq!(Preferences::default().viewport_background_color(), [40, 44, 52]);
        let p = tmp("sfondo");
        std::fs::write(&p, r#"{"auto_restart": false}"#).unwrap();
        let (old, err) = Preferences::load_from(&p);
        assert!(err.is_none());
        assert_eq!(
            old.viewport_background_color(),
            VIEWPORT_BACKGROUND_DEFAULT,
            "file di prima: Antracite"
        );
        assert_eq!(
            VIEWPORT_BACKGROUNDS.map(|(_, _, c)| crate::ui::color_picker::to_hex(c)),
            ["#282C34", "#30343B", "#222831", "#353A42"]
        );
        for (mode, name, c) in VIEWPORT_BACKGROUNDS {
            assert_eq!(viewport_background_preset(c), Some(mode));
            let chosen = Preferences {
                viewport_background_mode: mode,
                ..Default::default()
            };
            chosen.save_to(&p).unwrap();
            assert_eq!(Preferences::load_from(&p).0.viewport_background_color(), c, "{name}");
        }
        let custom = [200, 10, 77];
        assert_eq!(viewport_background_preset(custom), None, "personalizzato");
        let mut chosen = Preferences::default();
        chosen.set_custom_background(custom);
        chosen.save_to(&p).unwrap();
        assert_eq!(Preferences::load_from(&p).0.viewport_background_color(), custom);
    }

    /// Le impostazioni scritte dalle 0.51–0.52 (un colore solo) diventano la scelta giusta.
    #[test]
    fn viewport_background_from_older_files() {
        let p = tmp("sfondo-vecchio");
        std::fs::write(&p, r#"{"viewport_background": [53, 58, 66]}"#).unwrap();
        let (a, _) = Preferences::load_from(&p);
        assert_eq!(a.viewport_background_mode, ViewportBackground::GrigioChiaro);
        assert_eq!(a.viewport_background_custom, None);
        std::fs::write(&p, r#"{"viewport_background": [1, 2, 3]}"#).unwrap();
        let (b, _) = Preferences::load_from(&p);
        assert_eq!(b.viewport_background_mode, ViewportBackground::Personalizzato);
        assert_eq!(b.viewport_background_color(), [1, 2, 3]);
        b.save_to(&p).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(!text.contains("\"viewport_background\""), "il campo vecchio non si riscrive");
        assert_eq!(Preferences::load_from(&p).0.viewport_background_color(), [1, 2, 3]);
    }

    /// Personalizzato si sceglie con un solo clic e il suo colore resta passando dai preset:
    /// preset A → Personalizzato (senza toccare il colore) → preset B → Personalizzato →
    /// colore cambiato → preset A → Personalizzato.
    #[test]
    fn custom_background_is_one_click_and_kept() {
        let mut p = Preferences {
            viewport_background_mode: ViewportBackground::GrigioBlender,
            ..Default::default()
        };
        p.choose_custom_background();
        assert_eq!(p.viewport_background_mode, ViewportBackground::Personalizzato);
        assert_eq!(p.viewport_background_color(), [48, 52, 59], "la prima volta parte dal colore visto");
        p.viewport_background_mode = ViewportBackground::GrigioChiaro;
        assert_eq!(p.viewport_background_color(), [53, 58, 66]);
        p.choose_custom_background();
        assert_eq!(p.viewport_background_mode, ViewportBackground::Personalizzato);
        assert_eq!(p.viewport_background_color(), [48, 52, 59], "colore personalizzato conservato");
        p.set_custom_background([90, 20, 30]);
        p.viewport_background_mode = ViewportBackground::GrigioBlender;
        assert_eq!(p.viewport_background_color(), [48, 52, 59]);
        p.choose_custom_background();
        assert_eq!(p.viewport_background_color(), [90, 20, 30]);
        // Anche se il colore personalizzato è uguale a un preset, la scelta resta Personalizzato.
        p.set_custom_background([40, 44, 52]);
        assert_eq!(p.viewport_background_mode, ViewportBackground::Personalizzato);
    }

    /// Griglia: valori predefiniti, salvataggio e valori fuori intervallo.
    #[test]
    fn viewport_grid_defaults_persistence_and_limits() {
        let g = ViewportGrid::default();
        assert!(g.show);
        assert_eq!((g.unit, g.cell_m, g.major_every), (GridUnit::Cm, 0.01, 10));
        let p = tmp("griglia");
        std::fs::write(
            &p,
            r#"{"viewport_grid": {"cell_m": -3, "major_every": 1000, "intensity": 99, "unit": "Mm"}}"#,
        )
        .unwrap();
        let (a, err) = Preferences::load_from(&p);
        assert!(err.is_none(), "{err:?}");
        assert_eq!(a.viewport_grid.unit, GridUnit::Mm);
        assert_eq!(a.viewport_grid.cell_m, 1e-4);
        assert_eq!(a.viewport_grid.major_every, 100);
        assert_eq!(a.viewport_grid.intensity, 3.0);
        assert!(a.viewport_grid.show, "campo mancante: predefinito");
        assert!((GridUnit::Mm.meters() * 10.0 - GridUnit::Cm.meters()).abs() < 1e-9);
    }

    #[test]
    fn recent_projects_keep_order_without_duplicates() {
        let mut p = Preferences::default();
        for i in 0..12 {
            p.push_recent(Path::new(&format!("/p/{i}.frproject")));
        }
        assert_eq!(p.recent_projects.len(), MAX_RECENT);
        assert_eq!(p.recent_projects[0], Path::new("/p/11.frproject"));
        assert!(p.push_recent(Path::new("/p/5.frproject")));
        assert_eq!(p.recent_projects[0], Path::new("/p/5.frproject"));
        assert_eq!(p.recent_projects.iter().filter(|x| x.ends_with("5.frproject")).count(), 1);
        assert!(!p.push_recent(Path::new("/p/5.frproject")), "già in cima: niente da salvare");
    }
}
