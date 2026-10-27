//! Materiali PBR del renderer, indipendenti dal file CAD di origine.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Identificatore stabile di un materiale (stringa, salvata nel progetto).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MaterialId(pub String);

impl MaterialId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl std::fmt::Display for MaterialId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Materiale fisicamente plausibile in stile "metallic/roughness".
///
/// Modello implementato dal backend CPU (Milestone 1):
/// - `metallic` interpola tra dielettrico (diffuso + speculare GGX) e conduttore (GGX con F0 = base_color);
/// - `transmission` interpola verso un dielettrico trasparente liscio (vetro), con Fresnel esatto e `ior`;
/// - `emission` è radianza emessa (W·sr⁻¹·m⁻²) moltiplicata per `emission_strength`.
///
/// Texture opzionali: colore base (sRGB), roughness (canale R, lineare) e
/// normal map (tangent space, convenzione OpenGL / +Y), con mappatura UV o box.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Material {
    pub id: MaterialId,
    pub name: String,
    /// Colore base lineare (non sRGB).
    pub base_color: [f32; 3],
    pub metallic: f32,
    pub roughness: f32,
    pub ior: f32,
    /// Scala del riflesso speculare dei dielettrici (0.5 = F0 derivato da `ior`).
    pub specular: f32,
    pub transmission: f32,
    pub emission: [f32; 3],
    pub emission_strength: f32,
    pub base_color_texture: Option<PathBuf>,
    pub roughness_texture: Option<PathBuf>,
    pub normal_texture: Option<PathBuf>,
    pub normal_strength: f32,
    pub mapping: TextureMapping,
    /// Categoria per la libreria (solo UI).
    pub category: String,
}

/// Come si proiettano le texture sulla superficie.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Projection {
    /// Coordinate UV della mesh (OBJ con `vt`). Senza UV si ricade sulla proiezione box.
    Uv,
    /// Proiezione cubica in spazio mondo lungo l'asse dominante della normale:
    /// adatta ai modelli CAD, che di solito non hanno UV.
    #[default]
    Box,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TextureMapping {
    pub projection: Projection,
    /// Box: lato di una ripetizione della texture in metri. UV: numero di ripetizioni.
    pub scale: f32,
    pub rotation_deg: f32,
    pub offset: [f32; 2],
}

impl Default for TextureMapping {
    fn default() -> Self {
        Self {
            projection: Projection::Box,
            scale: 0.1,
            rotation_deg: 0.0,
            offset: [0.0; 2],
        }
    }
}

impl TextureMapping {
    /// Trasforma le coordinate di base (UV della mesh o proiezione box già in metri).
    pub fn apply(&self, uv: forge_math::Vec2, is_box: bool) -> forge_math::Vec2 {
        let s = if is_box { 1.0 / self.scale.max(1e-6) } else { self.scale };
        let (sn, cs) = self.rotation_deg.to_radians().sin_cos();
        let p = uv * s;
        forge_math::Vec2::new(p.x * cs - p.y * sn, p.x * sn + p.y * cs) + forge_math::Vec2::from_array(self.offset)
    }

    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

impl Default for Material {
    fn default() -> Self {
        Self {
            id: MaterialId::new("default"),
            name: "Predefinito".into(),
            base_color: [0.8, 0.8, 0.8],
            metallic: 0.0,
            roughness: 0.5,
            ior: 1.5,
            specular: 0.5,
            transmission: 0.0,
            emission: [0.0; 3],
            emission_strength: 0.0,
            base_color_texture: None,
            roughness_texture: None,
            normal_texture: None,
            normal_strength: 1.0,
            mapping: TextureMapping::default(),
            category: "Generico".into(),
        }
    }
}

impl Material {
    /// Riporta i parametri negli intervalli validi.
    pub fn sanitized(&self) -> Material {
        let mut m = self.clone();
        let c01 = |v: f32| if v.is_finite() { v.clamp(0.0, 1.0) } else { 0.0 };
        m.base_color = m.base_color.map(c01);
        m.metallic = c01(m.metallic);
        m.roughness = c01(m.roughness);
        m.specular = c01(m.specular);
        m.transmission = c01(m.transmission);
        m.ior = if m.ior.is_finite() { m.ior.clamp(1.0, 3.0) } else { 1.5 };
        m.emission = m.emission.map(|v| if v.is_finite() { v.max(0.0) } else { 0.0 });
        m.emission_strength = if m.emission_strength.is_finite() {
            m.emission_strength.max(0.0)
        } else {
            0.0
        };
        m.normal_strength = if m.normal_strength.is_finite() {
            m.normal_strength.clamp(0.0, 4.0)
        } else {
            1.0
        };
        if !m.mapping.scale.is_finite() || m.mapping.scale <= 0.0 {
            m.mapping.scale = TextureMapping::default().scale;
        }
        m
    }

    pub fn has_textures(&self) -> bool {
        self.base_color_texture.is_some() || self.roughness_texture.is_some() || self.normal_texture.is_some()
    }

    pub fn is_emissive(&self) -> bool {
        self.emission_strength > 0.0 && self.emission.iter().any(|&c| c > 0.0)
    }
}

/// Converte un colore sRGB (0..1) in lineare.
pub fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

pub fn linear_to_srgb(c: f32) -> f32 {
    let c = c.max(0.0);
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// Raccolta dei materiali disponibili nel progetto.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MaterialLibrary {
    pub materials: Vec<Material>,
}

impl MaterialLibrary {
    pub fn with_presets() -> Self {
        let mut lib = Self { materials: presets() };
        lib.ensure_default();
        lib
    }

    pub fn ensure_default(&mut self) {
        if self.get(&MaterialId::new("default")).is_none() {
            self.materials.insert(0, Material::default());
        }
    }

    pub fn get(&self, id: &MaterialId) -> Option<&Material> {
        self.materials.iter().find(|m| &m.id == id)
    }

    pub fn get_mut(&mut self, id: &MaterialId) -> Option<&mut Material> {
        self.materials.iter_mut().find(|m| &m.id == id)
    }

    pub fn index_of(&self, id: &MaterialId) -> Option<usize> {
        self.materials.iter().position(|m| &m.id == id)
    }

    /// Aggiunge o sostituisce un materiale con lo stesso id.
    pub fn upsert(&mut self, m: Material) {
        if let Some(existing) = self.get_mut(&m.id) {
            *existing = m;
        } else {
            self.materials.push(m);
        }
    }

    /// Genera un id univoco a partire da una base.
    pub fn unique_id(&self, base: &str) -> MaterialId {
        let mut candidate = MaterialId::new(base);
        let mut n = 1;
        while self.get(&candidate).is_some() {
            n += 1;
            candidate = MaterialId::new(format!("{base}-{n}"));
        }
        candidate
    }
}

fn srgb(r: u8, g: u8, b: u8) -> [f32; 3] {
    [r, g, b].map(|v| srgb_to_linear(v as f32 / 255.0))
}

/// Libreria iniziale. I valori sono punti di partenza ragionevoli, non misure
/// di laboratorio: un singolo valore di roughness non descrive ogni finitura reale.
/// Colori dei metalli: F0 approssimati da dati spettrali noti (es. "Real-Time Rendering", 4ª ed.).
pub fn presets() -> Vec<Material> {
    let m = |id: &str, name: &str, cat: &str| Material {
        id: MaterialId::new(id),
        name: name.into(),
        category: cat.into(),
        ..Default::default()
    };
    vec![
        Material {
            base_color: [0.56, 0.57, 0.58],
            metallic: 1.0,
            roughness: 0.35,
            ..m("acciaio-satinato", "Acciaio satinato", "Metalli")
        },
        Material {
            base_color: [0.56, 0.57, 0.58],
            metallic: 1.0,
            roughness: 0.06,
            ..m("acciaio-lucidato", "Acciaio lucidato", "Metalli")
        },
        Material {
            base_color: [0.91, 0.92, 0.92],
            metallic: 1.0,
            roughness: 0.25,
            ..m("alluminio", "Alluminio", "Metalli")
        },
        Material {
            base_color: srgb(40, 70, 140),
            metallic: 1.0,
            roughness: 0.3,
            ..m("alluminio-anodizzato-blu", "Alluminio anodizzato (blu)", "Metalli")
        },
        Material {
            base_color: [0.62, 0.64, 0.65],
            metallic: 1.0,
            roughness: 0.55,
            ..m("acciaio-zincato", "Acciaio zincato", "Metalli")
        },
        Material {
            base_color: [0.95, 0.64, 0.54],
            metallic: 1.0,
            roughness: 0.2,
            ..m("rame", "Rame", "Metalli")
        },
        Material {
            base_color: [0.91, 0.78, 0.42],
            metallic: 1.0,
            roughness: 0.2,
            ..m("ottone", "Ottone", "Metalli")
        },
        Material {
            base_color: srgb(30, 30, 32),
            roughness: 0.6,
            ..m("plastica-opaca-nera", "Plastica opaca (nera)", "Plastiche")
        },
        Material {
            base_color: srgb(180, 20, 20),
            roughness: 0.12,
            ..m("plastica-lucida-rossa", "Plastica lucida (rossa)", "Plastiche")
        },
        Material {
            base_color: srgb(15, 15, 15),
            roughness: 0.85,
            specular: 0.3,
            ..m("gomma", "Gomma", "Plastiche")
        },
        Material {
            base_color: [1.0, 1.0, 1.0],
            roughness: 0.0,
            transmission: 1.0,
            ior: 1.5,
            ..m("vetro", "Vetro", "Trasparenti")
        },
        Material {
            base_color: srgb(240, 240, 235),
            roughness: 0.08,
            specular: 0.5,
            ..m("ceramica", "Ceramica smaltata", "Ceramiche")
        },
        Material {
            base_color: srgb(140, 90, 50),
            roughness: 0.55,
            ..m("legno", "Legno (tinta unita, texture in Fase 3)", "Naturali")
        },
        Material {
            base_color: srgb(20, 60, 120),
            roughness: 0.25,
            ..m("vernice", "Vernice", "Vernici")
        },
        Material {
            base_color: [1.0, 1.0, 1.0],
            emission: [1.0, 1.0, 1.0],
            emission_strength: 10.0,
            roughness: 1.0,
            ..m("emissivo", "Superficie emissiva", "Emissivi")
        },
    ]
}
