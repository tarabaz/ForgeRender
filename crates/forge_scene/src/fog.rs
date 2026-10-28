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
    /// La nebbia non attraversa gli oggetti della scena.
    pub collisions: bool,
    /// Risoluzione della griglia (celle sul lato più lungo).
    pub resolution: u32,
}

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
            collisions: true,
            resolution: 160,
        }
    }
}

impl FogVolume {
    /// Zona simulata: in pianta centrata sull'emettitore; in altezza dall'emettitore
    /// meno `size.y` fino a un quarto di `size.y` sopra (la nebbia sale poco).
    pub fn bounds(&self) -> (Vec3, Vec3) {
        let s = self.size.max(Vec3::splat(0.01));
        let min = Vec3::new(self.emitter.x - s.x * 0.5, self.emitter.y - s.y, self.emitter.z - s.z * 0.5);
        let max = Vec3::new(self.emitter.x + s.x * 0.5, self.emitter.y + s.y * 0.25, self.emitter.z + s.z * 0.5);
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
        f.resolution = f.resolution.clamp(16, 400);
        f
    }
}
