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
        s
    }

    /// Le impostazioni che, se cambiano, invalidano l'accumulo.
    /// (Tone mapping ed esposizione si applicano in visualizzazione, e anche
    /// il numero di campioni obiettivo si può cambiare senza ripartire.)
    pub fn accumulation_key(&self) -> (u32, u32, u32, u32, u32, u64, bool) {
        (
            self.width,
            self.height,
            self.max_bounces,
            self.russian_roulette_start,
            self.clamp_indirect.to_bits(),
            self.seed,
            self.environment_visible,
        )
    }
}
