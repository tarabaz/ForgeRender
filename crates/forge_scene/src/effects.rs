//! Effetti costruiti come geometria: adesivi (decal) e polvere in sospensione.
//!
//! - **Adesivo**: un'immagine proiettata su un oggetto. La pellicola si ricava dai triangoli
//!   dell'oggetto dentro la scatola di proiezione (ritagliati sui suoi bordi), staccata di un
//!   soffio dalla superficie: segue anche le superfici curve (bottiglie, tazze). Il nodo è figlio
//!   dell'oggetto, quindi lo segue quando lo si sposta. Il materiale usa l'immagine come colore e
//!   la sua trasparenza come ritaglio.
//! - **Polvere**: tanti granelli minuscoli sparsi nell'aria, quasi invisibili al buio e brillanti
//!   dove li attraversa un fascio di luce (come in soffitta controluce).
//!
//! Il progetto salva solo i parametri: i nodi si ricreano con [`Scene::sync_effects`].

use crate::material::{Material, MaterialId, Projection, TextureMapping};
use crate::mesh::Mesh;
use crate::scene::{ComponentId, NodeIndex, Scene};
use forge_math::{Mat4, Vec2, Vec3};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Prefisso degli id dei nodi degli adesivi.
pub const DECAL_PREFIX: &str = "adesivo:";
/// Id del nodo della polvere in sospensione.
pub const DUST_ID: &str = "effetto:polvere";
/// Id del nodo (e del materiale) dei granelli del secondo colore.
pub const DUST_ID_2: &str = "effetto:polvere:2";

/// È l'id di un nodo o di un materiale della polvere in sospensione illuminata dai fasci (non dei
/// granelli luminosi, che sono geometria normale che emette luce).
pub fn is_dust_id(id: &str) -> bool {
    id == DUST_ID || id == DUST_ID_2
}

/// Prefisso degli id dei nodi (e dei materiali) dei granelli luminosi: `<prefisso><n>`, n = 1, 2...
pub const DUST_GLOW_PREFIX: &str = "effetto:polvere:luce:";

/// Id dei granelli luminosi del colore `k` (0 = il primo).
pub fn dust_glow_id(k: usize) -> String {
    format!("{DUST_GLOW_PREFIX}{}", k + 1)
}

/// È l'id di qualsiasi nodo o materiale della polvere (illuminata o luminosa).
pub fn is_any_dust_id(id: &str) -> bool {
    is_dust_id(id) || id.starts_with(DUST_GLOW_PREFIX)
}

/// Un colore di granelli luminosi: emettono luce propria, si vedono anche fuori dai fasci.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DustGlow {
    /// Colore della luce emessa (0–1).
    pub color: [f32; 3],
    /// Intensità della luce emessa (radianza, come l'emissione dei materiali).
    pub intensity: f32,
    /// Parte dei granelli che si accendono di questo colore (0–1).
    pub share: f32,
}

impl Default for DustGlow {
    fn default() -> Self {
        Self {
            color: [1.0, 0.78, 0.4],
            // Più alta il colore satura al bianco (come una lampadina vista dritta).
            intensity: 3.0,
            share: 0.05,
        }
    }
}

/// Forma dei granelli di polvere.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DustShape {
    /// Granelli tondi: diffondono la luce uguale da ogni lato.
    #[default]
    Grains,
    /// Scaglie piatte orientate a caso (polvere di fata, brillantini): riflettono come piccoli
    /// specchi, quindi brillano solo quelle girate giuste tra luce e camera.
    Flakes,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Decal {
    /// Numero univoco nella scena (id del nodo `adesivo:<n>`).
    pub n: u32,
    pub name: String,
    pub enabled: bool,
    /// Oggetto su cui è applicato.
    pub target: ComponentId,
    /// Immagine (PNG con trasparenza, JPG...).
    pub image: Option<PathBuf>,
    /// Centro dell'adesivo, nello spazio della mesh dell'oggetto.
    pub center: Vec3,
    /// Direzione verso cui guarda l'adesivo (uscente dalla superficie), spazio della mesh.
    pub normal: Vec3,
    /// Verso "su" dell'immagine (spazio della mesh), prima della rotazione.
    pub up: Vec3,
    /// Larghezza e altezza in metri.
    pub width: f32,
    pub height: f32,
    pub rotation_deg: f32,
    /// Spostamento sulla superficie (metri, lungo larghezza e altezza dell'adesivo).
    pub offset: [f32; 2],
    /// Profondità della proiezione (metri, per parte): quanto la superficie può essere curva.
    pub depth: f32,
    pub roughness: f32,
    pub metallic: f32,
}

impl Default for Decal {
    fn default() -> Self {
        Self {
            n: 1,
            name: "Adesivo".into(),
            enabled: true,
            target: ComponentId(String::new()),
            image: None,
            center: Vec3::ZERO,
            normal: Vec3::Z,
            up: Vec3::Y,
            width: 0.08,
            height: 0.08,
            rotation_deg: 0.0,
            offset: [0.0; 2],
            depth: 0.03,
            roughness: 0.35,
            metallic: 0.0,
        }
    }
}

impl Decal {
    pub fn node_id(&self) -> ComponentId {
        ComponentId(format!("{DECAL_PREFIX}{}", self.n))
    }

    pub fn material_id(&self) -> MaterialId {
        MaterialId::new(format!("{DECAL_PREFIX}{}", self.n))
    }

    /// Assi dell'adesivo (destra, su, fuori) nello spazio della mesh, con la rotazione.
    fn axes(&self) -> (Vec3, Vec3, Vec3) {
        let n = self.normal.normalize_or(Vec3::Z);
        let mut up = self.up - n * self.up.dot(n);
        if up.length_squared() < 1e-8 {
            up = if n.y.abs() < 0.9 { Vec3::Y } else { Vec3::Z };
            up -= n * up.dot(n);
        }
        let up = up.normalize();
        let right = up.cross(n).normalize();
        let (s, c) = self.rotation_deg.to_radians().sin_cos();
        (right * c + up * s, up * c - right * s, n)
    }

    /// Pellicola sui triangoli di `mesh` (spazio della mesh); `scale` = metri per unità della
    /// mesh. Vuota se l'adesivo non tocca l'oggetto.
    pub fn build_mesh(&self, mesh: &Mesh, scale: f32) -> Mesh {
        let s = scale.max(1e-9);
        let (right, up, n) = self.axes();
        let hw = (self.width * 0.5 / s).max(1e-9);
        let hh = (self.height * 0.5 / s).max(1e-9);
        let hd = (self.depth / s).max(1e-9);
        let c = self.center + right * (self.offset[0] / s) + up * (self.offset[1] / s);
        // Coordinate dell'adesivo: x, y in [−1, 1] sull'immagine, z in [−1, 1] in profondità.
        let to_local = |p: Vec3| {
            let d = p - c;
            Vec3::new(d.dot(right) / hw, d.dot(up) / hh, d.dot(n) / hd)
        };
        let lift = 0.0003 / s;
        let mut out = Mesh {
            name: self.name.clone(),
            ..Default::default()
        };
        let has_normals = mesh.has_normals();
        for tri in &mesh.indices {
            let p = tri.map(|i| mesh.positions[i as usize]);
            let gn = (p[1] - p[0]).cross(p[2] - p[0]);
            if gn.length_squared() < 1e-30 || gn.normalize().dot(n) < 0.05 {
                continue;
            }
            let l = p.map(to_local);
            if l.iter().all(|v| v.x > 1.0)
                || l.iter().all(|v| v.x < -1.0)
                || l.iter().all(|v| v.y > 1.0)
                || l.iter().all(|v| v.y < -1.0)
                || l.iter().all(|v| v.z > 1.0)
                || l.iter().all(|v| v.z < -1.0)
            {
                continue;
            }
            let nn = if has_normals {
                tri.map(|i| mesh.normals[i as usize])
            } else {
                [gn.normalize(); 3]
            };
            // Poligono (posizione, normale, coordinate dell'adesivo) ritagliato sui 6 piani.
            let mut poly: Vec<(Vec3, Vec3, Vec3)> = (0..3).map(|k| (p[k], nn[k], l[k])).collect();
            for axis in 0..3 {
                for sign in [1.0f32, -1.0] {
                    poly = clip(&poly, |v| 1.0 - sign * v[axis]);
                    if poly.len() < 3 {
                        break;
                    }
                }
                if poly.len() < 3 {
                    break;
                }
            }
            if poly.len() < 3 {
                continue;
            }
            let base = out.positions.len() as u32;
            for (pos, nor, loc) in &poly {
                let nor = nor.normalize_or(gn.normalize());
                out.positions.push(*pos + nor * lift);
                out.normals.push(nor);
                out.uvs.push(Vec2::new((loc.x + 1.0) * 0.5, (loc.y + 1.0) * 0.5));
            }
            for k in 1..poly.len() as u32 - 1 {
                out.indices.push([base, base + k, base + k + 1]);
            }
        }
        out
    }

    /// Materiale dell'adesivo: l'immagine come colore e come ritaglio (la sua trasparenza).
    pub fn material(&self) -> Material {
        Material {
            id: self.material_id(),
            name: self.name.clone(),
            base_color: [1.0; 3],
            roughness: self.roughness,
            metallic: self.metallic,
            base_color_texture: self.image.clone(),
            opacity_texture: self.image.clone(),
            mapping: TextureMapping {
                projection: Projection::Uv,
                scale: 1.0,
                ..Default::default()
            },
            category: "Adesivi".into(),
            ..Material::default()
        }
    }
}

/// Sutherland–Hodgman: tiene la parte con `dist(coordinate) >= 0`.
fn clip(poly: &[(Vec3, Vec3, Vec3)], dist: impl Fn(Vec3) -> f32) -> Vec<(Vec3, Vec3, Vec3)> {
    let mut out = Vec::with_capacity(poly.len() + 2);
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        let (da, db) = (dist(a.2), dist(b.2));
        if da >= 0.0 {
            out.push(a);
        }
        if (da >= 0.0) != (db >= 0.0) {
            let t = da / (da - db);
            out.push((a.0.lerp(b.0, t), a.1.lerp(b.1, t), a.2.lerp(b.2, t)));
        }
    }
    out
}

/// Scaglia: esagono piatto di diametro 1 nel piano xy (4 triangoli), centrato nell'origine.
fn flake() -> Mesh {
    let positions: Vec<Vec3> = (0..6)
        .map(|k| {
            let a = k as f32 * std::f32::consts::TAU / 6.0;
            Vec3::new(a.cos() * 0.5, a.sin() * 0.5, 0.0)
        })
        .collect();
    Mesh {
        name: "Scaglia".into(),
        normals: vec![Vec3::Z; 6],
        positions,
        uvs: Vec::new(),
        indices: vec![[0, 1, 2], [0, 2, 3], [0, 3, 4], [0, 4, 5]],
    }
}

/// Granello: icosaedro di diametro 1 (20 triangoli), centrato nell'origine.
fn grain() -> Mesh {
    let t = (1.0 + 5f32.sqrt()) * 0.5;
    let v = [
        (-1.0, t, 0.0),
        (1.0, t, 0.0),
        (-1.0, -t, 0.0),
        (1.0, -t, 0.0),
        (0.0, -1.0, t),
        (0.0, 1.0, t),
        (0.0, -1.0, -t),
        (0.0, 1.0, -t),
        (t, 0.0, -1.0),
        (t, 0.0, 1.0),
        (-t, 0.0, -1.0),
        (-t, 0.0, 1.0),
    ];
    let normals: Vec<Vec3> = v.iter().map(|&(x, y, z)| Vec3::new(x, y, z).normalize()).collect();
    Mesh {
        name: "Granello".into(),
        positions: normals.iter().map(|n| *n * 0.5).collect(),
        normals,
        uvs: Vec::new(),
        indices: vec![
            [0, 11, 5],
            [0, 5, 1],
            [0, 1, 7],
            [0, 7, 10],
            [0, 10, 11],
            [1, 5, 9],
            [5, 11, 4],
            [11, 10, 2],
            [10, 7, 6],
            [7, 1, 8],
            [3, 9, 4],
            [3, 4, 2],
            [3, 2, 6],
            [3, 6, 8],
            [3, 8, 9],
            [4, 9, 5],
            [2, 4, 11],
            [6, 2, 10],
            [8, 6, 7],
            [9, 8, 1],
        ],
    }
}

/// Polvere in sospensione nella foschia.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Dust {
    pub enabled: bool,
    /// Granelli per metro cubo.
    pub per_m3: f32,
    /// Diametro medio dei granelli in millimetri (variano da metà al doppio).
    pub size_mm: f32,
    /// Colore dei granelli (0–1).
    pub color: [f32; 3],
    pub seed: u32,
    /// Brillantezza dei granelli nel fascio (moltiplica la luce che diffondono): i granelli
    /// veri sono più piccoli di un pixel e nelle foto si vedono come puntini luminosi sfocati.
    pub brightness: f32,
    /// I granelli fanno ombra (piccole sagome scure dentro i fasci e sul piano illuminato).
    pub shadows: bool,
    /// Riquadro in cui stanno i granelli (centro e dimensioni, m). `None` = automatico attorno
    /// alla scena (i progetti di prima); si sposta e si ridimensiona con il gizmo.
    pub region: Option<(Vec3, Vec3)>,
    /// Secondo colore dei granelli (0–1).
    pub color2: [f32; 3],
    /// Parte dei granelli con il secondo colore (0 = tutti del primo).
    pub color2_share: f32,
    /// Opacità dei granelli (0–1): quanta luce diffondono e quanta ne fermano per l'ombra.
    pub opacity: f32,
    /// Quanto la dimensione dei granelli si scosta da quella impostata (0 = tutti uguali, 0,5 =
    /// da metà al doppio come nei progetti di prima, 1 = da quasi zero al triplo).
    pub size_spread: f32,
    /// Granelli tondi o scaglie piatte che luccicano.
    pub shape: DustShape,
    /// Scaglie: quanta luce riflettono a specchio invece che diffusa (0–1).
    pub sparkle: f32,
    /// Scaglie: ruvidità del riflesso (bassa = scintille piccole e intense, alta = più scaglie
    /// accese ma più tenui).
    pub flake_roughness: f32,
    /// Granelli luminosi: ogni colore prende la sua parte di granelli, che emettono luce.
    pub glow: Vec<DustGlow>,
    /// Alone dei granelli luminosi (lucciole): grandezza in diametri del granello (0 = niente).
    pub halo_size: f32,
    /// Forza dell'alone (0 = niente, 1 = normale).
    pub halo_strength: f32,
}

impl Default for Dust {
    fn default() -> Self {
        Self {
            enabled: false,
            per_m3: 1500.0,
            size_mm: 2.5,
            color: [0.85, 0.8, 0.72],
            seed: 1,
            brightness: 12.0,
            shadows: false,
            region: None,
            color2: [1.0, 0.82, 0.45],
            color2_share: 0.0,
            opacity: 1.0,
            size_spread: 0.5,
            shape: DustShape::Grains,
            sparkle: 0.8,
            flake_roughness: 0.08,
            glow: Vec::new(),
            halo_size: 6.0,
            halo_strength: 1.0,
        }
    }
}

/// Numero massimo di granelli (≈ 20 triangoli l'uno): oltre la polvere si dirada.
pub const DUST_MAX: usize = 150_000;

impl Dust {
    /// Granelli sparsi nel riquadro [min, max] (spazio mondo), fuori dagli oggetti grossolanamente
    /// non controllato: sono così piccoli che dentro un oggetto non si vedono. Restituisce i
    /// granelli del primo colore e quelli del secondo.
    pub fn build_meshes(&self, min: Vec3, max: Vec3) -> (Mesh, Mesh) {
        let (a, b, _) = self.build_groups(min, max);
        (a, b)
    }

    /// Come [`Dust::build_meshes`], e in più i granelli luminosi di ogni colore di [`Dust::glow`].
    pub fn build_groups(&self, min: Vec3, max: Vec3) -> (Mesh, Mesh, Vec<Mesh>) {
        let ext = (max - min).max(Vec3::ZERO);
        let volume = ext.x * ext.y * ext.z;
        let count = ((volume * self.per_m3.max(0.0)) as usize).min(DUST_MAX);
        let flakes = self.shape == DustShape::Flakes;
        let shape = if flakes { flake() } else { grain() };
        let new_mesh = |name: &str| {
            let mut m = Mesh {
                name: name.into(),
                ..Default::default()
            };
            m.positions.reserve(count * shape.positions.len());
            m
        };
        let mut first = new_mesh("Polvere in sospensione");
        let mut second = new_mesh("Polvere in sospensione (secondo colore)");
        let mut glows: Vec<Mesh> = (0..self.glow.len())
            .map(|k| new_mesh(&format!("Granelli luminosi {}", k + 1)))
            .collect();
        // Parti dei colori luminosi (se insieme superano il 100 % si riducono in proporzione).
        let shares: Vec<f32> = self
            .glow
            .iter()
            .map(|g| if g.share.is_finite() { g.share.clamp(0.0, 1.0) } else { 0.0 })
            .collect();
        let total: f32 = shares.iter().sum();
        let norm = if total > 1.0 { 1.0 / total } else { 1.0 };
        // Generatore a parte anche per i luminosi: gli altri granelli non si spostano.
        let mut glow_state = 0x6A09_E667_F3BC_C909u64 ^ (self.seed as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        let mut rnd3 = || {
            glow_state ^= glow_state << 13;
            glow_state ^= glow_state >> 7;
            glow_state ^= glow_state << 17;
            (glow_state >> 40) as f32 / (1u64 << 24) as f32
        };
        let mut state = 0x9E37_79B9_7F4A_7C15u64 ^ (self.seed as u64).wrapping_mul(0xD1B5_4A32_D192_ED03);
        let mut rnd = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 40) as f32 / (1u64 << 24) as f32
        };
        // Le scelte nuove (colore, orientamento delle scaglie) usano un generatore a parte: i
        // granelli dei progetti di prima restano dove erano.
        let mut extra = 0x2545_F491_4F6C_DD1Du64 ^ (self.seed as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let mut rnd2 = || {
            extra ^= extra << 13;
            extra ^= extra >> 7;
            extra ^= extra << 17;
            (extra >> 40) as f32 / (1u64 << 24) as f32
        };
        let spread = self.size_spread.clamp(0.0, 1.0);
        let share = self.color2_share.clamp(0.0, 1.0);
        for _ in 0..count {
            let c = min + Vec3::new(rnd(), rnd(), rnd()) * ext;
            // Fattore tra metà e il doppio (più granelli piccoli che grandi) con variazione 0,5,
            // stretto o allargato dalla variazione scelta.
            let f = 0.5 + 1.5 * rnd() * rnd();
            let f = (1.0 + (f - 1.0) * 2.0 * spread).max(0.05);
            let d = self.size_mm.max(0.01) * 1e-3 * f;
            let second_color = share > 0.0 && rnd2() < share;
            let mut glow_k = None;
            if !shares.is_empty() {
                let mut u = rnd3();
                for (k, s) in shares.iter().enumerate() {
                    let s = s * norm;
                    if u < s {
                        glow_k = Some(k);
                        break;
                    }
                    u -= s;
                }
            }
            let out = match glow_k {
                Some(k) => &mut glows[k],
                None if second_color => &mut second,
                None => &mut first,
            };
            let base = out.positions.len() as u32;
            if flakes {
                // Normale a caso sulla sfera e due assi nel piano della scaglia.
                let z = 2.0 * rnd2() - 1.0;
                let a = std::f32::consts::TAU * rnd2();
                let r = (1.0 - z * z).max(0.0).sqrt();
                let n = Vec3::new(r * a.cos(), r * a.sin(), z);
                let t = if n.x.abs() < 0.9 { Vec3::X } else { Vec3::Y }.cross(n).normalize();
                let b = n.cross(t);
                for p in &shape.positions {
                    out.positions.push(c + (t * p.x + b * p.y) * d);
                    out.normals.push(n);
                }
            } else {
                for (p, nrm) in shape.positions.iter().zip(&shape.normals) {
                    out.positions.push(c + *p * d);
                    out.normals.push(*nrm);
                }
            }
            out.indices.extend(shape.indices.iter().map(|t| t.map(|i| i + base)));
        }
        (first, second, glows)
    }

    /// Granelli del primo colore (vedi [`Dust::build_meshes`]).
    pub fn build_mesh(&self, min: Vec3, max: Vec3) -> Mesh {
        self.build_meshes(min, max).0
    }

    /// Materiale dei granelli del secondo colore.
    pub fn material2(&self) -> Material {
        Material {
            id: MaterialId::new(DUST_ID_2),
            name: "Polvere (secondo colore)".into(),
            base_color: self.color2,
            ..self.material()
        }
    }

    /// Materiale dei granelli luminosi del colore `k`.
    pub fn glow_material(&self, k: usize) -> Material {
        let g = self.glow.get(k).cloned().unwrap_or_default();
        Material {
            id: MaterialId::new(dust_glow_id(k)),
            name: format!("Polvere luminosa {}", k + 1),
            base_color: g.color,
            emission: g.color,
            emission_strength: g.intensity.max(0.0),
            roughness: 0.6,
            category: "Effetti".into(),
            ..Material::default()
        }
    }

    pub fn material(&self) -> Material {
        Material {
            id: MaterialId::new(DUST_ID),
            name: "Polvere".into(),
            base_color: self.color,
            roughness: 0.6,
            category: "Effetti".into(),
            ..Material::default()
        }
    }
}

impl Scene {
    /// Mette d'accordo i nodi degli effetti (adesivi, polvere) con i loro parametri: li crea,
    /// aggiorna la mesh e il materiale, toglie quelli che non ci sono più. Restituisce vero se la
    /// geometria è cambiata.
    pub fn sync_effects(&mut self) -> bool {
        let mut changed = false;
        // Adesivi spariti (o con l'oggetto eliminato): via il nodo e il materiale.
        let wanted: Vec<ComponentId> = self.decals.iter().map(|d| d.node_id()).collect();
        while let Some(i) = self
            .nodes
            .iter()
            .position(|n| n.id.0.starts_with(DECAL_PREFIX) && !wanted.contains(&n.id))
        {
            let id = self.nodes[i].id.clone();
            self.remove_subtree(i);
            self.assignments.remove(&id);
            self.materials.materials.retain(|m| m.id.0 != id.0);
            changed = true;
        }
        for k in 0..self.decals.len() {
            changed |= self.sync_decal(k);
        }
        changed |= self.sync_dust();
        changed
    }

    fn sync_decal(&mut self, k: usize) -> bool {
        let d = self.decals[k].clone();
        let id = d.node_id();
        let Some(target) = self.find(&d.target).filter(|&t| self.nodes[t].mesh.is_some()) else {
            return false;
        };
        let world = self.world_transform(target);
        let scale = forge_math::Mat3::from_mat4(world).determinant().abs().cbrt();
        let mesh = d.build_mesh(&self.meshes[self.nodes[target].mesh.unwrap()], scale);
        self.materials.upsert(d.material());
        self.assignments.insert(id.clone(), d.material_id());
        let node = match self.find(&id) {
            Some(i) if self.nodes[i].parent == Some(target) => i,
            Some(i) => {
                self.remove_subtree(i);
                return self.sync_decal(k);
            }
            None => {
                let m = self.add_mesh(Mesh::default());
                // L'indice dell'oggetto non cambia aggiungendo in coda.
                self.add_node(Some(target), id, d.name.clone(), Mat4::IDENTITY, Some(m))
            }
        };
        let changed = self.nodes[node].mesh.map(|m| *self.meshes[m] != mesh).unwrap_or(true);
        if let Some(m) = self.nodes[node].mesh {
            if changed {
                self.meshes[m] = std::sync::Arc::new(mesh);
            }
        }
        self.nodes[node].name = d.name.clone();
        let vis = d.enabled;
        let vis_changed = self.nodes[node].visible != vis;
        self.nodes[node].visible = vis;
        changed || vis_changed
    }

    fn sync_dust(&mut self) -> bool {
        let on = self.haze_dust.enabled;
        if !on {
            let mut changed = false;
            for id in [DUST_ID_2, DUST_ID] {
                if let Some(i) = self.find(&ComponentId(id.into())) {
                    self.remove_subtree(i);
                    changed = true;
                }
            }
            changed |= self.remove_dust_glow_from(0);
            return changed;
        }
        let (center, size) = match self.haze_dust.region {
            Some(r) => r,
            None => {
                // Riquadro automatico (progetti di prima): la scena senza la polvere, la camera
                // compresa, al massimo 6 m per lato.
                let dust_nodes: Vec<NodeIndex> = (0..self.nodes.len()).filter(|&i| is_any_dust_id(&self.nodes[i].id.0)).collect();
                for &i in &dust_nodes {
                    self.nodes[i].visible = false;
                }
                let mut b = self.bounds();
                for &i in &dust_nodes {
                    self.nodes[i].visible = true;
                }
                if b.is_empty() {
                    return false;
                }
                b.grow_point(self.render_camera().position);
                ((b.min + b.max) * 0.5, (b.max - b.min).min(Vec3::splat(6.0)))
            }
        };
        let half = size.max(Vec3::splat(0.01)) * 0.5;
        let (first, second, glows) = self.haze_dust.build_groups(center - half, center + half);
        let mut changed = self.sync_dust_node(DUST_ID, "Polvere in sospensione", first, self.haze_dust.material());
        if second.indices.is_empty() {
            if let Some(i) = self.find(&ComponentId(DUST_ID_2.into())) {
                self.remove_subtree(i);
                changed = true;
            }
        } else {
            changed |= self.sync_dust_node(
                DUST_ID_2,
                "Polvere in sospensione (secondo colore)",
                second,
                self.haze_dust.material2(),
            );
        }
        // Granelli luminosi: un nodo per colore (quelli senza granelli o tolti spariscono).
        let n_glow = glows.len();
        for (k, mesh) in glows.into_iter().enumerate() {
            let id = dust_glow_id(k);
            if mesh.indices.is_empty() {
                if let Some(i) = self.find(&ComponentId(id.clone())) {
                    self.remove_subtree(i);
                    changed = true;
                }
                continue;
            }
            let name = format!("Granelli luminosi {}", k + 1);
            changed |= self.sync_dust_node(&id, &name, mesh, self.haze_dust.glow_material(k));
        }
        changed |= self.remove_dust_glow_from(n_glow);
        changed
    }

    /// Toglie i nodi (e i materiali) dei granelli luminosi dal colore `from` in poi.
    fn remove_dust_glow_from(&mut self, from: usize) -> bool {
        let mut changed = false;
        while let Some(i) = self.nodes.iter().position(|n| {
            n.id.0
                .strip_prefix(DUST_GLOW_PREFIX)
                .and_then(|k| k.parse::<usize>().ok())
                .is_some_and(|k| k > from)
        }) {
            let id = self.nodes[i].id.clone();
            self.remove_subtree(i);
            self.assignments.remove(&id);
            self.materials.materials.retain(|m| m.id.0 != id.0);
            changed = true;
        }
        changed
    }

    /// Crea o aggiorna un nodo della polvere: vero se la mesh è cambiata.
    fn sync_dust_node(&mut self, id: &str, name: &str, mesh: Mesh, material: Material) -> bool {
        let cid = ComponentId(id.into());
        self.materials.upsert(material);
        self.assignments.insert(cid.clone(), MaterialId::new(id));
        match self.find(&cid) {
            Some(i) => {
                let m = self.nodes[i].mesh.unwrap_or_else(|| self.add_mesh(Mesh::default()));
                self.nodes[i].mesh = Some(m);
                if *self.meshes[m] == mesh {
                    return false;
                }
                self.meshes[m] = std::sync::Arc::new(mesh);
            }
            None => {
                let m = self.add_mesh(mesh);
                self.add_node(None, cid, name, Mat4::IDENTITY, Some(m));
            }
        }
        true
    }

    /// Primo punto colpito dal raggio (spazio mondo) sui componenti visibili di `among` e dei
    /// loro discendenti (tutti, se vuoto), esclusi gli effetti: (nodo, punto, normale verso il
    /// raggio).
    pub fn raycast(&self, among: &[NodeIndex], o: Vec3, d: Vec3) -> Option<(NodeIndex, Vec3, Vec3)> {
        let mut nodes = Vec::new();
        let mut stack: Vec<NodeIndex> = if among.is_empty() { self.roots.clone() } else { among.to_vec() };
        while let Some(i) = stack.pop() {
            if !nodes.contains(&i) {
                nodes.push(i);
                stack.extend(&self.nodes[i].children);
            }
        }
        let mut best: Option<(f32, NodeIndex, Vec3)> = None;
        for i in nodes {
            let Some(m) = self.nodes[i].mesh else { continue };
            if self.is_effect_node(i) || !self.is_visible(i) {
                continue;
            }
            let w = self.world_transform(i);
            let mesh = &self.meshes[m];
            let pts: Vec<Vec3> = mesh.positions.iter().map(|p| w.transform_point3(*p)).collect();
            for t in &mesh.indices {
                let [a, b, c] = t.map(|k| pts[k as usize]);
                let (e1, e2) = (b - a, c - a);
                let pv = d.cross(e2);
                let det = e1.dot(pv);
                if det.abs() < 1e-12 {
                    continue;
                }
                let inv = 1.0 / det;
                let tv = o - a;
                let u = tv.dot(pv) * inv;
                if !(0.0..=1.0).contains(&u) {
                    continue;
                }
                let q = tv.cross(e1);
                let v = d.dot(q) * inv;
                if v < 0.0 || u + v > 1.0 {
                    continue;
                }
                let dist = e2.dot(q) * inv;
                if dist > 1e-5 && best.is_none_or(|(bd, _, _)| dist < bd) {
                    let mut n = e1.cross(e2).normalize_or(Vec3::Y);
                    if n.dot(d) > 0.0 {
                        n = -n;
                    }
                    best = Some((dist, i, n));
                }
            }
        }
        best.map(|(t, i, n)| (i, o + d * t, n))
    }

    /// Adesivo nuovo sul punto dell'oggetto colpito dal raggio, largo `width` metri con le
    /// proporzioni `aspect` (altezza / larghezza), dritto rispetto a `up` (spazio mondo).
    pub fn new_decal(&self, among: &[NodeIndex], o: Vec3, d: Vec3, up: Vec3, width: f32, aspect: f32) -> Option<Decal> {
        let (node, p, n) = self.raycast(among, o, d)?;
        let w = self.world_transform(node);
        let inv = w.inverse();
        let n_local = (forge_math::Mat3::from_mat4(w).transpose() * n).normalize_or(Vec3::Z);
        let n_next = self.decals.iter().map(|d| d.n).max().unwrap_or(0) + 1;
        let height = width * aspect.clamp(0.02, 50.0);
        Some(Decal {
            n: n_next,
            name: format!("Adesivo {n_next}"),
            target: self.nodes[node].id.clone(),
            center: inv.transform_point3(p),
            normal: n_local,
            up: inv.transform_vector3(up).normalize_or(Vec3::Y),
            width,
            height,
            depth: width.max(height) * 0.5,
            ..Default::default()
        })
    }

    /// Riquadro di partenza della polvere: gli oggetti della scena (senza pavimenti enormi) fino
    /// alle luci a fascio (spot), così i granelli ci sono in tutto il fascio, fino al piano
    /// illuminato; al massimo 6 m per lato.
    pub fn default_dust_region(&self) -> (Vec3, Vec3) {
        let existing = self.find(&ComponentId(DUST_ID.into()));
        let mut b = forge_math::Aabb::EMPTY;
        for i in 0..self.nodes.len() {
            if Some(i) == existing || self.is_effect_node(i) || self.nodes[i].mesh.is_none() || !self.is_visible(i) {
                continue;
            }
            b.grow(&self.bounds_of(i));
        }
        if b.is_empty() {
            b.grow_point(Vec3::ZERO);
            b.grow_point(Vec3::splat(1.0));
        }
        for l in &self.lights {
            if l.enabled && matches!(l.kind, crate::LightKind::Spot { .. }) {
                b.grow_point(l.position);
            }
        }
        let c = (b.min + b.max) * 0.5;
        (c, (b.max - b.min).max(Vec3::splat(0.2)).min(Vec3::splat(6.0)))
    }

    /// È un nodo creato da un effetto (adesivo, polvere): non si salva come oggetto.
    pub fn is_effect_node(&self, idx: NodeIndex) -> bool {
        let id = &self.nodes[idx].id.0;
        id.starts_with(DECAL_PREFIX) || is_any_dust_id(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives;

    /// Adesivo sul lato di un cubo: copre solo la faccia davanti, con le UV da 0 a 1.
    #[test]
    fn decal_covers_only_the_front_face() {
        let cube = primitives::cuboid(Vec3::splat(0.2));
        let d = Decal {
            center: Vec3::new(0.0, 0.0, 0.1),
            normal: Vec3::Z,
            width: 0.1,
            height: 0.05,
            depth: 0.02,
            ..Default::default()
        };
        let m = d.build_mesh(&cube, 1.0);
        assert!(!m.indices.is_empty());
        for p in &m.positions {
            assert!((p.z - 0.1).abs() < 1e-3, "solo sulla faccia +Z: {p:?}");
            assert!(p.x.abs() <= 0.05 + 1e-4 && p.y.abs() <= 0.025 + 1e-4, "{p:?}");
        }
        let (lo, hi) = m
            .uvs
            .iter()
            .fold((Vec2::splat(9.0), Vec2::splat(-9.0)), |(a, b), uv| (a.min(*uv), b.max(*uv)));
        assert!(lo.x.abs() < 1e-4 && lo.y.abs() < 1e-4 && (hi.x - 1.0).abs() < 1e-4 && (hi.y - 1.0).abs() < 1e-4);
    }

    /// Su un cilindro l'adesivo segue la curva.
    #[test]
    fn decal_wraps_a_curved_surface() {
        let cyl = primitives::cylinder(0.05, 0.2, 64);
        let d = Decal {
            center: Vec3::new(0.0, 0.1, 0.05),
            normal: Vec3::Z,
            width: 0.06,
            height: 0.04,
            depth: 0.03,
            ..Default::default()
        };
        let m = d.build_mesh(&cyl, 1.0);
        assert!(m.indices.len() > 4, "{}", m.indices.len());
        for p in &m.positions {
            let r = (p.x * p.x + p.z * p.z).sqrt();
            assert!((r - 0.05).abs() < 2e-3, "sulla superficie del cilindro: r = {r}");
        }
    }

    /// Il raggio dalla camera trova la faccia davanti del cubo e l'adesivo nasce lì.
    #[test]
    fn new_decal_lands_on_the_hit_face() {
        let mut s = Scene::default();
        let m = s.add_mesh(primitives::cuboid(Vec3::splat(0.2)));
        s.add_node(None, ComponentId("cubo".into()), "Cubo", Mat4::from_translation(Vec3::X), Some(m));
        let d = s
            .new_decal(&[], Vec3::new(1.0, 0.0, 2.0), -Vec3::Z, Vec3::Y, 0.05, 0.5)
            .expect("colpito");
        assert_eq!(d.target.0, "cubo");
        assert!((d.center - Vec3::new(0.0, 0.0, 0.1)).length() < 1e-4, "{:?}", d.center);
        assert!((d.normal - Vec3::Z).length() < 1e-4);
        assert!((d.height - 0.025).abs() < 1e-6);
        s.decals.push(d);
        assert!(s.sync_effects());
        let i = s.find(&ComponentId("adesivo:1".into())).expect("nodo dell'adesivo");
        assert_eq!(s.nodes[i].parent, s.find(&ComponentId("cubo".into())));
        assert!(!s.meshes[s.nodes[i].mesh.unwrap()].indices.is_empty());
        assert!(!s.sync_effects(), "niente da rifare");
        s.decals.clear();
        assert!(s.sync_effects());
        assert!(s.find(&ComponentId("adesivo:1".into())).is_none());
    }

    #[test]
    fn dust_is_deterministic_and_bounded() {
        let d = Dust {
            enabled: true,
            per_m3: 1000.0,
            ..Default::default()
        };
        let a = d.build_mesh(Vec3::ZERO, Vec3::ONE);
        assert_eq!(a, d.build_mesh(Vec3::ZERO, Vec3::ONE));
        assert!(a.positions.iter().all(|p| p.min_element() > -0.01 && p.max_element() < 1.01));
        assert!(!a.indices.is_empty());
    }
}
