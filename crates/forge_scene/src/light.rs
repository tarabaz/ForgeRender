use forge_math::Vec3;
use serde::{Deserialize, Serialize};

/// Tipi di sorgente luminosa. Unità:
/// - Point/Spot: intensità radiante (W/sr) = `intensity * color`;
/// - Directional: irradianza (W/m²) sulla superficie perpendicolare;
/// - Area: radianza emessa (W·sr⁻¹·m⁻²) dalla faccia frontale del rettangolo.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LightKind {
    Point,
    Spot { inner_angle_deg: f32, outer_angle_deg: f32 },
    Directional,
    Area { width: f32, height: f32 },
}

impl LightKind {
    pub fn label(&self) -> &'static str {
        match self {
            LightKind::Point => "Puntiforme",
            LightKind::Spot { .. } => "Spot",
            LightKind::Directional => "Direzionale",
            LightKind::Area { .. } => "Area (rettangolare)",
        }
    }
}

/// Sorgente luminosa. L'orientamento è dato da `position` → `target`
/// (per la luce direzionale conta solo la direzione).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Light {
    pub name: String,
    pub kind: LightKind,
    pub position: Vec3,
    pub target: Vec3,
    /// Colore lineare.
    pub color: [f32; 3],
    pub intensity: f32,
    pub enabled: bool,
    /// Se falso la luce illumina senza proiettare ombre (non fisico, utile in studio).
    pub cast_shadows: bool,
    /// Se vera, una luce ad area è visibile direttamente dalla camera.
    pub visible_to_camera: bool,
}

impl Light {
    pub fn direction(&self) -> Vec3 {
        (self.target - self.position).normalize_or(-Vec3::Y)
    }

    pub fn radiance_color(&self) -> Vec3 {
        Vec3::from_array(self.color) * self.intensity.max(0.0)
    }

    pub fn point(name: &str, position: Vec3, intensity: f32) -> Self {
        Self {
            name: name.into(),
            kind: LightKind::Point,
            position,
            target: Vec3::ZERO,
            color: [1.0; 3],
            intensity,
            enabled: true,
            cast_shadows: true,
            visible_to_camera: false,
        }
    }

    pub fn area(name: &str, position: Vec3, target: Vec3, width: f32, height: f32, radiance: f32) -> Self {
        Self {
            name: name.into(),
            kind: LightKind::Area { width, height },
            position,
            target,
            color: [1.0; 3],
            intensity: radiance,
            enabled: true,
            cast_shadows: true,
            visible_to_camera: false,
        }
    }
}

/// Converte una temperatura colore (K) in RGB lineare normalizzato
/// (approssimazione di Tanner Helland del corpo nero, 1000–40000 K).
pub fn kelvin_to_rgb(kelvin: f32) -> [f32; 3] {
    let t = kelvin.clamp(1000.0, 40000.0) / 100.0;
    let r = if t <= 66.0 {
        255.0
    } else {
        329.698_73 * (t - 60.0).powf(-0.133_204_76)
    };
    let g = if t <= 66.0 {
        99.470_8 * t.ln() - 161.119_57
    } else {
        288.122_17 * (t - 60.0).powf(-0.075_514_85)
    };
    let b = if t >= 66.0 {
        255.0
    } else if t <= 19.0 {
        0.0
    } else {
        138.517_73 * (t - 10.0).ln() - 305.044_8
    };
    let srgb = [r, g, b].map(|v: f32| (v / 255.0).clamp(0.0, 1.0));
    let lin = srgb.map(crate::material::srgb_to_linear);
    let max = lin.iter().cloned().fold(1e-6, f32::max);
    lin.map(|v| v / max)
}

/// Preset di illuminazione per fotografia di prodotto. `radius` è la dimensione
/// caratteristica della scena (raggio della bounding sphere), `center` il suo centro.
pub fn preset(name: &str, center: Vec3, radius: f32) -> Option<Vec<Light>> {
    let r = radius.max(1e-3);
    let at = |x: f32, y: f32, z: f32| center + Vec3::new(x, y, z) * r;
    // Radianza scelta per dare un'esposizione corretta ~indipendente dalla scala:
    // l'irradianza di un'area light a distanza d scala come (A/d²), e A/d² è costante qui.
    let lights = match name {
        "studio" => vec![
            Light::area("Softbox principale", at(-2.0, 3.0, 2.5), center, 2.5 * r, 2.5 * r, 3.5),
            Light::area("Riempimento", at(3.0, 1.5, 1.5), center, 2.0 * r, 2.0 * r, 0.8),
        ],
        "tre-punti" => vec![
            Light::area("Key", at(-2.5, 2.5, 2.5), center, 1.5 * r, 1.5 * r, 12.0),
            Light::area("Fill", at(3.0, 1.0, 2.0), center, 1.5 * r, 1.5 * r, 3.0),
            Light::area("Rim", at(0.5, 2.5, -3.0), center, 1.0 * r, 1.0 * r, 15.0),
        ],
        "softbox" => vec![Light::area("Softbox grande", at(0.0, 3.5, 1.0), center, 4.0 * r, 3.0 * r, 4.0)],
        "industriale" => (0..3)
            .map(|i| {
                let x = (i as f32 - 1.0) * 2.0;
                let mut l = Light::area(
                    &format!("Plafoniera {}", i + 1),
                    at(x, 3.5, 0.0),
                    at(x, 0.0, 0.0),
                    0.4 * r,
                    2.5 * r,
                    25.0,
                );
                l.color = kelvin_to_rgb(4500.0);
                l
            })
            .collect(),
        "cinematografica" => {
            let mut key = Light::area("Key calda", at(-3.0, 1.2, 1.0), center, 1.0 * r, 1.5 * r, 25.0);
            key.color = kelvin_to_rgb(3200.0);
            let mut rim = Light::area("Controluce fredda", at(2.0, 2.0, -3.0), center, 0.6 * r, 2.0 * r, 30.0);
            rim.color = kelvin_to_rgb(9000.0);
            vec![key, rim]
        }
        _ => return None,
    };
    Some(lights)
}

pub const PRESETS: [(&str, &str); 5] = [
    ("studio", "Studio neutro"),
    ("tre-punti", "Tre punti luce"),
    ("softbox", "Softbox"),
    ("industriale", "Illuminazione industriale"),
    ("cinematografica", "Cinematografica"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kelvin_is_plausible() {
        let warm = kelvin_to_rgb(2700.0);
        let cool = kelvin_to_rgb(9000.0);
        assert!(warm[0] > warm[2]);
        assert!(cool[2] > cool[0]);
        let d65 = kelvin_to_rgb(6500.0);
        assert!(d65.iter().all(|&c| c > 0.85));
    }

    #[test]
    fn all_presets_exist() {
        for (id, _) in PRESETS {
            assert!(!preset(id, Vec3::ZERO, 1.0).unwrap().is_empty());
        }
    }
}
