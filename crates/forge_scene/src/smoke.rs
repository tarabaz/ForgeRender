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

/// Come nasce la nube.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SmokeMode {
    /// Forma scelta e rumore frattale: immediato.
    #[default]
    Procedural,
    /// Simulazione fisica (fumo caldo che sale, gas pesante che scende) fino all'istante scelto,
    /// poi lo stesso dettaglio procedurale sul bordo.
    Simulated,
}

/// Parametri della simulazione del fumo (modo [`SmokeMode::Simulated`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SmokeSim {
    /// Sorgente: posizione rispetto al centro del fondo del riquadro (m) e raggio (m).
    pub source_offset: Vec3,
    pub source_radius: f32,
    /// Quanto fumo esce (1 = la sorgente si riempie in un secondo).
    pub emission: f32,
    /// Calore della sorgente (0 = fumo freddo, 1 = caldo tipico, 3 = molto caldo).
    pub temperature: f32,
    /// Spinta del caldo verso l'alto.
    pub buoyancy: f32,
    /// Peso del fumo: alto = gas pesante che scende e si stende.
    pub weight: f32,
    /// Velocità di uscita dalla sorgente (m/s; negativa = verso il basso).
    pub exit_speed: f32,
    /// Raffreddamento (1/s): alto = il fumo smette presto di salire.
    pub cooling: f32,
    /// Dissolvenza del fumo (1/s).
    pub dissipation: f32,
    /// Vortici (confinamento della vorticità).
    pub vorticity: f32,
    /// Turbolenza (agitazione) e la sua grandezza (m).
    pub turbulence: f32,
    pub turbulence_scale: f32,
    /// Vento (m/s).
    pub wind: Vec3,
    /// Secondi simulati: l'istante che si congela per il render.
    pub time: f32,
    /// Qualità della simulazione (1–4): celle sul lato più lungo del riquadro.
    pub quality: u32,
}

/// Celle della simulazione sul lato più lungo per qualità 1–4.
pub const SMOKE_SIM_CELLS: [usize; 4] = [48, 72, 100, 136];

impl Default for SmokeSim {
    fn default() -> Self {
        Self {
            source_offset: Vec3::new(0.0, 0.06, 0.0),
            source_radius: 0.05,
            emission: 3.0,
            temperature: 1.0,
            buoyancy: 4.0,
            weight: 0.3,
            exit_speed: 0.3,
            cooling: 0.6,
            dissipation: 0.05,
            vorticity: 4.0,
            turbulence: 3.0,
            turbulence_scale: 0.15,
            wind: Vec3::ZERO,
            time: 3.0,
            quality: 2,
        }
    }
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
    /// Procedurale (di serie, i progetti di prima) o simulato.
    pub mode: SmokeMode,
    pub sim: SmokeSim,
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
            mode: SmokeMode::Procedural,
            sim: SmokeSim::default(),
        }
    }
}

/// Preset: punti di partenza modificabili. `hint` dice dove mettere il riquadro.
pub struct SmokePreset {
    pub name: &'static str,
    pub hint: &'static str,
    pub make: fn() -> SmokeCloud,
}

pub const SMOKE_PRESETS: [SmokePreset; 8] = [
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
    SmokePreset {
        name: "Fumo caldo simulato",
        hint: "Simulazione fisica: fumo caldo che esce dal fondo del riquadro, sale, si allarga e si arriccia da solo",
        make: || SmokeCloud {
            name: "Fumo simulato".into(),
            mode: SmokeMode::Simulated,
            size: Vec3::new(0.8, 1.4, 0.8),
            density: 60.0,
            color: [0.72, 0.72, 0.72],
            detail_scale: 0.06,
            turbulence: 0.6,
            billows: 0.5,
            wisps: 0.5,
            softness: 0.35,
            variation: 0.3,
            sim: SmokeSim {
                source_radius: 0.08,
                ..SmokeSim::default()
            },
            ..Default::default()
        },
    },
    SmokePreset {
        name: "Gas pesante simulato",
        hint: "Simulazione fisica di un gas freddo e pesante che esce, scende e si stende sul piano, scorrendo attorno agli oggetti",
        make: || SmokeCloud {
            name: "Gas pesante".into(),
            mode: SmokeMode::Simulated,
            size: Vec3::new(1.6, 0.5, 1.6),
            density: 90.0,
            color: [0.82, 0.84, 0.88],
            detail_scale: 0.08,
            turbulence: 0.5,
            billows: 0.6,
            wisps: 0.4,
            softness: 0.4,
            variation: 0.3,
            sim: SmokeSim {
                source_offset: Vec3::new(0.0, 0.3, 0.0),
                source_radius: 0.07,
                emission: 6.0,
                temperature: 0.0,
                weight: 3.0,
                exit_speed: -0.1,
                dissipation: 0.1,
                vorticity: 1.5,
                turbulence: 0.6,
                turbulence_scale: 0.15,
                time: 4.0,
                ..SmokeSim::default()
            },
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
        let m = &mut s.sim;
        m.source_offset = if m.source_offset.is_finite() { m.source_offset } else { Vec3::ZERO };
        m.source_radius = fin(m.source_radius, 0.04).clamp(0.001, 50.0);
        m.emission = fin(m.emission, 3.0).clamp(0.0, 100.0);
        m.temperature = fin(m.temperature, 1.0).clamp(0.0, 10.0);
        m.buoyancy = fin(m.buoyancy, 4.0).clamp(0.0, 50.0);
        m.weight = fin(m.weight, 0.3).clamp(0.0, 50.0);
        m.exit_speed = fin(m.exit_speed, 0.3).clamp(-10.0, 10.0);
        m.cooling = fin(m.cooling, 0.6).clamp(0.0, 20.0);
        m.dissipation = fin(m.dissipation, 0.15).clamp(0.0, 20.0);
        m.vorticity = fin(m.vorticity, 2.5).clamp(0.0, 20.0);
        m.turbulence = fin(m.turbulence, 1.0).clamp(0.0, 20.0);
        m.turbulence_scale = fin(m.turbulence_scale, 0.12).clamp(0.002, 20.0);
        m.wind = if m.wind.is_finite() {
            m.wind.clamp(Vec3::splat(-20.0), Vec3::splat(20.0))
        } else {
            Vec3::ZERO
        };
        m.time = fin(m.time, 3.0).clamp(0.05, 60.0);
        m.quality = m.quality.clamp(1, 4);
        s
    }

    /// Centro della sorgente della simulazione (spazio mondo).
    pub fn sim_source(&self) -> Vec3 {
        let (lo, _) = self.bounds();
        Vec3::new(self.center.x, lo.y, self.center.z) + self.sim.source_offset
    }
}
