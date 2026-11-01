use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ToneMapping {
    /// Nessuna compressione: clamp a 1 e conversione sRGB.
    Standard,
    /// Curva filmica ACES (approssimazione di Narkowicz 2015).
    #[default]
    AcesFilmic,
}

impl ToneMapping {
    pub fn label(self) -> &'static str {
        match self {
            ToneMapping::Standard => "Standard (clamp)",
            ToneMapping::AcesFilmic => "ACES filmico",
        }
    }
}

/// Dispositivo su cui calcolare il rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BackendKind {
    #[default]
    Cpu,
    Gpu,
}

impl BackendKind {
    pub fn label(self) -> &'static str {
        match self {
            BackendKind::Cpu => "CPU (tutti i core)",
            BackendKind::Gpu => "GPU (compute shader)",
        }
    }
}

/// Quando applicare la riduzione del rumore.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DenoiseMode {
    Off,
    /// Solo sull'immagine finale (a fine render, in pausa o all'interruzione).
    #[default]
    Final,
    /// Anche durante il render: anteprima veloce ripulita a 8, 16, 32… campioni.
    Progressive,
}

impl DenoiseMode {
    pub fn label(self) -> &'static str {
        match self {
            DenoiseMode::Off => "Disattivata",
            DenoiseMode::Final => "Solo immagine finale",
            DenoiseMode::Progressive => "Anteprima veloce + finale",
        }
    }
}

/// Qualità del filtro (corrisponde alle modalità di Open Image Denoise).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DenoiseQuality {
    Fast,
    Balanced,
    #[default]
    High,
}

impl DenoiseQuality {
    pub fn label(self) -> &'static str {
        match self {
            DenoiseQuality::Fast => "Veloce",
            DenoiseQuality::Balanced => "Bilanciata",
            DenoiseQuality::High => "Alta",
        }
    }
}

/// Impostazioni della riduzione del rumore. Non invalidano l'accumulo: si
/// possono cambiare durante il render.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DenoiseSettings {
    pub mode: DenoiseMode,
    /// Esegue il filtro sulla GPU NVIDIA tramite CUDA (se supportata), altrimenti sulla CPU.
    pub use_cuda: bool,
    /// Qualità per l'immagine finale.
    pub quality: DenoiseQuality,
    /// Qualità per le anteprime durante il render.
    pub preview_quality: DenoiseQuality,
    /// Miscela tra immagine grezza (0) e ripulita (1): applicata in visualizzazione ed esportazione.
    pub strength: f32,
    /// Usa albedo e normali come guida: preserva spigoli, testi e texture.
    pub use_guides: bool,
    /// Campioni per pixel della prima anteprima ripulita (poi a ogni raddoppio).
    pub first_preview_samples: u32,
}

impl Default for DenoiseSettings {
    fn default() -> Self {
        Self {
            mode: DenoiseMode::Final,
            use_cuda: true,
            quality: DenoiseQuality::High,
            preview_quality: DenoiseQuality::Fast,
            strength: 1.0,
            use_guides: true,
            first_preview_samples: 8,
        }
    }
}

/// Preset del numero di campioni per pixel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SamplesPreset {
    Fixed(u32),
    Custom,
    Unlimited,
}

impl SamplesPreset {
    pub const FIXED: [u32; 7] = [16, 32, 64, 128, 256, 512, 1024];
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RenderSettings {
    pub width: u32,
    pub height: u32,
    /// Campioni per pixel obiettivo. 0 = progressivo senza limite.
    pub target_samples: u32,
    /// Numero massimo di rimbalzi del percorso.
    pub max_bounces: u32,
    /// Rimbalzo da cui inizia la Russian roulette.
    pub russian_roulette_start: u32,
    /// Limite di luminanza dei contributi indiretti (0 = disattivato).
    /// Riduce i "firefly" ma introduce bias: è disattivato per default.
    pub clamp_indirect: f32,
    pub seed: u64,
    /// Thread CPU (0 = automatico: thread logici − 1).
    pub threads: usize,
    pub tone_mapping: ToneMapping,
    /// Esposizione in stop (EV).
    pub exposure_ev: f32,
    /// Se falso lo sfondo è nero ma l'ambiente continua a illuminare.
    pub environment_visible: bool,
    pub backend: BackendKind,
    pub denoise: DenoiseSettings,
    /// Campionamento adattivo: errore relativo stimato sotto cui una zona dell'immagine
    /// smette di ricevere campioni (0 = spento, tutti i pixel fino a `target_samples`).
    /// Sulla scena di test 0.08 ≈ qualità di 64 campioni uniformi, 0.04 ≈ 256, 0.02 ≈ 1000.
    #[serde(default)]
    pub noise_threshold: f32,
    /// Campioni minimi per pixel prima di valutare il rumore.
    #[serde(default = "default_adaptive_min")]
    pub adaptive_min_samples: u32,
    /// Trasporto della luce in RGB (predefinito) o spettrale (lunghezze d'onda: dispersione nei
    /// vetri, colori più fedeli; più lento e con più rumore di colore).
    #[serde(default)]
    pub color_mode: ColorMode,
    /// Caustiche con le mappe di fotoni (vedi [`CausticSettings`]).
    #[serde(default)]
    pub caustics: CausticSettings,
}

fn default_adaptive_min() -> u32 {
    16
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            target_samples: 128,
            max_bounces: 12,
            russian_roulette_start: 3,
            clamp_indirect: 0.0,
            seed: 0x5EED,
            threads: 0,
            tone_mapping: ToneMapping::AcesFilmic,
            exposure_ev: 0.0,
            environment_visible: true,
            backend: BackendKind::Cpu,
            denoise: DenoiseSettings::default(),
            noise_threshold: 0.0,
            adaptive_min_samples: default_adaptive_min(),
            color_mode: ColorMode::Rgb,
            caustics: CausticSettings::default(),
        }
    }
}

impl RenderSettings {
    pub fn aspect(&self) -> f32 {
        self.width.max(1) as f32 / self.height.max(1) as f32
    }

    pub fn effective_threads(&self) -> usize {
        if self.threads > 0 {
            self.threads
        } else {
            std::thread::available_parallelism()
                .map(|n| n.get().saturating_sub(1).max(1))
                .unwrap_or(1)
        }
    }

    /// Valida i limiti per evitare allocazioni assurde.
    pub fn sanitized(&self) -> RenderSettings {
        let mut s = self.clone();
        s.width = s.width.clamp(16, 16384);
        s.height = s.height.clamp(16, 16384);
        s.max_bounces = s.max_bounces.clamp(1, 256);
        if !s.clamp_indirect.is_finite() || s.clamp_indirect < 0.0 {
            s.clamp_indirect = 0.0;
        }
        if !s.exposure_ev.is_finite() {
            s.exposure_ev = 0.0;
        }
        s.denoise.strength = if s.denoise.strength.is_finite() {
            s.denoise.strength.clamp(0.0, 1.0)
        } else {
            1.0
        };
        s.denoise.first_preview_samples = s.denoise.first_preview_samples.clamp(1, 4096);
        s.noise_threshold = if s.noise_threshold.is_finite() {
            s.noise_threshold.clamp(0.0, 1.0)
        } else {
            0.0
        };
        s.adaptive_min_samples = s.adaptive_min_samples.clamp(2, 65536);
        s
    }

    /// Le impostazioni che, se cambiano, invalidano l'accumulo.
    /// (Tone mapping ed esposizione si applicano in visualizzazione, e anche
    /// il numero di campioni obiettivo si può cambiare senza ripartire.)
    pub fn accumulation_key(&self) -> (u32, u32, u32, u32, u32, u64, bool, ColorMode, u64) {
        (
            self.width,
            self.height,
            self.max_bounces,
            self.russian_roulette_start,
            self.clamp_indirect.to_bits(),
            self.seed,
            self.environment_visible,
            self.color_mode,
            self.caustics.key(),
        )
    }
}

/// Come si trasporta la luce.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ColorMode {
    /// Tre canali (rosso, verde, blu): il render di sempre.
    #[default]
    Rgb,
    /// Quattro lunghezze d'onda per percorso (hero wavelength): dispersione e colori spettrali.
    Spectral,
}

impl ColorMode {
    pub fn label(self) -> &'static str {
        match self {
            ColorMode::Rgb => "RGB",
            ColorMode::Spectral => "Spettrale",
        }
    }
}

/// Caustiche con il photon mapping progressivo (Knaus e Zwicker 2011, "Progressive Photon
/// Mapping: A Probabilistic Approach"): a ogni passata si emettono fotoni dalle luci, che
/// attraversano vetri lisci (Snell, Fresnel, riflessione totale, assorbimento; per lunghezza d'onda
/// nel render spettrale) e si posano sulla prima superficie non speculare; la camera li raccoglie
/// entro un raggio che si restringe a ogni passata, così le caustiche diventano sempre più nitide e
/// la stima converge al valore esatto. Il path tracer smette di contare gli stessi percorsi.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CausticSettings {
    pub enabled: bool,
    /// Fotoni emessi per passata, in migliaia.
    pub photons_k: u32,
    /// Raggio di raccolta iniziale in millimetri (0 = automatico dalla dimensione della scena).
    pub radius_mm: f32,
}

impl Default for CausticSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            photons_k: 200,
            radius_mm: 0.0,
        }
    }
}

impl CausticSettings {
    /// Fotoni per passata (almeno mille).
    pub fn photons(&self) -> u32 {
        self.photons_k.clamp(1, 100_000) * 1000
    }

    /// Impronta per la chiave di accumulo (0 se spente).
    pub fn key(&self) -> u64 {
        if !self.enabled {
            return 0;
        }
        1 | (self.photons_k as u64) << 1 | (self.radius_mm.to_bits() as u64) << 32
    }

    /// Raggio di raccolta alla passata `k` (0, 1, 2…): r₀ · (k + 1)^(−1/6), cioè α = 2/3 nello
    /// schema di Knaus e Zwicker (varianza e distorsione scendono insieme, la stima converge).
    pub fn radius_at(&self, r0: f32, k: u32) -> f32 {
        r0 * ((k + 1) as f32).powf(-1.0 / 6.0)
    }
}
