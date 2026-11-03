//! Nebbia volumetrica procedurale (stile ghiaccio secco).
//!
//! La nebbia nasce da un emettitore puntiforme, cade, si deposita sulle
//! superfici e si allarga, scorrendo attorno agli oggetti. Lo stato viene
//! calcolato per un "tempo simulato" (nessuna timeline): stessi parametri e
//! stesso seed danno sempre la stessa nube.

use forge_math::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FogVolume {
    pub name: String,
    pub enabled: bool,
    /// Seme del rumore e della turbolenza: cambia la forma, a parità di parametri.
    pub seed: u32,
    /// Punto da cui esce la nebbia (es. la ciotola con il ghiaccio secco).
    pub emitter: Vec3,
    /// Dimensioni della zona simulata (X, Y, Z in metri), centrata sull'emettitore
    /// in pianta; in altezza va da `size.y` sotto l'emettitore fino a poco sopra.
    pub size: Vec3,
    /// Coefficiente di estinzione della nebbia piena (1/m): più alto = più opaca.
    pub density: f32,
    /// Dimensione tipica dei dettagli (filamenti, sbuffi) in metri.
    pub scale: f32,
    /// Turbolenza 0–1: vortici e filamenti più marcati.
    pub turbulence: f32,
    /// Albedo di scattering (colore e intensità della luce diffusa, 0–1).
    pub albedo: [f32; 3],
    /// Anisotropia Henyey–Greenstein (−1 indietro, 0 uniforme, +1 in avanti).
    pub anisotropy: f32,
    /// Secondi di emissione simulati.
    pub sim_time: f32,
    /// Raggio della sorgente (es. la ciotola) in metri.
    pub source_radius: f32,
    /// Quantità di nebbia emessa (1 = una ciotola di ghiaccio secco in acqua calda).
    pub emission_rate: f32,
    /// Velocità con cui la nebbia si dissolve (1 = normale, 0 = non si dissolve).
    pub dissipation: f32,
    /// Tendenza della nebbia "vecchia" a scaldarsi e risalire a sbuffi (0 = resta a terra).
    pub rise: f32,
    /// La nebbia non attraversa gli oggetti della scena.
    pub collisions: bool,
    /// Risoluzione della griglia del dettaglio (celle sul lato più lungo).
    pub resolution: u32,
    /// Celle massime della simulazione sul lato più lungo (qualità della griglia):
    /// più alta = moto più fine (riccioli, colate sottili), calcolo molto più lento.
    pub sim_cells: u32,
    /// Spostamento della zona simulata rispetto alla posizione predefinita attorno
    /// all'emettitore (es. più in alto per lasciare spazio agli sbuffi).
    pub zone_offset: Vec3,
}

/// Livelli di qualità della griglia: (nome, celle della simulazione, risoluzione del dettaglio).
pub const FOG_QUALITY: [(&str, u32, u32); 5] = [
    ("Bozza", 48, 96),
    ("Media", 96, 160),
    ("Alta", 128, 200),
    ("Molto alta", 192, 300),
    ("Ultra", 256, 400),
];

impl Default for FogVolume {
    fn default() -> Self {
        Self {
            name: "Ghiaccio secco".into(),
            enabled: true,
            seed: 1,
            emitter: Vec3::new(0.0, 0.3, 0.0),
            size: Vec3::new(3.0, 1.0, 3.0),
            density: 60.0,
            scale: 0.25,
            turbulence: 0.6,
            albedo: [0.95; 3],
            anisotropy: 0.3,
            sim_time: 4.0,
            source_radius: 0.12,
            emission_rate: 1.0,
            dissipation: 1.0,
            rise: 0.4,
            collisions: true,
            resolution: 160,
            sim_cells: 128,
            zone_offset: Vec3::ZERO,
        }
    }
}

impl FogVolume {
    /// Zona simulata: in pianta centrata sull'emettitore; in altezza il 55% sotto
    /// l'emettitore (dove la nebbia cade) e il 45% sopra (sbuffi che risalgono).
    pub fn bounds(&self) -> (Vec3, Vec3) {
        let s = self.size.max(Vec3::splat(0.01));
        let c = self.emitter + self.zone_offset;
        let min = Vec3::new(c.x - s.x * 0.5, c.y - s.y * 0.55, c.z - s.z * 0.5);
        let max = Vec3::new(c.x + s.x * 0.5, c.y + s.y * 0.45, c.z + s.z * 0.5);
        (min, max)
    }

    /// Zona simulata estesa fino al suolo `ground` (la superficie sotto la zona) se
    /// questo è più in basso: il fondo della zona è chiuso e non deve fare da pavimento
    /// invisibile a mezz'aria.
    pub fn bounds_over(&self, ground: Option<f32>) -> (Vec3, Vec3) {
        let (mut min, max) = self.bounds();
        if let Some(g) = ground.filter(|g| g.is_finite() && *g < min.y) {
            min.y = g - 0.01 * self.size.y.max(0.01);
        }
        (min, max)
    }

    /// Parametri riportati in intervalli validi.
    pub fn sanitized(&self) -> FogVolume {
        let mut f = self.clone();
        let fin = |v: f32, d: f32| if v.is_finite() { v } else { d };
        f.emitter = if f.emitter.is_finite() { f.emitter } else { Vec3::ZERO };
        f.size = if f.size.is_finite() {
            f.size.clamp(Vec3::splat(0.05), Vec3::splat(100.0))
        } else {
            Vec3::ONE
        };
        f.density = fin(f.density, 30.0).clamp(0.0, 10_000.0);
        f.scale = fin(f.scale, 0.25).clamp(0.005, 100.0);
        f.turbulence = fin(f.turbulence, 0.6).clamp(0.0, 1.0);
        f.albedo = f.albedo.map(|a| fin(a, 0.9).clamp(0.0, 1.0));
        f.anisotropy = fin(f.anisotropy, 0.0).clamp(-0.95, 0.95);
        f.sim_time = fin(f.sim_time, 4.0).clamp(0.0, 120.0);
        f.source_radius = fin(f.source_radius, 0.12).clamp(0.005, 10.0);
        f.emission_rate = fin(f.emission_rate, 1.0).clamp(0.0, 100.0);
        f.dissipation = fin(f.dissipation, 1.0).clamp(0.0, 10.0);
        f.rise = fin(f.rise, 0.4).clamp(0.0, 1.0);
        f.resolution = f.resolution.clamp(16, 400);
        f.sim_cells = f.sim_cells.clamp(32, 256);
        f.zone_offset = if f.zone_offset.is_finite() { f.zone_offset } else { Vec3::ZERO };
        f
    }
}

/// Preset scenografici: impostano tutti i parametri del look insieme (non posizione,
/// seed, nome, accensione e collisioni). `hint` dice dove mettere l'emettitore.
pub struct FogPreset {
    pub name: &'static str,
    pub hint: &'static str,
    /// Zona, sorgente, quantità, tempo, densità, scala, turbolenza, risalita,
    /// dissolvenza, anisotropia, risoluzione.
    pub values: [f32; 13],
}

impl FogPreset {
    pub fn apply(&self, f: &mut FogVolume) {
        let v = self.values;
        f.size = Vec3::new(v[0], v[1], v[2]);
        f.source_radius = v[3];
        f.emission_rate = v[4];
        f.sim_time = v[5];
        f.density = v[6];
        f.scale = v[7];
        f.turbulence = v[8];
        f.rise = v[9];
        f.dissipation = v[10];
        f.anisotropy = v[11];
        f.resolution = v[12] as u32;
        f.albedo = [0.95; 3];
    }
}

pub const FOG_PRESETS: [FogPreset; 5] = [
    FogPreset {
        name: "Misurino che trabocca",
        hint: "Emettitore dentro il contenitore, 1 cm sotto il bordo: cupola a cavolfiore, colata lungo la parete, tappeto sul tavolo.",
        values: [0.6, 0.4, 0.6, 0.03, 0.08, 6.0, 250.0, 0.03, 0.6, 0.25, 1.0, 0.3, 160.0],
    },
    FogPreset {
        name: "Beuta: nuvola che sale",
        hint: "Emettitore all'imboccatura di un collo stretto: la nebbia esce a sbuffi e sale in una nuvola, qualche ricciolo ricade.",
        values: [0.7, 0.5, 0.7, 0.015, 0.06, 6.0, 250.0, 0.03, 0.7, 0.7, 1.0, 0.4, 160.0],
    },
    FogPreset {
        name: "Cocktail: colata e riccioli",
        hint: "Emettitore nel bicchiere poco sotto il bordo: poca nebbia che trabocca a filo, scende lungo lo stelo e si arriccia.",
        values: [0.45, 0.4, 0.45, 0.025, 0.05, 6.0, 300.0, 0.02, 0.9, 0.15, 0.8, 0.4, 200.0],
    },
    FogPreset {
        name: "Calderone / pozione",
        hint: "Emettitore nel calderone o in una ciotola grande: nebbia densa che ribolle sopra il bordo e scivola a terra.",
        values: [1.6, 1.0, 1.6, 0.12, 1.8, 10.0, 120.0, 0.08, 0.7, 0.3, 1.0, 0.5, 160.0],
    },
    FogPreset {
        name: "Tappeto basso da palco",
        hint: "Emettitore a 20–40 cm da terra (la bocca della macchina): tappeto basso e compatto che copre il pavimento.",
        values: [6.0, 1.2, 6.0, 0.35, 25.0, 12.0, 40.0, 0.4, 0.5, 0.05, 0.5, 0.3, 160.0],
    },
];

/// Foschia d'ambiente: aria leggermente velata in tutta la scena, che rende visibili i raggi
/// di luce (con le ombre degli oggetti, "lame" di luce) e allontana lo sfondo. Una sola per
/// scena; spenta di serie.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Haze {
    pub enabled: bool,
    /// Coefficiente di estinzione (1/m): 0,05 = velo leggero, 0,5 = foschia fitta.
    pub density: f32,
    /// Colore (albedo) della foschia, 0–1: bianco = aria neutra.
    pub color: [f32; 3],
    /// Anisotropia Henyey–Greenstein: alta = raggi luminosi soprattutto in controluce.
    pub anisotropy: f32,
    /// Altezza (m) in cui la foschia si dimezza salendo dal punto più basso della scena:
    /// 0 = uniforme, piccola = foschia a terra.
    pub falloff_height: f32,
    /// Quanto la foschia è irregolare, 0–1: 0 = uniforme, 1 = banchi e vuoti marcati.
    pub variation: f32,
    /// Dimensione tipica dei vortici e dei banchi (metri).
    pub swirl_scale: f32,
    /// Seme della forma dei vortici.
    pub seed: u32,
    /// Tipo di foschia: come la densità si distribuisce nello spazio.
    pub kind: HazeKind,
    /// Forza dei vortici (distorsione del dominio del rumore): 0 = macchie tonde, 3 = riccioli.
    pub warp: f32,
    /// Ottave del frattale (1–8): più ottave = dettagli sempre più fini.
    pub octaves: u32,
    /// Ruvidità del frattale (guadagno tra le ottave, 0,3–0,8): alta = dettagli fini marcati.
    pub roughness: f32,
    /// Qualità della griglia (1–4): celle più fitte, calcolo più lungo e più memoria.
    pub quality: u32,
}

/// Tipo di foschia d'ambiente.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum HazeKind {
    /// Aria velata con banchi e riccioli, che può diradarsi salendo (la "nebbia bassa").
    #[default]
    Ambient,
    /// Aria quasi limpida con ammassi densi qua e là, come nuvole di gas.
    GasClouds,
    /// Strato di pochi centimetri sul pavimento, tirato in filamenti come fumo tossico.
    ToxicFloor,
    /// Veli orizzontali sovrapposti a quote diverse, ondulati e bucati.
    Layers,
    /// Riccioli di vapore: vortici molto marcati (distorsione doppia), contrasto alto.
    Billows,
}

impl HazeKind {
    pub const ALL: [HazeKind; 5] = [
        HazeKind::Ambient,
        HazeKind::GasClouds,
        HazeKind::ToxicFloor,
        HazeKind::Layers,
        HazeKind::Billows,
    ];
}

impl Haze {
    /// Foschia accesa del tipo `kind` con le impostazioni di partenza adatte.
    pub fn of_kind(kind: HazeKind) -> Haze {
        let base = Haze {
            enabled: true,
            kind,
            ..Default::default()
        };
        match kind {
            HazeKind::Ambient => Haze {
                density: 0.9,
                variation: 0.7,
                swirl_scale: 0.5,
                anisotropy: 0.3,
                falloff_height: 0.25,
                ..base
            },
            HazeKind::GasClouds => Haze {
                density: 1.2,
                variation: 0.45,
                swirl_scale: 0.9,
                anisotropy: 0.4,
                warp: 1.4,
                octaves: 5,
                roughness: 0.55,
                ..base
            },
            HazeKind::ToxicFloor => Haze {
                density: 4.0,
                color: [0.8, 0.95, 0.7],
                variation: 0.85,
                swirl_scale: 0.35,
                anisotropy: 0.3,
                falloff_height: 0.06,
                warp: 1.0,
                ..base
            },
            HazeKind::Layers => Haze {
                density: 0.8,
                variation: 0.6,
                swirl_scale: 0.8,
                anisotropy: 0.5,
                falloff_height: 0.35,
                ..base
            },
            HazeKind::Billows => Haze {
                density: 0.7,
                variation: 0.9,
                swirl_scale: 0.7,
                anisotropy: 0.45,
                warp: 2.6,
                octaves: 6,
                roughness: 0.6,
                ..base
            },
        }
    }
}

impl Default for Haze {
    fn default() -> Self {
        Self {
            enabled: false,
            density: 0.15,
            color: [0.92, 0.93, 0.95],
            anisotropy: 0.6,
            falloff_height: 0.0,
            variation: 0.55,
            swirl_scale: 0.8,
            seed: 1,
            kind: HazeKind::Ambient,
            warp: 1.6,
            octaves: 4,
            roughness: 0.5,
            quality: 2,
        }
    }
}

impl Haze {
    /// Densità alla quota `y` sopra il punto più basso `floor`.
    pub fn density_at(&self, y: f32, floor: f32) -> f32 {
        if self.falloff_height <= 0.0 {
            return self.density.max(0.0);
        }
        self.density.max(0.0) * (-(y - floor).max(0.0) * std::f32::consts::LN_2 / self.falloff_height).exp()
    }
}
