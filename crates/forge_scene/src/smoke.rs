//! Fumo volumetrico: nubi di fumo vere (struttura 3D, riccioli, filamenti, bordi sfilacciati),
//! posizionate con un riquadro nella scena e generate da un campo di densità procedurale.
//!
//! Il riquadro (`center`, `size`) è la zona in cui il fumo esiste: spostarlo o ridimensionarlo
//! sposta e ridimensiona il fumo renderizzato. Dentro, la forma (`shape`) dà l'ingombro di
//! massima e il rumore frattale a dominio distorto i dettagli a più scale. Il fumo è fisico:
//! diffonde e assorbe la luce, fa ombra e la riceve (niente emissione: il fuoco è un'altra cosa).
//! Stessi parametri e stesso seme danno sempre la stessa nube.

use forge_math::{Quat, Vec3};
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
    /// Bordi sfumati: fascia vicino alle pareti e al soffitto del riquadro (frazione del lato,
    /// 0–0,4) in cui il fumo si dissolve, così non si ammassa contro il riquadro e non finisce
    /// tagliato netto. 0 = come prima (riquadro pieno fino al bordo). Il fondo non si sfuma.
    pub edge_fade: f32,
    /// Fumo portato da particelle: veli e filamenti sottili che si ripiegano (vapore, incenso)
    /// invece della nuvola gonfia del fumo sulla griglia. Più lento da calcolare.
    pub particles: bool,
    /// Sorgente a chiazze, 0–1 (solo con le particelle): il fumo esce da pochi punti che si
    /// spostano piano, in nastri separati.
    pub patchiness: f32,
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
            edge_fade: 0.2,
            particles: false,
            patchiness: 0.0,
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
    /// Fumo simulato solo nella viewport (anteprima a punti): il render non lo calcola finché non
    /// si riaccende il calcolo vero. Serve a sistemare posizione e sorgente senza aspettare.
    pub preview_only: bool,
    /// Centro del riquadro (m, spazio mondo).
    pub center: Vec3,
    /// Dimensioni del riquadro (m), nei suoi assi.
    pub size: Vec3,
    /// Rotazione del riquadro attorno all'asse verticale (gradi). Solo attorno alla verticale:
    /// il caldo sale e il gas pesante scende comunque in verticale.
    pub rotation_deg: f32,
    /// Densità del fumo pieno (1/m): 5 = fumo leggero, 50 = denso, 300 = quasi opaco.
    pub density: f32,
    /// Contrasto della densità (0,5–4, 1 = come prima): più alto, le parti diradate diventano
    /// trasparenti mentre i grumi restano densi (e fanno ombra su sé stessi), così anche un fumo
    /// leggero tiene il rilievo invece di diventare un velo uniforme.
    pub contrast: f32,
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
            preview_only: false,
            center: Vec3::new(0.0, 0.5, 0.0),
            size: Vec3::splat(0.6),
            rotation_deg: 0.0,
            density: 40.0,
            contrast: 1.0,
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

pub const SMOKE_PRESETS: [SmokePreset; 9] = [
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
        hint: "Simulazione a particelle: vapore leggero e trasparente che sale piano dalla superficie in nastri separati, si torce e svanisce: il fondo del riquadro va sulla superficie del caffè, la sorgente è larga come la tazza",
        // Particelle (veli sottili, niente nuvola gonfia), sorgente su tutta la superficie a
        // chiazze che si spostano (nastri separati), poco caldo e poca densità: trasparente.
        make: || SmokeCloud {
            name: "Vapore".into(),
            mode: SmokeMode::Simulated,
            shape: SmokeShape::Plume,
            size: Vec3::new(0.16, 0.26, 0.16),
            density: 2.5,
            contrast: 1.0,
            color: [0.96, 0.96, 0.96],
            anisotropy: 0.55,
            detail_scale: 0.012,
            quality: 3,
            sim: SmokeSim {
                source_offset: Vec3::new(0.0, 0.008, 0.0),
                source_radius: 0.034,
                emission: 1.2,
                temperature: 0.7,
                buoyancy: 1.2,
                weight: 0.0,
                exit_speed: 0.04,
                cooling: 1.0,
                dissipation: 0.6,
                vorticity: 5.0,
                turbulence: 0.25,
                turbulence_scale: 0.06,
                time: 4.0,
                quality: 2,
                edge_fade: 0.25,
                particles: true,
                patchiness: 0.85,
                ..SmokeSim::default()
            },
            ..Default::default()
        },
    },
    SmokePreset {
        name: "Incenso o sigaretta",
        hint: "Simulazione a particelle: un filo sottile e compatto che sale dritto e poi si rompe in lunghi filamenti e veli che si arricciano: il fondo del riquadro va sulla punta del bastoncino o della sigaretta",
        // Sorgente di un paio di millimetri, fumo caldo che esce veloce (il filo resta dritto e
        // compatto), poca dissolvenza (filamenti lunghi); il contrasto tiene pieno il filo e
        // trasparenti i veli.
        make: || SmokeCloud {
            name: "Incenso".into(),
            mode: SmokeMode::Simulated,
            shape: SmokeShape::Wisp,
            size: Vec3::new(0.22, 0.46, 0.22),
            density: 2.5,
            contrast: 1.8,
            color: [0.85, 0.87, 0.9],
            anisotropy: 0.55,
            detail_scale: 0.012,
            quality: 4,
            sim: SmokeSim {
                source_offset: Vec3::new(0.0, 0.004, 0.0),
                source_radius: 0.002,
                emission: 1.0,
                temperature: 2.0,
                buoyancy: 10.0,
                weight: 0.0,
                exit_speed: 0.3,
                cooling: 0.3,
                dissipation: 0.05,
                vorticity: 5.0,
                turbulence: 0.3,
                turbulence_scale: 0.05,
                time: 5.0,
                quality: 2,
                edge_fade: 0.1,
                particles: true,
                patchiness: 0.0,
                ..SmokeSim::default()
            },
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
    /// Riquadro allineato agli assi del mondo (min, max) che contiene il riquadro ruotato.
    pub fn bounds(&self) -> (Vec3, Vec3) {
        let h = self.half();
        let (s, c) = self.yaw().sin_cos();
        let w = Vec3::new(c.abs() * h.x + s.abs() * h.z, h.y, s.abs() * h.x + c.abs() * h.z);
        (self.center - w, self.center + w)
    }

    /// Mezze dimensioni del riquadro nei suoi assi (m).
    pub fn half(&self) -> Vec3 {
        let s = if self.size.is_finite() { self.size } else { Vec3::ONE };
        s.max(Vec3::splat(0.01)) * 0.5
    }

    fn yaw(&self) -> f32 {
        if self.rotation_deg.is_finite() {
            self.rotation_deg.to_radians()
        } else {
            0.0
        }
    }

    /// Rotazione del riquadro (attorno alla verticale).
    pub fn rotation(&self) -> Quat {
        Quat::from_rotation_y(self.yaw())
    }

    /// Punto del mondo nelle coordinate del riquadro (origine al centro, assi del riquadro).
    pub fn to_local(&self, p: Vec3) -> Vec3 {
        self.rotation().inverse() * (p - self.center)
    }

    /// Punto nelle coordinate del riquadro riportato nel mondo.
    pub fn to_world(&self, l: Vec3) -> Vec3 {
        self.center + self.rotation() * l
    }

    /// Gli 8 spigoli del riquadro (mondo): bit 0 = x, bit 1 = y, bit 2 = z (0 = lato negativo).
    pub fn corners(&self) -> [Vec3; 8] {
        let h = self.half();
        std::array::from_fn(|i| {
            let sx = if i & 1 == 0 { -1.0 } else { 1.0 };
            let sy = if i & 2 == 0 { -1.0 } else { 1.0 };
            let sz = if i & 4 == 0 { -1.0 } else { 1.0 };
            self.to_world(Vec3::new(sx * h.x, sy * h.y, sz * h.z))
        })
    }

    /// Parametri riportati in intervalli validi.
    pub fn sanitized(&self) -> SmokeCloud {
        let mut s = self.clone();
        let fin = |v: f32, d: f32| if v.is_finite() { v } else { d };
        s.center = if s.center.is_finite() { s.center } else { Vec3::ZERO };
        s.rotation_deg = fin(s.rotation_deg, 0.0).rem_euclid(360.0);
        s.size = if s.size.is_finite() {
            s.size.clamp(Vec3::splat(0.01), Vec3::splat(100.0))
        } else {
            Vec3::ONE
        };
        s.density = fin(s.density, 40.0).clamp(0.0, 5000.0);
        s.contrast = fin(s.contrast, 1.0).clamp(0.5, 4.0);
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
        m.edge_fade = fin(m.edge_fade, 0.2).clamp(0.0, 0.4);
        m.patchiness = fin(m.patchiness, 0.0).clamp(0.0, 1.0);
        s
    }

    /// Quanto resta del fumo simulato nel punto `p` per i bordi sfumati (1 = tutto, 0 = sulla
    /// parete): sale dolcemente nella fascia [`SmokeSim::edge_fade`] dalle pareti e dal soffitto.
    pub fn edge_keep(&self, p: Vec3) -> f32 {
        let f = self.sim.edge_fade;
        if f.is_nan() || f <= 0.0 {
            return 1.0;
        }
        let h = self.half();
        let l = self.to_local(p);
        // Distanza dalle pareti in frazione del lato, negli assi del riquadro (il fondo non conta).
        let d = [
            (h.x - l.x.abs()) / (2.0 * h.x),
            (h.y - l.y) / (2.0 * h.y),
            (h.z - l.z.abs()) / (2.0 * h.z),
        ];
        d.iter()
            .map(|&x| {
                let t = (x / f).clamp(0.0, 1.0);
                t * t * (3.0 - 2.0 * t)
            })
            .product()
    }

    /// Centro della sorgente della simulazione (spazio mondo).
    pub fn sim_source(&self) -> Vec3 {
        self.to_world(Vec3::new(0.0, -self.half().y, 0.0) + self.sim.source_offset)
    }

    /// Celle sul lato più lungo del riquadro per la simulazione. Partono da
    /// [`SMOKE_SIM_CELLS`] della qualità; se così una cella è più grande della sorgente (riquadro
    /// grande, sorgente piccola) si rimpiccioliscono fino a far stare la sorgente in circa tre
    /// celle di diametro, senza superare il numero totale di celle della qualità calcolato su un
    /// riquadro cubico (oltre, il calcolo diventerebbe troppo lungo: vedi [`Self::sim_warning`]).
    pub fn sim_cells(&self) -> usize {
        let s = self.sanitized();
        let ext = s.size.max(Vec3::splat(1e-3));
        let longest = ext.max_element();
        let base = SMOKE_SIM_CELLS[(s.sim.quality.clamp(1, 4) - 1) as usize];
        let want = (2.0 * s.sim.source_radius / 3.0).max(1e-4);
        // Lato della cella che tiene il totale entro base³ (il riquadro di partenza è un cubo).
        let budget = (ext.x * ext.y * ext.z / (base as f32).powi(3)).cbrt();
        let cell = (longest / base as f32).min(want.max(budget));
        ((longest / cell).ceil() as usize).clamp(base, 1024)
    }

    /// Lato di una cella della simulazione (m).
    pub fn sim_cell(&self) -> f32 {
        self.sanitized().size.max_element() / self.sim_cells() as f32
    }

    /// Avviso se la simulazione è troppo grossolana per la sorgente: la cella è più grande del
    /// raggio della sorgente, quindi il fumo nasce in pochi grumi (e un gas pesante resta in uno
    /// strato più sottile di una cella, che il pavimento può cancellare). `None` se va bene.
    pub fn sim_warning(&self) -> Option<(f32, f32)> {
        let cell = self.sim_cell();
        let r = self.sanitized().sim.source_radius;
        (self.mode == SmokeMode::Simulated && cell > r).then_some((cell, r))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Riquadro piccolo: le celle della qualità. Riquadro grande con sorgente piccola (il caso
    /// del gas pesante in un riquadro di 10 × 1,5 × 5 m): celle più fini, entro il totale della
    /// qualità, e l'avviso solo se la sorgente resta più piccola di una cella.
    /// Riquadro ruotato attorno alla verticale: la sorgente gira con lui, il riquadro del mondo
    /// lo contiene, i bordi sfumati seguono i suoi assi.
    #[test]
    fn rotated_box_carries_the_source() {
        let mut s = SmokeCloud {
            center: Vec3::new(1.0, 0.5, 0.0),
            size: Vec3::new(2.0, 1.0, 0.5),
            mode: SmokeMode::Simulated,
            ..Default::default()
        };
        s.sim.source_offset = Vec3::new(0.5, 0.1, 0.0);
        s.sim.edge_fade = 0.2;
        let before = s.sim_source();
        assert!((before - Vec3::new(1.5, 0.1, 0.0)).length() < 1e-5);
        s.rotation_deg = 90.0;
        // Ruotato di 90°: l'asse x del riquadro va verso −z.
        assert!((s.sim_source() - Vec3::new(1.0, 0.1, -0.5)).length() < 1e-5, "{:?}", s.sim_source());
        let (lo, hi) = s.bounds();
        assert!((hi - lo - Vec3::new(0.5, 1.0, 2.0)).length() < 1e-4, "{:?}", hi - lo);
        let p = Vec3::new(0.3, 0.2, 0.7);
        assert!((s.to_world(s.to_local(p)) - p).length() < 1e-5);
        // Al centro tutto il fumo, vicino alla parete lunga (ora lungo z) quasi niente.
        assert!((s.edge_keep(Vec3::new(1.0, 0.2, 0.0)) - 1.0).abs() < 1e-5);
        assert!(s.edge_keep(Vec3::new(1.0, 0.2, -0.98)) < 0.05);
        for c in s.corners() {
            assert!(c.cmpge(lo - Vec3::splat(1e-4)).all() && c.cmple(hi + Vec3::splat(1e-4)).all());
        }
    }

    #[test]
    fn sim_cells_follow_the_source() {
        let mut s = SmokeCloud {
            mode: SmokeMode::Simulated,
            size: Vec3::new(1.6, 0.5, 1.6),
            ..Default::default()
        };
        s.sim.source_radius = 0.07;
        s.sim.quality = 2;
        assert_eq!(s.sim_cells(), 72);
        assert!(s.sim_warning().is_none());
        s.size = Vec3::new(10.0845, 1.5035, 4.9446);
        let n = s.sim_cells();
        assert!(n > 72, "{n}");
        let cell = s.sim_cell();
        let total = (s.size / cell).ceil();
        assert!(total.x * total.y * total.z <= 72f32.powi(3) * 1.1, "{total:?}");
        assert!(cell < 0.07 && s.sim_warning().is_none(), "{cell}");
        // Sorgente minuscola: celle al limite del calcolo, avviso.
        s.sim.source_radius = 0.01;
        assert!(s.sim_warning().is_some());
    }
}
