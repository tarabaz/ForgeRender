use forge_math::Vec3;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Illuminazione ambientale (luce proveniente dall'infinito).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Environment {
    /// Radianza uniforme in tutte le direzioni.
    Uniform { color: [f32; 3], strength: f32 },
    /// Cielo a gradiente: zenit → orizzonte → suolo.
    Gradient {
        zenith: [f32; 3],
        horizon: [f32; 3],
        ground: [f32; 3],
        strength: f32,
    },
    /// Mappa HDRI equirettangolare (.hdr Radiance o .exr), caricata dal renderer.
    Hdri {
        path: PathBuf,
        strength: f32,
        /// Rotazione attorno all'asse verticale (gradi).
        rotation_deg: f32,
    },
}

impl Default for Environment {
    fn default() -> Self {
        Environment::Gradient {
            zenith: [0.35, 0.42, 0.55],
            horizon: [0.8, 0.82, 0.85],
            ground: [0.25, 0.24, 0.23],
            strength: 0.45,
        }
    }
}

impl Environment {
    /// Radianza per una direzione (normalizzata) in spazio mondo, asse Y verso l'alto.
    pub fn radiance(&self, dir: Vec3) -> Vec3 {
        match self {
            Environment::Uniform { color, strength } => Vec3::from_array(*color) * *strength,
            Environment::Gradient {
                zenith,
                horizon,
                ground,
                strength,
            } => {
                let y = dir.y.clamp(-1.0, 1.0);
                let c = if y >= 0.0 {
                    let t = y.powf(0.6);
                    Vec3::from_array(*horizon).lerp(Vec3::from_array(*zenith), t)
                } else {
                    let t = (-y).powf(0.4);
                    Vec3::from_array(*horizon).lerp(Vec3::from_array(*ground), t)
                };
                c * *strength
            }
            // La mappa è campionata dal renderer (vedi `forge_render_core::EnvironmentMap`).
            Environment::Hdri { .. } => Vec3::ZERO,
        }
    }
}
