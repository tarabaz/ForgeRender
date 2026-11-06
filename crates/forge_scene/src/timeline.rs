//! Timeline: animazione dei parametri della scena nel tempo, come in un programma di montaggio.
//!
//! - Gli **elementi** ([`Target`]) sono le cose animabili: oggetti, luci, camera, fumi, nebbie,
//!   foschia, polvere, ambiente. Ognuno ha le sue **proprietà** compatibili ([`Prop`]); quelle di
//!   base (posizione, rotazione, scala, opacità per gli oggetti) si vedono sempre, le altre si
//!   aggiungono una alla volta.
//! - Posizione, rotazione, bersaglio, colore e scala per asse si vedono e si animano come una riga
//!   sola ([`Target::rows`]): le chiavi di una riga valgono per tutti i suoi assi, che restano
//!   tracce separate (ognuno con la sua curva e i suoi modificatori).
//! - Una **traccia** ([`Track`]) anima una proprietà di un elemento con delle **chiavi**
//!   ([`Key`]): tempo, valore e interpolazione verso la chiave dopo ([`Interp`]: a scatto,
//!   lineare, morbida in entrata, in uscita, in entrata e uscita, curva continua).
//! - Sopra il valore delle chiavi si applicano, in ordine, i **modificatori** ([`Modifier`]):
//!   tremolio, oscillazione, rampa, ripetizione, avanti e indietro, spostamento, moltiplicazione,
//!   limiti, scatti.
//! - Senza chiavi la traccia vale il suo valore di base ([`Track::base`]), più i modificatori.
//!
//! Gli effetti che si calcolano nel tempo (fumo simulato, ghiaccio secco) con la timeline
//! accesa seguono il cursore: l'istante simulato è il tempo della timeline meno l'inizio
//! dell'effetto ([`Timeline::starts`]).

use crate::{ComponentId, MaterialId, Scene};
use forge_math::Vec3;
use serde::{Deserialize, Serialize};

/// Interpolazione da una chiave alla successiva.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Interp {
    /// Il valore resta fermo fino alla chiave dopo, poi salta.
    Hold,
    /// Velocità costante.
    Linear,
    /// Parte piano e accelera.
    EaseIn,
    /// Parte veloce e rallenta.
    EaseOut,
    /// Parte piano, accelera e rallenta all'arrivo (la più naturale).
    #[default]
    EaseInOut,
    /// Curva continua che passa per tutte le chiavi senza fermarsi (Catmull-Rom).
    Smooth,
}

impl Interp {
    pub const ALL: [Interp; 6] = [
        Interp::Hold,
        Interp::Linear,
        Interp::EaseIn,
        Interp::EaseOut,
        Interp::EaseInOut,
        Interp::Smooth,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Interp::Hold => "A scatto",
            Interp::Linear => "Lineare",
            Interp::EaseIn => "Morbida in entrata",
            Interp::EaseOut => "Morbida in uscita",
            Interp::EaseInOut => "Morbida in entrata e uscita",
            Interp::Smooth => "Curva continua",
        }
    }
}

/// Chiave: valore in un istante, e come si arriva alla chiave dopo.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Key {
    /// Secondi dall'inizio della timeline.
    pub t: f32,
    pub v: f32,
    pub interp: Interp,
}

/// Modificatore: cambia il valore della traccia nel tempo, sopra le chiavi.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Modifier {
    /// Tremolio casuale e continuo (ampiezza, volte al secondo, seme).
    Wiggle { amplitude: f32, frequency: f32, seed: u32 },
    /// Oscillazione regolare (seno): ampiezza, volte al secondo, fase in gradi.
    Oscillate { amplitude: f32, frequency: f32, phase_deg: f32 },
    /// Aumenta di continuo: unità al secondo (es. rotazione continua).
    Ramp { per_second: f32 },
    /// Ripete l'animazione delle chiavi dalla prima all'ultima.
    Loop,
    /// Ripete l'animazione delle chiavi avanti e indietro.
    PingPong,
    /// Aggiunge un valore fisso.
    Offset { value: f32 },
    /// Moltiplica per un fattore.
    Multiply { factor: f32 },
    /// Tiene il valore tra un minimo e un massimo.
    Clamp { min: f32, max: f32 },
    /// Animazione a scatti (passo uno): il valore cambia solo `per_second` volte al secondo.
    Steps { per_second: f32 },
}

impl Modifier {
    /// Modificatori di partenza per il pulsante "+".
    pub fn all_defaults() -> Vec<Modifier> {
        vec![
            Modifier::Wiggle {
                amplitude: 0.05,
                frequency: 2.0,
                seed: 1,
            },
            Modifier::Oscillate {
                amplitude: 0.1,
                frequency: 0.5,
                phase_deg: 0.0,
            },
            Modifier::Ramp { per_second: 45.0 },
            Modifier::Loop,
            Modifier::PingPong,
            Modifier::Offset { value: 0.0 },
            Modifier::Multiply { factor: 1.0 },
            Modifier::Clamp { min: 0.0, max: 1.0 },
            Modifier::Steps { per_second: 12.0 },
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Modifier::Wiggle { .. } => "Tremolio",
            Modifier::Oscillate { .. } => "Oscillazione",
            Modifier::Ramp { .. } => "Rampa continua",
            Modifier::Loop => "Ripeti",
            Modifier::PingPong => "Avanti e indietro",
            Modifier::Offset { .. } => "Spostamento",
            Modifier::Multiply { .. } => "Moltiplica",
            Modifier::Clamp { .. } => "Limiti",
            Modifier::Steps { .. } => "A scatti",
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            Modifier::Wiggle { .. } => "Tremolio casuale e continuo attorno al valore (mano che regge la camera, fiamma, insetto)",
            Modifier::Oscillate { .. } => "Va su e giù con regolarità (pendolo, respiro, luce che pulsa)",
            Modifier::Ramp { .. } => "Aumenta sempre della stessa quantità al secondo (rotazione continua, piattaforma girevole)",
            Modifier::Loop => "Ripete l'animazione delle chiavi all'infinito",
            Modifier::PingPong => "Ripete l'animazione delle chiavi andando avanti e poi indietro",
            Modifier::Offset { .. } => "Aggiunge un valore fisso a tutta l'animazione",
            Modifier::Multiply { .. } => "Moltiplica tutta l'animazione (più ampia o più piccola)",
            Modifier::Clamp { .. } => "Il valore non scende sotto il minimo e non sale sopra il massimo",
            Modifier::Steps { .. } => "Il valore cambia a scatti, poche volte al secondo (stop motion)",
        }
    }

    /// Cambia il tempo (ripetizioni e scatti) prima di leggere le chiavi.
    fn remap_time(&self, t: f32, first: f32, last: f32) -> f32 {
        let span = last - first;
        match self {
            Modifier::Loop if span > 1e-4 && t > last => first + (t - first).rem_euclid(span),
            Modifier::PingPong if span > 1e-4 && t > last => {
                let k = (t - first).rem_euclid(2.0 * span);
                first + if k > span { 2.0 * span - k } else { k }
            }
            Modifier::Steps { per_second } if *per_second > 0.0 => (t * per_second).floor() / per_second,
            _ => t,
        }
    }

    /// Cambia il valore.
    fn apply(&self, v: f32, t: f32) -> f32 {
        match self {
            Modifier::Wiggle {
                amplitude,
                frequency,
                seed,
            } => v + amplitude * wiggle(t * frequency.max(0.0), *seed),
            Modifier::Oscillate {
                amplitude,
                frequency,
                phase_deg,
            } => v + amplitude * (std::f32::consts::TAU * frequency * t + phase_deg.to_radians()).sin(),
            Modifier::Ramp { per_second } => v + per_second * t,
            Modifier::Offset { value } => v + value,
            Modifier::Multiply { factor } => v * factor,
            Modifier::Clamp { min, max } => v.clamp(min.min(*max), max.max(*min)),
            _ => v,
        }
    }
}

/// Rumore liscio 1D in [−1, 1] (somma di due ottave di rumore a valori interpolati).
fn wiggle(x: f32, seed: u32) -> f32 {
    let h = |i: i64| {
        let mut k = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (seed as u64).wrapping_mul(0xD1B5_4A32_D192_ED03);
        k ^= k >> 31;
        k = k.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        k ^= k >> 29;
        ((k >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0
    };
    let oct = |x: f32| {
        let i = x.floor();
        let f = x - i;
        let s = f * f * (3.0 - 2.0 * f);
        h(i as i64) * (1.0 - s) + h(i as i64 + 1) * s
    };
    (oct(x) * 0.7 + oct(x * 2.3 + 17.0) * 0.3).clamp(-1.0, 1.0)
}

/// Un modificatore con il suo interruttore.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModSlot {
    pub modifier: Modifier,
    pub enabled: bool,
}

/// Cosa si anima.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Target {
    /// Oggetto (componente) della scena.
    Object(ComponentId),
    /// Luce, per nome.
    Light(String),
    /// Camera che fa il render (quella attiva, o la vista).
    Camera,
    /// Nube di fumo, per numero.
    Smoke(u32),
    /// Nebbia di ghiaccio secco, per nome.
    Fog(String),
    /// Foschia d'ambiente.
    Haze,
    /// Polvere in sospensione.
    Dust,
    /// Ambiente (cielo, HDRI).
    Environment,
}

/// Proprietà animabile (le unità sono quelle dei pannelli: metri, gradi, 0–1...).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Prop {
    PosX,
    PosY,
    PosZ,
    RotX,
    RotY,
    RotZ,
    /// Scala uguale sui tre assi.
    Scale,
    ScaleX,
    ScaleY,
    ScaleZ,
    /// Opacità (0–1).
    Opacity,
    /// Visibile (sotto 0,5 = nascosto).
    Visible,
    Intensity,
    ColorR,
    ColorG,
    ColorB,
    TargetX,
    TargetY,
    TargetZ,
    /// Apertura del cono delle spot (gradi).
    ConeAngle,
    /// Campo visivo verticale (gradi).
    Fov,
    /// Diaframma (f/).
    FStop,
    /// Distanza di messa a fuoco (m).
    FocusDistance,
    Density,
    /// Rotazione dell'HDRI (gradi).
    HdriRotation,
    /// Brillantezza dei granelli di polvere.
    Brightness,
}

impl Prop {
    pub fn label(self) -> &'static str {
        match self {
            Prop::PosX => "Posizione X",
            Prop::PosY => "Posizione Y",
            Prop::PosZ => "Posizione Z",
            Prop::RotX => "Rotazione X",
            Prop::RotY => "Rotazione Y",
            Prop::RotZ => "Rotazione Z",
            Prop::Scale => "Scala",
            Prop::ScaleX => "Scala X",
            Prop::ScaleY => "Scala Y",
            Prop::ScaleZ => "Scala Z",
            Prop::Opacity => "Opacità",
            Prop::Visible => "Visibile",
            Prop::Intensity => "Intensità",
            Prop::ColorR => "Colore R",
            Prop::ColorG => "Colore G",
            Prop::ColorB => "Colore B",
            Prop::TargetX => "Bersaglio X",
            Prop::TargetY => "Bersaglio Y",
            Prop::TargetZ => "Bersaglio Z",
            Prop::ConeAngle => "Apertura del cono",
            Prop::Fov => "Campo visivo",
            Prop::FStop => "Diaframma",
            Prop::FocusDistance => "Distanza di fuoco",
            Prop::Density => "Densità",
            Prop::HdriRotation => "Rotazione HDRI",
            Prop::Brightness => "Brillantezza",
        }
    }

    /// Unità mostrata accanto al valore.
    pub fn unit(self) -> &'static str {
        match self {
            Prop::PosX | Prop::PosY | Prop::PosZ | Prop::TargetX | Prop::TargetY | Prop::TargetZ | Prop::FocusDistance => " m",
            Prop::RotX | Prop::RotY | Prop::RotZ | Prop::ConeAngle | Prop::Fov | Prop::HdriRotation => "°",
            _ => "",
        }
    }

    /// Passo del trascinamento del valore.
    pub fn speed(self) -> f64 {
        match self {
            Prop::RotX | Prop::RotY | Prop::RotZ | Prop::ConeAngle | Prop::Fov | Prop::HdriRotation => 0.5,
            Prop::Intensity | Prop::Density | Prop::Brightness => 0.05,
            _ => 0.005,
        }
    }

    /// Cambia la geometria (va ricostruita la BVH) invece che solo l'aspetto.
    pub fn moves_geometry(self) -> bool {
        matches!(
            self,
            Prop::PosX
                | Prop::PosY
                | Prop::PosZ
                | Prop::RotX
                | Prop::RotY
                | Prop::RotZ
                | Prop::Scale
                | Prop::ScaleX
                | Prop::ScaleY
                | Prop::ScaleZ
                | Prop::Visible
        )
    }
}

/// Una riga della timeline: un nome e le proprietà che comanda insieme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    pub label: &'static str,
    pub props: &'static [Prop],
}

impl Prop {
    /// Assi uniti con questa proprietà in una riga sola (None = proprietà a sé).
    pub fn group(self) -> Option<Row> {
        const POS: Row = Row {
            label: "Posizione",
            props: &[Prop::PosX, Prop::PosY, Prop::PosZ],
        };
        const ROT: Row = Row {
            label: "Rotazione",
            props: &[Prop::RotX, Prop::RotY, Prop::RotZ],
        };
        const SCALE: Row = Row {
            label: "Scala per asse",
            props: &[Prop::ScaleX, Prop::ScaleY, Prop::ScaleZ],
        };
        const TARGET: Row = Row {
            label: "Bersaglio",
            props: &[Prop::TargetX, Prop::TargetY, Prop::TargetZ],
        };
        const COLOR: Row = Row {
            label: "Colore",
            props: &[Prop::ColorR, Prop::ColorG, Prop::ColorB],
        };
        match self {
            Prop::PosX | Prop::PosY | Prop::PosZ => Some(POS),
            Prop::RotX | Prop::RotY | Prop::RotZ => Some(ROT),
            Prop::ScaleX | Prop::ScaleY | Prop::ScaleZ => Some(SCALE),
            Prop::TargetX | Prop::TargetY | Prop::TargetZ => Some(TARGET),
            Prop::ColorR | Prop::ColorG | Prop::ColorB => Some(COLOR),
            _ => None,
        }
    }
}

impl Target {
    /// Righe di un elenco di proprietà: gli assi di un gruppo una volta sola.
    fn rows_of(props: &[Prop]) -> Vec<(&'static str, Vec<Prop>)> {
        let mut out: Vec<(&'static str, Vec<Prop>)> = Vec::new();
        for &p in props {
            match p.group() {
                Some(g) if out.iter().any(|(l, _)| *l == g.label) => {}
                Some(g) => out.push((g.label, g.props.to_vec())),
                None => out.push((p.label(), vec![p])),
            }
        }
        out
    }

    /// Righe di tutte le proprietà compatibili.
    pub fn rows(&self) -> Vec<(&'static str, Vec<Prop>)> {
        Self::rows_of(self.all_props())
    }

    /// La riga è tra quelle di base (sempre in vista).
    pub fn is_base(&self, prop: Prop) -> bool {
        self.base_props().contains(&prop)
    }

    /// Proprietà di base, sempre in vista per l'elemento.
    pub fn base_props(&self) -> &'static [Prop] {
        match self {
            Target::Object(_) => &[
                Prop::PosX,
                Prop::PosY,
                Prop::PosZ,
                Prop::RotX,
                Prop::RotY,
                Prop::RotZ,
                Prop::Scale,
                Prop::Opacity,
            ],
            Target::Light(_) => &[Prop::PosX, Prop::PosY, Prop::PosZ, Prop::Intensity],
            Target::Camera => &[Prop::PosX, Prop::PosY, Prop::PosZ, Prop::TargetX, Prop::TargetY, Prop::TargetZ],
            Target::Smoke(_) => &[Prop::PosX, Prop::PosY, Prop::PosZ, Prop::Density],
            Target::Fog(_) => &[Prop::Density],
            Target::Haze => &[Prop::Density],
            Target::Dust => &[Prop::Brightness],
            Target::Environment => &[Prop::Intensity, Prop::HdriRotation],
        }
    }

    /// Tutte le proprietà compatibili (le base e quelle da aggiungere con "+").
    pub fn all_props(&self) -> &'static [Prop] {
        match self {
            Target::Object(_) => &[
                Prop::PosX,
                Prop::PosY,
                Prop::PosZ,
                Prop::RotX,
                Prop::RotY,
                Prop::RotZ,
                Prop::Scale,
                Prop::ScaleX,
                Prop::ScaleY,
                Prop::ScaleZ,
                Prop::Opacity,
                Prop::Visible,
            ],
            Target::Light(_) => &[
                Prop::PosX,
                Prop::PosY,
                Prop::PosZ,
                Prop::Intensity,
                Prop::ColorR,
                Prop::ColorG,
                Prop::ColorB,
                Prop::TargetX,
                Prop::TargetY,
                Prop::TargetZ,
                Prop::ConeAngle,
                Prop::Visible,
            ],
            Target::Camera => &[
                Prop::PosX,
                Prop::PosY,
                Prop::PosZ,
                Prop::TargetX,
                Prop::TargetY,
                Prop::TargetZ,
                Prop::Fov,
                Prop::FStop,
                Prop::FocusDistance,
            ],
            Target::Smoke(_) => &[
                Prop::PosX,
                Prop::PosY,
                Prop::PosZ,
                Prop::Density,
                Prop::ColorR,
                Prop::ColorG,
                Prop::ColorB,
                Prop::Visible,
            ],
            Target::Fog(_) => &[Prop::Density, Prop::Visible],
            Target::Haze => &[Prop::Density, Prop::ColorR, Prop::ColorG, Prop::ColorB],
            Target::Dust => &[Prop::Brightness],
            Target::Environment => &[Prop::Intensity, Prop::HdriRotation],
        }
    }

    /// Nome da mostrare.
    pub fn name(&self, scene: &Scene) -> String {
        match self {
            Target::Object(id) => scene.find(id).map(|i| scene.nodes[i].name.clone()).unwrap_or_else(|| id.0.clone()),
            Target::Light(n) => n.clone(),
            Target::Camera => "Camera".into(),
            Target::Smoke(n) => scene
                .smokes
                .iter()
                .find(|s| s.n == *n)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| format!("Fumo {n}")),
            Target::Fog(n) => n.clone(),
            Target::Haze => "Foschia d'ambiente".into(),
            Target::Dust => "Polvere in sospensione".into(),
            Target::Environment => "Ambiente".into(),
        }
    }

    /// L'elemento c'è ancora nella scena.
    pub fn exists(&self, scene: &Scene) -> bool {
        match self {
            Target::Object(id) => scene.find(id).is_some(),
            Target::Light(n) => scene.lights.iter().any(|l| &l.name == n),
            Target::Smoke(n) => scene.smokes.iter().any(|s| s.n == *n),
            Target::Fog(n) => scene.fogs.iter().any(|f| &f.name == n),
            _ => true,
        }
    }
}

/// Animazione di una proprietà di un elemento.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Track {
    pub target: Target,
    pub prop: Prop,
    /// Chiavi in ordine di tempo.
    pub keys: Vec<Key>,
    /// Valore senza chiavi (i modificatori si applicano sopra).
    pub base: f32,
    pub modifiers: Vec<ModSlot>,
    pub enabled: bool,
    /// Ultimo valore scritto nella scena: se la scena ora dice altro l'ha cambiato l'utente
    /// (gizmo, pannelli) e la timeline registra una chiave invece di rimettere il suo valore.
    #[serde(skip)]
    pub last: Option<f32>,
    /// Ultimo valore chiesto alla scena: se la scena lo limita (scala a zero, opacità sopra il
    /// 100 %) non si riscrive a ogni fotogramma, altrimenti la scena sembrerebbe cambiare sempre.
    #[serde(skip)]
    pub wanted: Option<f32>,
}

impl Default for Track {
    fn default() -> Self {
        Self {
            target: Target::Camera,
            prop: Prop::PosX,
            keys: Vec::new(),
            base: 0.0,
            modifiers: Vec::new(),
            enabled: true,
            last: None,
            wanted: None,
        }
    }
}

impl Track {
    /// Anima qualcosa: con chiavi o con modificatori accesi. Senza, la traccia non tocca la scena.
    pub fn is_animated(&self) -> bool {
        self.enabled && (!self.keys.is_empty() || self.modifiers.iter().any(|m| m.enabled))
    }

    /// Valore nell'istante `t` (secondi).
    pub fn value_at(&self, t: f32) -> f32 {
        let active = || self.modifiers.iter().filter(|m| m.enabled).map(|m| &m.modifier);
        let (first, last) = match (self.keys.first(), self.keys.last()) {
            (Some(a), Some(b)) => (a.t, b.t),
            _ => (0.0, 0.0),
        };
        let mut tk = t;
        for m in active() {
            tk = m.remap_time(tk, first, last);
        }
        let mut v = if self.keys.is_empty() { self.base } else { self.keys_at(tk) };
        for m in active() {
            v = m.apply(v, t);
        }
        v
    }

    /// Valore delle sole chiavi.
    pub fn keys_at(&self, t: f32) -> f32 {
        let k = &self.keys;
        match k.len() {
            0 => self.base,
            1 => k[0].v,
            _ => {
                if t <= k[0].t {
                    return k[0].v;
                }
                if t >= k[k.len() - 1].t {
                    return k[k.len() - 1].v;
                }
                let i = k.partition_point(|key| key.t <= t).saturating_sub(1).min(k.len() - 2);
                let (a, b) = (k[i], k[i + 1]);
                let span = (b.t - a.t).max(1e-6);
                let u = ((t - a.t) / span).clamp(0.0, 1.0);
                match a.interp {
                    Interp::Hold => a.v,
                    Interp::Linear => a.v + (b.v - a.v) * u,
                    Interp::EaseIn => a.v + (b.v - a.v) * u * u * u,
                    Interp::EaseOut => a.v + (b.v - a.v) * (1.0 - (1.0 - u).powi(3)),
                    Interp::EaseInOut => {
                        let s = if u < 0.5 {
                            4.0 * u * u * u
                        } else {
                            1.0 - (-2.0 * u + 2.0).powi(3) / 2.0
                        };
                        a.v + (b.v - a.v) * s
                    }
                    Interp::Smooth => {
                        // Catmull-Rom con tangenti dalle chiavi vicine (tempi non uniformi).
                        let p0 = if i > 0 { k[i - 1] } else { a };
                        let p3 = if i + 2 < k.len() { k[i + 2] } else { b };
                        let m1 = if i > 0 {
                            (b.v - p0.v) / (b.t - p0.t).max(1e-6) * span
                        } else {
                            b.v - a.v
                        };
                        let m2 = if i + 2 < k.len() {
                            (p3.v - a.v) / (p3.t - a.t).max(1e-6) * span
                        } else {
                            b.v - a.v
                        };
                        let (u2, u3) = (u * u, u * u * u);
                        (2.0 * u3 - 3.0 * u2 + 1.0) * a.v + (u3 - 2.0 * u2 + u) * m1 + (-2.0 * u3 + 3.0 * u2) * b.v + (u3 - u2) * m2
                    }
                }
            }
        }
    }

    /// Chiave all'istante `t` (entro `tol` secondi), se c'è.
    pub fn key_near(&self, t: f32, tol: f32) -> Option<usize> {
        self.keys.iter().position(|k| (k.t - t).abs() <= tol)
    }

    /// Mette (o aggiorna) una chiave nell'istante `t`.
    pub fn set_key(&mut self, t: f32, v: f32, tol: f32) {
        match self.key_near(t, tol) {
            Some(i) => self.keys[i].v = v,
            None => {
                let interp = self.keys.last().map(|k| k.interp).unwrap_or_default();
                self.keys.push(Key { t, v, interp });
                self.sort();
            }
        }
    }

    pub fn sort(&mut self) {
        self.keys.sort_by(|a, b| a.t.total_cmp(&b.t));
    }
}

/// Cosa è cambiato applicando la timeline (cosa va rifatto).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Applied {
    /// Oggetti nascosti o mostrati: cambia l'elenco di quello che c'è da disegnare.
    pub geometry: bool,
    /// Oggetti spostati, ruotati o scalati: cambiano solo le loro matrici.
    pub transforms: bool,
    /// Materiali, luci, ambiente, effetti: si rifà l'aspetto.
    pub shading: bool,
    pub camera: bool,
    /// Effetti calcolati nel tempo (fumo, ghiaccio secco): l'istante è cambiato.
    pub effects: bool,
}

impl Applied {
    pub fn any(&self) -> bool {
        self.geometry || self.transforms || self.shading || self.camera || self.effects
    }
}

/// Timeline del progetto.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Timeline {
    /// Durata dell'animazione (secondi).
    pub duration: f32,
    /// Fotogrammi al secondo (per il render dell'animazione e lo scatto del cursore).
    pub fps: f32,
    /// Posizione del cursore (secondi).
    pub time: f32,
    /// Elementi mostrati, nell'ordine.
    pub elements: Vec<Target>,
    pub tracks: Vec<Track>,
    /// Inizio degli effetti calcolati nel tempo (fumo, ghiaccio secco), in secondi: prima non
    /// c'è niente, poi l'effetto parte da zero.
    pub starts: Vec<(Target, f32)>,
    /// Fine dell'emissione degli effetti nel tempo (secondi della timeline): da lì la sorgente non
    /// emette più, ma quello che è uscito continua a vivere. Senza voce: emette sempre.
    pub stops: Vec<(Target, f32)>,
    /// Area di lavoro: la parte che si riproduce e si salva (secondi). `work_out` ≤ 0 = fino alla
    /// fine della durata.
    pub work_in: f32,
    pub work_out: f32,
}

impl Default for Timeline {
    fn default() -> Self {
        Self {
            duration: 5.0,
            fps: 24.0,
            time: 0.0,
            elements: Vec::new(),
            tracks: Vec::new(),
            starts: Vec::new(),
            stops: Vec::new(),
            work_in: 0.0,
            work_out: 0.0,
        }
    }
}

/// Due chiavi più vicine di così sono la stessa.
const KEY_TOL: f32 = 1e-4;

/// Tempo minimo simulato per un effetto appena partito (un istante a zero non ha fumo).
pub const MIN_EFFECT_TIME: f32 = 0.05;

impl Timeline {
    /// Area di lavoro (inizio, fine), dentro la durata.
    pub fn work_area(&self) -> (f32, f32) {
        let end = if self.work_out > 0.0 {
            self.work_out.min(self.duration)
        } else {
            self.duration
        };
        (self.work_in.clamp(0.0, end), end)
    }

    /// Fotogrammi dell'area di lavoro: gli istanti da salvare.
    pub fn work_frames(&self) -> Vec<f32> {
        let (a, b) = self.work_area();
        let f = self.frame();
        let n = (((b - a) / f).round() as usize).max(1);
        (0..n).map(|k| a + k as f32 * f).collect()
    }

    /// Durata di un fotogramma.
    pub fn frame(&self) -> f32 {
        1.0 / self.fps.clamp(1.0, 240.0)
    }

    /// Tempo arrotondato al fotogramma.
    pub fn snap(&self, t: f32) -> f32 {
        let f = self.frame();
        ((t / f).round() * f).clamp(0.0, self.duration.max(f))
    }

    pub fn track(&self, target: &Target, prop: Prop) -> Option<usize> {
        self.tracks.iter().position(|t| &t.target == target && t.prop == prop)
    }

    /// Aggiunge l'elemento (se non c'è già) con le sue tracce di base, ferme sul valore attuale.
    pub fn add_element(&mut self, scene: &Scene, target: Target) {
        if !self.elements.contains(&target) {
            self.elements.push(target.clone());
        }
        for &p in target.base_props() {
            self.add_track(scene, target.clone(), p);
        }
    }

    /// Aggiunge la traccia di una proprietà (se non c'è già), ferma sul valore attuale.
    pub fn add_track(&mut self, scene: &Scene, target: Target, prop: Prop) -> usize {
        if let Some(i) = self.track(&target, prop) {
            return i;
        }
        let base = get(scene, &target, prop).unwrap_or(0.0);
        self.tracks.push(Track {
            target,
            prop,
            base,
            ..Default::default()
        });
        self.tracks.len() - 1
    }

    /// Toglie l'elemento e le sue tracce.
    pub fn remove_element(&mut self, target: &Target) {
        self.elements.retain(|t| t != target);
        self.tracks.retain(|t| &t.target != target);
        self.starts.retain(|(t, _)| t != target);
        self.stops.retain(|(t, _)| t != target);
    }

    /// Inizio dell'effetto `target` (0 se non impostato).
    pub fn start_of(&self, target: &Target) -> f32 {
        self.starts.iter().find(|(t, _)| t == target).map(|(_, s)| *s).unwrap_or(0.0)
    }

    pub fn set_start(&mut self, target: &Target, start: f32) {
        match self.starts.iter_mut().find(|(t, _)| t == target) {
            Some(s) => s.1 = start,
            None => self.starts.push((target.clone(), start)),
        }
    }

    /// Fine dell'emissione dell'effetto (None = emette sempre).
    pub fn stop_of(&self, target: &Target) -> Option<f32> {
        self.stops.iter().find(|(t, _)| t == target).map(|(_, s)| *s)
    }

    pub fn set_stop(&mut self, target: &Target, stop: Option<f32>) {
        self.stops.retain(|(t, _)| t != target);
        if let Some(s) = stop {
            self.stops.push((target.clone(), s));
        }
    }

    /// Istante simulato dell'effetto all'istante `t` della timeline.
    pub fn effect_time(&self, target: &Target, t: f32) -> f32 {
        (t - self.start_of(target)).max(MIN_EFFECT_TIME)
    }

    /// Tempi di tutte le chiavi (per saltare da una chiave all'altra).
    pub fn key_times(&self) -> Vec<f32> {
        let mut v: Vec<f32> = self.tracks.iter().flat_map(|t| t.keys.iter().map(|k| k.t)).collect();
        v.sort_by(f32::total_cmp);
        v.dedup_by(|a, b| (*a - *b).abs() < 1e-4);
        v
    }

    /// Applica la timeline alla scena nell'istante `self.time`. Una proprietà cambiata
    /// dall'utente dall'ultima volta (gizmo, pannelli) diventa una chiave nell'istante attuale
    /// (o il nuovo valore di base, se la traccia non ha chiavi) invece di essere rimessa com'era.
    pub fn apply(&mut self, scene: &mut Scene) -> Applied {
        let t = self.time;
        let tol = self.frame() * 0.5;
        let mut out = Applied::default();
        // Tracce con una chiave registrata: gli altri assi della riga la prendono anche loro.
        let mut recorded: Vec<(Target, Prop)> = Vec::new();
        for tr in &mut self.tracks {
            if !tr.enabled || !tr.target.exists(scene) {
                continue;
            }
            let Some(now) = get(scene, &tr.target, tr.prop) else { continue };
            if let Some(last) = tr.last {
                if (now - last).abs() > 1e-5 {
                    // L'utente ha cambiato il valore: si registra.
                    if tr.keys.is_empty() {
                        tr.base = now;
                    } else {
                        tr.set_key(t, now, tol);
                        recorded.push((tr.target.clone(), tr.prop));
                    }
                    tr.last = Some(now);
                    continue;
                }
            }
            if !tr.is_animated() {
                tr.last = Some(now);
                continue;
            }
            let v = tr.value_at(t);
            let same = tr.wanted.is_some_and(|w| (w - v).abs() <= 1e-6) && tr.last.is_some_and(|l| (l - now).abs() <= 1e-5);
            if (v - now).abs() > 1e-6 && !same {
                tr.wanted = Some(v);
                set(scene, &tr.target, tr.prop, v);
                if tr.prop == Prop::Visible && matches!(tr.target, Target::Object(_)) {
                    out.geometry = true;
                } else if tr.prop.moves_geometry() && matches!(tr.target, Target::Object(_)) {
                    out.transforms = true;
                } else if matches!(tr.target, Target::Camera) {
                    out.camera = true;
                } else {
                    out.shading = true;
                }
            }
            tr.last = get(scene, &tr.target, tr.prop);
        }
        for (target, prop) in recorded {
            let Some(g) = prop.group() else { continue };
            for &axis in g.props.iter().filter(|&&a| a != prop) {
                if let (Some(i), Some(v)) = (self.track(&target, axis), get(scene, &target, axis)) {
                    self.tracks[i].set_key(t, v, tol);
                    self.tracks[i].last = Some(v);
                }
            }
        }
        // Effetti nel tempo: l'istante simulato segue il cursore.
        let smokes: Vec<(usize, f32)> = scene
            .smokes
            .iter()
            .enumerate()
            .filter(|(_, s)| s.mode == crate::SmokeMode::Simulated)
            .map(|(i, s)| (i, self.effect_time(&Target::Smoke(s.n), t)))
            .collect();
        for (i, st) in smokes {
            if (scene.smokes[i].sim.time - st).abs() > 1e-4 {
                scene.smokes[i].sim.time = st;
                out.effects = true;
            }
            let target = Target::Smoke(scene.smokes[i].n);
            // Secondi simulati: dall'inizio dell'effetto (0 = non si ferma).
            let stop = self.stop_of(&target).map(|s| (s - self.start_of(&target)).max(1e-3)).unwrap_or(0.0);
            if (scene.smokes[i].sim.emit_stop - stop).abs() > 1e-4 {
                scene.smokes[i].sim.emit_stop = stop;
                out.effects = true;
            }
        }
        let fogs: Vec<(usize, f32)> = scene
            .fogs
            .iter()
            .enumerate()
            .map(|(i, f)| (i, self.effect_time(&Target::Fog(f.name.clone()), t)))
            .collect();
        for (i, ft) in fogs {
            if (scene.fogs[i].sim_time - ft).abs() > 1e-4 {
                scene.fogs[i].sim_time = ft;
                out.effects = true;
            }
            let target = Target::Fog(scene.fogs[i].name.clone());
            let stop = self.stop_of(&target).map(|s| (s - self.start_of(&target)).max(1e-3)).unwrap_or(0.0);
            if (scene.fogs[i].emit_stop - stop).abs() > 1e-4 {
                scene.fogs[i].emit_stop = stop;
                out.effects = true;
            }
        }
        out
    }

    /// Il cursore va sull'istante degli effetti nel tempo della scena, così restano come erano
    /// (un fumo salvato a 4 s non riparte vuoto): serve quando arriva un effetto o si apre un
    /// progetto senza animazione.
    pub fn adopt_effects(&mut self, scene: &Scene) {
        // Ogni effetto nel tempo ha la sua riga, con inizio e fine a portata di mano.
        let simulated = scene.smokes.iter().filter(|s| s.mode == crate::SmokeMode::Simulated);
        let targets: Vec<Target> = simulated
            .map(|s| Target::Smoke(s.n))
            .chain(scene.fogs.iter().map(|f| Target::Fog(f.name.clone())))
            .collect();
        for target in targets {
            if !self.elements.contains(&target) {
                self.elements.push(target);
            }
        }
        let t = scene
            .smokes
            .iter()
            .filter(|s| s.mode == crate::SmokeMode::Simulated)
            .map(|s| s.sim.time)
            .chain(scene.fogs.iter().map(|f| f.sim_time))
            .fold(0.0f32, f32::max);
        self.duration = self.duration.max(t);
        // Con un'animazione già fatta il cursore resta dov'è.
        if t > self.time && self.tracks.iter().all(|tr| !tr.is_animated()) {
            self.time = t;
        }
    }

    /// Indici delle chiavi selezionate ((traccia, tempo) ancora esistenti).
    fn resolve(&self, sel: &[(usize, f32)]) -> Vec<(usize, usize)> {
        sel.iter()
            .filter_map(|&(i, t)| Some((i, self.tracks.get(i)?.key_near(t, KEY_TOL)?)))
            .collect()
    }

    /// Sposta le chiavi selezionate di `dt` secondi (restano dentro la durata) e aggiorna la
    /// selezione. Restituisce lo spostamento fatto.
    pub fn shift_keys(&mut self, sel: &mut [(usize, f32)], dt: f32) -> f32 {
        let keys = self.resolve(sel);
        if keys.is_empty() {
            return 0.0;
        }
        let times = keys.iter().map(|&(i, k)| self.tracks[i].keys[k].t);
        let (lo, hi) = times.fold((f32::MAX, f32::MIN), |(a, b), t| (a.min(t), b.max(t)));
        let dt = dt.clamp(-lo, (self.duration - hi).max(0.0));
        for &(i, k) in &keys {
            self.tracks[i].keys[k].t += dt;
        }
        for s in sel.iter_mut() {
            s.1 += dt;
        }
        for &(i, _) in &keys {
            self.tracks[i].sort();
        }
        dt
    }

    /// Dopo uno spostamento: due chiavi nello stesso istante di una traccia diventano una, e vale
    /// quella spostata (`moved`: le chiavi come erano prima, vedi [`Timeline::copy_keys`]).
    pub fn settle_keys(&mut self, moved: &[(usize, Key)]) {
        for tr in 0..self.tracks.len() {
            let chosen: Vec<f32> = moved.iter().filter(|m| m.0 == tr).map(|m| m.1.v).collect();
            let keys = &mut self.tracks[tr].keys;
            let mut k = 0;
            while k + 1 < keys.len() {
                if (keys[k + 1].t - keys[k].t).abs() > KEY_TOL {
                    k += 1;
                    continue;
                }
                // Stesso istante: resta quella spostata.
                let second_moved = chosen.contains(&keys[k + 1].v);
                keys.remove(if second_moved { k } else { k + 1 });
            }
        }
    }

    pub fn delete_keys(&mut self, scene: &Scene, sel: &[(usize, f32)]) {
        let mut keys = self.resolve(sel);
        keys.sort_unstable();
        keys.dedup();
        for &(i, k) in keys.iter().rev() {
            self.tracks[i].keys.remove(k);
            if self.tracks[i].keys.is_empty() {
                // Senza chiavi resta il valore attuale.
                let tr = &mut self.tracks[i];
                tr.base = get(scene, &tr.target, tr.prop).unwrap_or(tr.base);
            }
        }
    }

    pub fn set_keys_interp(&mut self, sel: &[(usize, f32)], interp: Interp) {
        for (i, k) in self.resolve(sel) {
            self.tracks[i].keys[k].interp = interp;
        }
    }

    /// Chiavi selezionate come (traccia, tempo, valore, interpolazione), per copiarle.
    pub fn copy_keys(&self, sel: &[(usize, f32)]) -> Vec<(usize, Key)> {
        self.resolve(sel).into_iter().map(|(i, k)| (i, self.tracks[i].keys[k])).collect()
    }

    /// Incolla le chiavi con la prima all'istante `at`; restituisce la nuova selezione.
    pub fn paste_keys(&mut self, keys: &[(usize, Key)], at: f32) -> Vec<(usize, f32)> {
        let first = keys.iter().map(|k| k.1.t).fold(f32::MAX, f32::min);
        let tol = self.frame() * 0.5;
        let mut sel = Vec::new();
        for &(i, k) in keys {
            let Some(tr) = self.tracks.get_mut(i) else { continue };
            let t = k.t - first + at;
            tr.set_key(t, k.v, tol);
            if let Some(j) = tr.key_near(t, tol) {
                tr.keys[j].interp = k.interp;
                sel.push((i, tr.keys[j].t));
            }
        }
        sel
    }

    /// Toglie le tracce e gli elementi di cose che non ci sono più.
    pub fn prune(&mut self, scene: &Scene) {
        self.elements.retain(|t| t.exists(scene));
        self.tracks.retain(|t| t.target.exists(scene));
        self.starts.retain(|(t, _)| t.exists(scene));
        self.stops.retain(|(t, _)| t.exists(scene));
    }
}

/// Id del materiale con l'opacità di un oggetto animata.
pub fn opacity_material_id(node: &ComponentId) -> MaterialId {
    MaterialId::new(format!("{OPACITY_PREFIX}{}", node.0))
}

/// Prefisso dei materiali creati per l'opacità animata degli oggetti.
pub const OPACITY_PREFIX: &str = "timeline:opacita:";

fn camera_mut(scene: &mut Scene) -> &mut crate::Camera {
    match scene.active_camera {
        Some(i) if i < scene.render_cameras.len() => &mut scene.render_cameras[i].camera,
        _ => &mut scene.camera,
    }
}

fn axis(v: Vec3, p: Prop) -> f32 {
    match p {
        Prop::PosX | Prop::RotX | Prop::ScaleX | Prop::TargetX | Prop::ColorR => v.x,
        Prop::PosY | Prop::RotY | Prop::ScaleY | Prop::TargetY | Prop::ColorG => v.y,
        _ => v.z,
    }
}

fn set_axis(v: &mut Vec3, p: Prop, x: f32) {
    match p {
        Prop::PosX | Prop::RotX | Prop::ScaleX | Prop::TargetX | Prop::ColorR => v.x = x,
        Prop::PosY | Prop::RotY | Prop::ScaleY | Prop::TargetY | Prop::ColorG => v.y = x,
        _ => v.z = x,
    }
}

fn rgb(c: [f32; 3]) -> Vec3 {
    Vec3::from_array(c)
}

/// Valore attuale di una proprietà (None se l'elemento o la proprietà non c'è).
pub fn get(scene: &Scene, target: &Target, prop: Prop) -> Option<f32> {
    use Prop::*;
    match target {
        Target::Object(id) => {
            let i = scene.find(id)?;
            let n = &scene.nodes[i];
            let u = n.user;
            Some(match prop {
                PosX | PosY | PosZ => axis(u.translation, prop),
                RotX | RotY | RotZ => axis(u.rotation_deg, prop),
                ScaleX | ScaleY | ScaleZ => axis(u.scale, prop),
                Scale => (u.scale.x + u.scale.y + u.scale.z) / 3.0,
                Visible => n.visible as u8 as f32,
                Opacity => scene.materials.get(scene.resolve_material(i)).map(|m| m.opacity).unwrap_or(1.0),
                _ => return None,
            })
        }
        Target::Light(name) => {
            let l = scene.lights.iter().find(|l| &l.name == name)?;
            Some(match prop {
                PosX | PosY | PosZ => axis(l.position, prop),
                TargetX | TargetY | TargetZ => axis(l.target, prop),
                Intensity => l.intensity,
                ColorR | ColorG | ColorB => axis(rgb(l.color), prop),
                Visible => l.enabled as u8 as f32,
                ConeAngle => match l.kind {
                    crate::LightKind::Spot { outer_angle_deg, .. } => outer_angle_deg,
                    _ => return None,
                },
                _ => return None,
            })
        }
        Target::Camera => {
            let c = scene.render_camera();
            Some(match prop {
                PosX | PosY | PosZ => axis(c.position, prop),
                TargetX | TargetY | TargetZ => axis(c.target, prop),
                Fov => c.vfov_deg,
                FStop => c.lens.f_stop,
                FocusDistance => c.lens.focus_distance,
                _ => return None,
            })
        }
        Target::Smoke(n) => {
            let s = scene.smokes.iter().find(|s| s.n == *n)?;
            Some(match prop {
                PosX | PosY | PosZ => axis(s.center, prop),
                Density => s.density,
                ColorR | ColorG | ColorB => axis(rgb(s.color), prop),
                Visible => s.enabled as u8 as f32,
                _ => return None,
            })
        }
        Target::Fog(name) => {
            let f = scene.fogs.iter().find(|f| &f.name == name)?;
            Some(match prop {
                Density => f.density,
                Visible => f.enabled as u8 as f32,
                _ => return None,
            })
        }
        Target::Haze => Some(match prop {
            Density => scene.haze.density,
            ColorR | ColorG | ColorB => axis(rgb(scene.haze.color), prop),
            _ => return None,
        }),
        Target::Dust => Some(match prop {
            Brightness => scene.haze_dust.brightness,
            _ => return None,
        }),
        Target::Environment => {
            use crate::Environment::*;
            Some(match (prop, &scene.environment) {
                (Intensity, Uniform { strength, .. } | Gradient { strength, .. } | Hdri { strength, .. }) => *strength,
                (HdriRotation, Hdri { rotation_deg, .. }) => *rotation_deg,
                _ => return None,
            })
        }
    }
}

/// Scrive una proprietà nella scena.
pub fn set(scene: &mut Scene, target: &Target, prop: Prop, v: f32) {
    use Prop::*;
    match target {
        Target::Object(id) => {
            let Some(i) = scene.find(id) else { return };
            match prop {
                PosX | PosY | PosZ => set_axis(&mut scene.nodes[i].user.translation, prop, v),
                RotX | RotY | RotZ => set_axis(&mut scene.nodes[i].user.rotation_deg, prop, v),
                ScaleX | ScaleY | ScaleZ => set_axis(&mut scene.nodes[i].user.scale, prop, v.max(1e-4)),
                Scale => {
                    let s = scene.nodes[i].user.scale;
                    let avg = (s.x + s.y + s.z) / 3.0;
                    let k = if avg.abs() > 1e-6 { v.max(1e-4) / avg } else { 1.0 };
                    scene.nodes[i].user.scale = s * k;
                }
                Visible => scene.nodes[i].visible = v >= 0.5,
                Opacity => set_object_opacity(scene, i, v),
                _ => {}
            }
        }
        Target::Light(name) => {
            let Some(l) = scene.lights.iter_mut().find(|l| &l.name == name) else {
                return;
            };
            match prop {
                PosX | PosY | PosZ => set_axis(&mut l.position, prop, v),
                TargetX | TargetY | TargetZ => set_axis(&mut l.target, prop, v),
                Intensity => l.intensity = v.max(0.0),
                ColorR | ColorG | ColorB => {
                    let mut c = rgb(l.color);
                    set_axis(&mut c, prop, v.clamp(0.0, 1.0));
                    l.color = c.to_array();
                }
                Visible => l.enabled = v >= 0.5,
                ConeAngle => {
                    if let crate::LightKind::Spot {
                        inner_angle_deg,
                        outer_angle_deg,
                    } = &mut l.kind
                    {
                        let ratio = if *outer_angle_deg > 0.0 {
                            *inner_angle_deg / *outer_angle_deg
                        } else {
                            0.7
                        };
                        *outer_angle_deg = v.clamp(1.0, 89.0);
                        *inner_angle_deg = *outer_angle_deg * ratio;
                    }
                }
                _ => {}
            }
        }
        Target::Camera => {
            let c = camera_mut(scene);
            match prop {
                PosX | PosY | PosZ => set_axis(&mut c.position, prop, v),
                TargetX | TargetY | TargetZ => set_axis(&mut c.target, prop, v),
                Fov => c.vfov_deg = v.clamp(1.0, 170.0),
                FStop => c.lens.f_stop = v.max(0.5),
                FocusDistance => c.lens.focus_distance = v.max(0.001),
                _ => {}
            }
        }
        Target::Smoke(n) => {
            let Some(s) = scene.smokes.iter_mut().find(|s| s.n == *n) else {
                return;
            };
            match prop {
                PosX | PosY | PosZ => set_axis(&mut s.center, prop, v),
                Density => s.density = v.max(0.0),
                ColorR | ColorG | ColorB => {
                    let mut c = rgb(s.color);
                    set_axis(&mut c, prop, v.clamp(0.0, 1.0));
                    s.color = c.to_array();
                }
                Visible => s.enabled = v >= 0.5,
                _ => {}
            }
        }
        Target::Fog(name) => {
            let Some(f) = scene.fogs.iter_mut().find(|f| &f.name == name) else {
                return;
            };
            match prop {
                Density => f.density = v.max(0.0),
                Visible => f.enabled = v >= 0.5,
                _ => {}
            }
        }
        Target::Haze => match prop {
            Density => scene.haze.density = v.max(0.0),
            ColorR | ColorG | ColorB => {
                let mut c = rgb(scene.haze.color);
                set_axis(&mut c, prop, v.clamp(0.0, 1.0));
                scene.haze.color = c.to_array();
            }
            _ => {}
        },
        Target::Dust => {
            if prop == Brightness {
                scene.haze_dust.brightness = v.max(0.0);
            }
        }
        Target::Environment => {
            use crate::Environment::*;
            match (prop, &mut scene.environment) {
                (Intensity, Uniform { strength, .. } | Gradient { strength, .. } | Hdri { strength, .. }) => *strength = v.max(0.0),
                (HdriRotation, Hdri { rotation_deg, .. }) => *rotation_deg = v,
                _ => {}
            }
        }
    }
}

/// Opacità di un oggetto: un materiale suo (copia di quello che aveva) con l'opacità scelta;
/// tornando a 1 riprende il materiale di prima.
fn set_object_opacity(scene: &mut Scene, node: usize, v: f32) {
    let id = scene.nodes[node].id.clone();
    let own = opacity_material_id(&id);
    let v = v.clamp(0.0, 1.0);
    let current = scene.resolve_material(node).clone();
    if current == own {
        if let Some(m) = scene.materials.materials.iter_mut().find(|m| m.id == own) {
            m.opacity = v;
        }
        return;
    }
    if v >= 1.0 {
        return;
    }
    let Some(src) = scene.materials.get(&current).cloned() else {
        return;
    };
    let mut m = src;
    m.id = own.clone();
    m.name = format!("{} (opacità animata)", m.name);
    m.opacity = v;
    scene.materials.upsert(m);
    // Il materiale di prima resta come origine: la copia tiene i suoi parametri.
    scene.assignments.insert(id, own);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(keys: &[(f32, f32)], interp: Interp) -> Track {
        Track {
            keys: keys.iter().map(|&(t, v)| Key { t, v, interp }).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn interpolations_pass_through_the_keys() {
        for i in Interp::ALL {
            let t = track(&[(0.0, 0.0), (1.0, 10.0), (2.0, 4.0)], i);
            assert!((t.value_at(0.0) - 0.0).abs() < 1e-4, "{i:?}");
            if i != Interp::Hold {
                assert!((t.value_at(1.0) - 10.0).abs() < 1e-4, "{i:?}");
            }
            assert!((t.value_at(5.0) - 4.0).abs() < 1e-4, "dopo l'ultima chiave resta lì ({i:?})");
        }
        let lin = track(&[(0.0, 0.0), (2.0, 10.0)], Interp::Linear);
        assert!((lin.value_at(1.0) - 5.0).abs() < 1e-4);
        let ease = track(&[(0.0, 0.0), (2.0, 10.0)], Interp::EaseIn);
        assert!(ease.value_at(0.5) < lin.value_at(0.5), "morbida in entrata parte più piano");
        let hold = track(&[(0.0, 1.0), (2.0, 10.0)], Interp::Hold);
        assert_eq!(hold.value_at(1.9), 1.0);
    }

    #[test]
    fn modifiers_change_the_value() {
        let mut t = track(&[(0.0, 0.0), (1.0, 1.0)], Interp::Linear);
        t.modifiers.push(ModSlot {
            modifier: Modifier::Loop,
            enabled: true,
        });
        assert!((t.value_at(1.5) - 0.5).abs() < 1e-4, "ripete: {}", t.value_at(1.5));
        t.modifiers[0].modifier = Modifier::PingPong;
        assert!((t.value_at(1.25) - 0.75).abs() < 1e-4, "torna indietro");
        t.modifiers[0].modifier = Modifier::Ramp { per_second: 10.0 };
        assert!((t.value_at(2.0) - 21.0).abs() < 1e-4);
        t.modifiers[0].enabled = false;
        assert!((t.value_at(2.0) - 1.0).abs() < 1e-4, "spento non conta");
        let mut w = Track {
            base: 3.0,
            ..Default::default()
        };
        w.modifiers.push(ModSlot {
            modifier: Modifier::Wiggle {
                amplitude: 0.5,
                frequency: 3.0,
                seed: 2,
            },
            enabled: true,
        });
        assert!(w.is_animated());
        let vals: Vec<f32> = (0..50).map(|k| w.value_at(k as f32 * 0.1)).collect();
        assert!(vals.iter().all(|v| (v - 3.0).abs() <= 0.5 + 1e-4));
        assert!(vals.iter().any(|v| (v - 3.0).abs() > 0.05), "si muove");
    }

    #[test]
    fn apply_moves_objects_and_records_user_changes() {
        let mut s = Scene::test_scene();
        let id = s.nodes[s.roots[0]].id.clone();
        let mut tl = Timeline::default();
        tl.add_element(&s, Target::Object(id.clone()));
        let k = tl.track(&Target::Object(id.clone()), Prop::PosX).unwrap();
        tl.tracks[k].keys = vec![
            Key {
                t: 0.0,
                v: 0.0,
                interp: Interp::Linear,
            },
            Key {
                t: 2.0,
                v: 1.0,
                interp: Interp::Linear,
            },
        ];
        tl.time = 1.0;
        let a = tl.apply(&mut s);
        assert!(a.transforms && !a.geometry);
        let i = s.find(&id).unwrap();
        assert!((s.nodes[i].user.translation.x - 0.5).abs() < 1e-5);
        // L'utente sposta l'oggetto a t = 1: diventa una chiave.
        s.nodes[i].user.translation.x = 0.8;
        tl.apply(&mut s);
        assert_eq!(tl.tracks[k].keys.len(), 3);
        assert!((tl.tracks[k].keys_at(1.0) - 0.8).abs() < 1e-5);
        // Opacità: materiale proprio con l'opacità scelta.
        let o = tl.track(&Target::Object(id.clone()), Prop::Opacity).unwrap();
        tl.tracks[o].keys = vec![Key {
            t: 0.0,
            v: 0.3,
            interp: Interp::Linear,
        }];
        tl.apply(&mut s);
        let m = s.materials.get(s.resolve_material(i)).unwrap();
        assert!((m.opacity - 0.3).abs() < 1e-5);
    }

    #[test]
    fn work_area_limits_the_saved_frames() {
        let mut tl = Timeline {
            duration: 4.0,
            fps: 10.0,
            ..Default::default()
        };
        assert_eq!(tl.work_area(), (0.0, 4.0));
        assert_eq!(tl.work_frames().len(), 40);
        tl.work_in = 1.0;
        tl.work_out = 2.5;
        assert_eq!(tl.work_area(), (1.0, 2.5));
        let f = tl.work_frames();
        assert_eq!(f.len(), 15);
        assert!((f[0] - 1.0).abs() < 1e-5);
        tl.duration = 2.0;
        assert_eq!(tl.work_area(), (1.0, 2.0), "la durata accorciata la taglia");
    }

    /// Un valore che la scena limita non si riscrive a ogni fotogramma: la scena non cambia più e
    /// la geometria non si ricostruisce all'infinito.
    #[test]
    fn a_clamped_value_settles() {
        let mut s = Scene::test_scene();
        let id = s.nodes[s.roots[0]].id.clone();
        let t = Target::Object(id);
        let mut tl = Timeline::default();
        tl.add_element(&s, t.clone());
        for (prop, v) in [(Prop::Scale, 0.0), (Prop::Opacity, 1.5)] {
            let i = tl.track(&t, prop).unwrap();
            tl.tracks[i].keys = vec![Key {
                t: 0.0,
                v,
                interp: Interp::Linear,
            }];
        }
        tl.apply(&mut s);
        for _ in 0..3 {
            assert!(!tl.apply(&mut s).any(), "niente da rifare a tempo fermo");
        }
    }

    #[test]
    fn rows_join_the_axes() {
        let rows = Target::Object(ComponentId("a".into())).rows();
        let pos = rows.iter().find(|r| r.0 == "Posizione").expect("riga");
        assert_eq!(pos.1, vec![Prop::PosX, Prop::PosY, Prop::PosZ]);
        assert_eq!(rows.iter().filter(|r| r.1.contains(&Prop::PosY)).count(), 1);
        assert!(rows.iter().any(|r| r.0 == "Rotazione"));
        assert!(rows.iter().any(|r| r.1 == vec![Prop::Opacity]));
    }

    /// Spostando un oggetto solo in X, anche Y e Z prendono la chiave: la riga è una.
    #[test]
    fn a_recorded_key_goes_on_every_axis_of_the_row() {
        let mut s = Scene::test_scene();
        let id = s.nodes[s.roots[0]].id.clone();
        let t = Target::Object(id.clone());
        let mut tl = Timeline::default();
        tl.add_element(&s, t.clone());
        let k = |t: f32, v: f32| Key {
            t,
            v,
            interp: Interp::Linear,
        };
        let x = tl.track(&t, Prop::PosX).unwrap();
        let y = tl.track(&t, Prop::PosY).unwrap();
        tl.tracks[x].keys = vec![k(0.0, 0.0), k(2.0, 1.0)];
        tl.tracks[y].keys = vec![k(0.0, 0.0), k(2.0, 2.0)];
        tl.time = 1.0;
        tl.apply(&mut s);
        let i = s.find(&id).unwrap();
        s.nodes[i].user.translation.x = 0.9;
        tl.apply(&mut s);
        assert_eq!(tl.tracks[x].keys.len(), 3);
        assert_eq!(tl.tracks[y].keys.len(), 3);
        assert!((tl.tracks[y].keys_at(1.0) - 1.0).abs() < 1e-4, "Y resta dov'era");
    }

    #[test]
    fn selected_keys_move_together_and_stay_inside() {
        let s = Scene::default();
        let mut tl = Timeline::default();
        let k = |t: f32, v: f32| Key {
            t,
            v,
            interp: Interp::Linear,
        };
        tl.tracks.push(Track {
            keys: vec![k(1.0, 1.0), k(2.0, 2.0), k(3.0, 3.0)],
            ..Default::default()
        });
        tl.duration = 4.0;
        let mut sel = vec![(0, 1.0), (0, 2.0)];
        assert!((tl.shift_keys(&mut sel, 0.5) - 0.5).abs() < 1e-5);
        assert_eq!(sel, vec![(0, 1.5), (0, 2.5)]);
        // Non escono dall'inizio: si fermano con la prima a zero.
        assert!((tl.shift_keys(&mut sel, -5.0) + 1.5).abs() < 1e-5);
        assert_eq!(tl.tracks[0].keys[0].t, 0.0);
        // Sposta sopra la chiave a 3 s: ne resta una, quella selezionata.
        let mut one = vec![(0, 1.0)];
        let before = tl.copy_keys(&one);
        tl.shift_keys(&mut one, 2.0);
        tl.settle_keys(&before);
        assert_eq!(tl.tracks[0].keys.len(), 2);
        assert_eq!(tl.tracks[0].keys.last().unwrap().v, 2.0);
        tl.set_keys_interp(&one, Interp::Hold);
        assert_eq!(tl.tracks[0].keys[1].interp, Interp::Hold);
        let copied = tl.copy_keys(&one);
        let pasted = tl.paste_keys(&copied, 3.5);
        assert_eq!(pasted.len(), 1);
        assert_eq!(tl.tracks[0].keys.len(), 3);
        tl.delete_keys(&s, &pasted);
        tl.delete_keys(&s, &one);
        assert_eq!(tl.tracks[0].keys.len(), 1);
    }

    #[test]
    fn effects_follow_the_playhead() {
        let mut s = Scene::default();
        s.smokes.push(crate::SmokeCloud {
            mode: crate::SmokeMode::Simulated,
            ..Default::default()
        });
        let mut tl = Timeline::default();
        tl.set_start(&Target::Smoke(1), 1.0);
        tl.time = 3.0;
        assert!(tl.apply(&mut s).effects);
        assert!((s.smokes[0].sim.time - 2.0).abs() < 1e-5);
        tl.time = 0.5;
        tl.apply(&mut s);
        assert_eq!(s.smokes[0].sim.time, MIN_EFFECT_TIME);
    }

    #[test]
    fn a_new_effect_gets_its_row_and_respects_the_animation() {
        let mut s = Scene::default();
        s.smokes.push(crate::SmokeCloud {
            mode: crate::SmokeMode::Simulated,
            ..Default::default()
        });
        s.smokes[0].sim.time = 3.0;
        let target = Target::Smoke(s.smokes[0].n);
        // Senza animazione il cursore va sull'istante dell'effetto.
        let mut tl = Timeline::default();
        tl.adopt_effects(&s);
        assert!(tl.elements.contains(&target));
        assert_eq!(tl.time, 3.0);
        tl.adopt_effects(&s);
        assert_eq!(tl.elements.iter().filter(|e| **e == target).count(), 1);
        // Con un'animazione il cursore resta dov'è.
        let mut tl = Timeline::default();
        tl.tracks.push(Track {
            keys: vec![Key {
                t: 1.0,
                v: 1.0,
                interp: Interp::Linear,
            }],
            ..Default::default()
        });
        tl.time = 1.0;
        tl.adopt_effects(&s);
        assert_eq!(tl.time, 1.0);
    }
}
