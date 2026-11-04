//! Fumo volumetrico: nubi di fumo vere (struttura 3D, riccioli, filamenti, bordi sfilacciati),
//! posizionate con un riquadro nella scena e generate da un campo di densità procedurale.
//!
//! Il riquadro (`center`, `size`) è la zona in cui il fumo esiste: spostarlo o ridimensionarlo
//! sposta e ridimensiona il fumo renderizzato. Dentro, la forma (`shape`) dà l'ingombro di
//! massima e il rumore frattale a dominio distorto i dettagli a più scale. Il fumo è fisico:
//! diffonde e assorbe la luce, fa ombra e la riceve (niente emissione: il fuoco è un'altra cosa).
//! Stessi parametri e stesso seme danno sempre la stessa nube.

use forge_math::Vec3;
use serde::{Deserialize, Serialize};

/// Forma di massima della nube.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SmokeShape {
    /// Nube sospesa: un ammasso tondeggiante che riempie il riquadro (ellissoide).
    #[default]
    Cloud,
    /// Pennacchio: esce stretto dal fondo del riquadro, sale allargandosi e si dissolve.
    Plume,
    /// Filo d'incenso: sottile e liscio in basso, poi si rompe in riccioli salendo.
    Wisp,
    /// Banco: riempie tutto il riquadro, con i bordi sfilacciati (fumo denso, stanza piena).
    Bank,
}

impl SmokeShape {
    pub const ALL: [SmokeShape; 4] = [SmokeShape::Cloud, SmokeShape::Plume, SmokeShape::Wisp, SmokeShape::Bank];
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SmokeCloud {
    /// Numero univoco nella scena.
    pub n: u32,
    pub name: String,
    pub enabled: bool,
    pub shape: SmokeShape,
    /// Centro del riquadro (m, spazio mondo).
    pub center: Vec3,
    /// Dimensioni del riquadro (m).
    pub size: Vec3,
    /// Densità del fumo pieno (1/m): 5 = fumo leggero, 50 = denso, 300 = quasi opaco.
    pub density: f32,
    /// Colore del fumo (albedo, 0–1): bianco = vapore, grigio = fumo, scuro = fuliggine.
    pub color: [f32; 3],
    /// Anisotropia Henyey–Greenstein: alta = brilla in controluce.
    pub anisotropy: f32,
    /// Grandezza dei riccioli più grandi (m).
    pub detail_scale: f32,
    /// Ottave del frattale (1–10).
    pub octaves: u32,
    /// Ruvidità del frattale (0,2–0,85): alta = dettagli fini marcati.
    pub roughness: f32,
    /// Forza dei vortici (distorsione del dominio, 0–8).
    pub turbulence: f32,
    /// Sbuffi "a cavolfiore" sul bordo, 0–1.
    pub billows: f32,
    /// Bordi sfilacciati in filamenti, 0–1.
    pub wisps: f32,
    /// Morbidezza dei bordi, 0–1: 0 = bordo netto, 1 = sfuma piano.
    pub softness: f32,
    /// Variazione della densità dentro la nube, 0–1.
    pub variation: f32,
    /// Pennacchio e incenso: quanto si allarga salendo (0 = colonna, 1 = cono largo come il riquadro).
    pub spread: f32,
    /// Pennacchio e incenso: quanto serpeggia salendo, 0–1.
    pub meander: f32,
    /// Pennacchio e incenso: quanto si dissolve salendo, 0–1 (0 = denso fino in cima).
    pub fade: f32,
    /// Quota da cui parte il fumo, sopra il fondo del riquadro (m, 0 = dal fondo).
    pub bottom_height: f32,
    /// Quota a cui si ferma il fumo, sopra il fondo del riquadro (m, 0 = fino in cima).
    pub top_height: f32,
    /// Il fumo non entra negli oggetti (niente fumo dentro i pezzi).
    pub collisions: bool,
    /// Qualità della griglia (1–4).
    pub quality: u32,
    pub seed: u32,
}

impl Default for SmokeCloud {
    fn default() -> Self {
        Self {
            n: 1,
            name: "Fumo".into(),
            enabled: true,
            shape: SmokeShape::Cloud,
            center: Vec3::new(0.0, 0.5, 0.0),
            size: Vec3::splat(0.6),
            density: 40.0,
            color: [0.8, 0.8, 0.8],
            anisotropy: 0.35,
            detail_scale: 0.15,
            octaves: 6,
            roughness: 0.55,
            turbulence: 1.2,
            billows: 0.6,
            wisps: 0.5,
            softness: 0.35,
            variation: 0.5,
            spread: 0.6,
            meander: 0.4,
            fade: 0.5,
            bottom_height: 0.0,
            top_height: 0.0,
            collisions: true,
            quality: 2,
            seed: 1,
        }
    }
}

/// Preset: punti di partenza modificabili. `hint` dice dove mettere il riquadro.
pub struct SmokePreset {
    pub name: &'static str,
    pub hint: &'static str,
    pub make: fn() -> SmokeCloud,
}

pub const SMOKE_PRESETS: [SmokePreset; 6] = [
    SmokePreset {
        name: "Nube sospesa",
        hint: "Un ammasso di fumo morbido a mezz'aria, con sbuffi e bordi sfilacciati",
        make: || SmokeCloud {
            name: "Nube sospesa".into(),
            ..Default::default()
        },
    },
    SmokePreset {
        name: "Incenso",
        hint: "Un filo sottile che sale liscio e poi si rompe in riccioli: il fondo del riquadro va sulla punta del bastoncino",
        make: || SmokeCloud {
            name: "Incenso".into(),
            shape: SmokeShape::Wisp,
            size: Vec3::new(0.25, 0.6, 0.25),
            density: 220.0,
            color: [0.78, 0.8, 0.84],
            anisotropy: 0.55,
            detail_scale: 0.06,
            octaves: 5,
            roughness: 0.5,
            turbulence: 1.6,
            billows: 0.1,
            wisps: 0.8,
            softness: 0.4,
            variation: 0.4,
            spread: 0.35,
            meander: 0.7,
            fade: 0.35,
            quality: 3,
            ..Default::default()
        },
    },
    SmokePreset {
        name: "Vapore da tazza",
        hint: "Vapore bianco e leggero che sale dalla tazza e svanisce: il fondo del riquadro va sulla superficie del caffè",
        make: || SmokeCloud {
            name: "Vapore".into(),
            shape: SmokeShape::Plume,
            size: Vec3::new(0.14, 0.25, 0.14),
            density: 110.0,
            color: [0.95, 0.95, 0.95],
            anisotropy: 0.5,
            detail_scale: 0.035,
            octaves: 5,
            roughness: 0.5,
            turbulence: 1.4,
            billows: 0.3,
            wisps: 0.7,
            softness: 0.6,
            variation: 0.7,
            spread: 0.55,
            meander: 0.6,
            fade: 0.85,
            quality: 3,
            ..Default::default()
        },
    },
    SmokePreset {
        name: "Pennacchio da tubo",
        hint:
            "Fumo che esce da un tubo, un ugello o un camino, sale allargandosi e si apre in sbuffi: il fondo del riquadro va sulla bocca",
        make: || SmokeCloud {
            name: "Pennacchio".into(),
            shape: SmokeShape::Plume,
            size: Vec3::new(0.8, 1.6, 0.8),
            density: 45.0,
            color: [0.7, 0.7, 0.7],
            anisotropy: 0.35,
            detail_scale: 0.22,
            octaves: 6,
            roughness: 0.55,
            turbulence: 1.5,
            billows: 0.75,
            wisps: 0.45,
            softness: 0.3,
            variation: 0.55,
            spread: 0.85,
            meander: 0.35,
            fade: 0.4,
            ..Default::default()
        },
    },
    SmokePreset {
        name: "Fumo denso industriale",
        hint: "Fumo scuro e pesante che si gonfia in grossi sbuffi: ciminiere, incendi, scene drammatiche",
        make: || SmokeCloud {
            name: "Fumo denso".into(),
            shape: SmokeShape::Plume,
            size: Vec3::new(1.5, 2.5, 1.5),
            density: 120.0,
            color: [0.22, 0.21, 0.2],
            anisotropy: 0.2,
            detail_scale: 0.4,
            octaves: 7,
            roughness: 0.55,
            turbulence: 1.3,
            billows: 0.95,
            wisps: 0.3,
            softness: 0.2,
            variation: 0.45,
            spread: 1.0,
            meander: 0.25,
            fade: 0.2,
            ..Default::default()
        },
    },
    SmokePreset {
        name: "Banco di fumo",
        hint: "Fumo che riempie tutto il riquadro con i bordi sfilacciati: stanza piena di fumo, palco, strato sotto il soffitto",
        make: || SmokeCloud {
            name: "Banco di fumo".into(),
            shape: SmokeShape::Bank,
            size: Vec3::new(2.0, 0.6, 2.0),
            density: 15.0,
            color: [0.85, 0.85, 0.85],
            anisotropy: 0.4,
            detail_scale: 0.3,
            octaves: 6,
            roughness: 0.55,
            turbulence: 1.8,
            billows: 0.5,
            wisps: 0.6,
            softness: 0.5,
            variation: 0.7,
            ..Default::default()
        },
    },
];

impl SmokeCloud {
    /// Riquadro (min, max) nello spazio mondo.
    pub fn bounds(&self) -> (Vec3, Vec3) {
        let h = self.size.max(Vec3::splat(0.01)) * 0.5;
        (self.center - h, self.center + h)
    }

    /// Parametri riportati in intervalli validi.
    pub fn sanitized(&self) -> SmokeCloud {
        let mut s = self.clone();
        let fin = |v: f32, d: f32| if v.is_finite() { v } else { d };
        s.center = if s.center.is_finite() { s.center } else { Vec3::ZERO };
        s.size = if s.size.is_finite() {
            s.size.clamp(Vec3::splat(0.01), Vec3::splat(100.0))
        } else {
            Vec3::ONE
        };
        s.density = fin(s.density, 40.0).clamp(0.0, 5000.0);
        s.color = s.color.map(|c| fin(c, 0.8).clamp(0.0, 1.0));
        s.anisotropy = fin(s.anisotropy, 0.3).clamp(-0.95, 0.95);
        s.detail_scale = fin(s.detail_scale, 0.15).clamp(0.002, 50.0);
        s.octaves = s.octaves.clamp(1, 10);
        s.roughness = fin(s.roughness, 0.55).clamp(0.2, 0.85);
        s.turbulence = fin(s.turbulence, 1.2).clamp(0.0, 8.0);
        for v in [
            &mut s.billows,
            &mut s.wisps,
            &mut s.softness,
            &mut s.variation,
            &mut s.spread,
            &mut s.meander,
            &mut s.fade,
        ] {
            *v = fin(*v, 0.5).clamp(0.0, 1.0);
        }
        s.bottom_height = fin(s.bottom_height, 0.0).max(0.0);
        s.top_height = fin(s.top_height, 0.0).max(0.0);
        s.quality = s.quality.clamp(1, 4);
        s
    }
}
