use crate::env_map::Distribution1D;
use crate::texture::Texture;
use crate::EnvironmentMap;
use forge_accel::TriangleAccel;
use forge_math::{Aabb, Mat3, Vec2, Vec3};
use forge_scene::{Camera, Environment, LightKind, Material, Scene, TextureMapping};
use std::sync::Arc;

/// Materiale con i parametri derivati già calcolati per l'integratore.
/// È `Copy`: con le texture l'integratore ne crea una copia locale per ogni punto colpito.
#[derive(Clone, Copy, Debug)]
pub struct RenderMaterial {
    pub base_color: Vec3,
    pub metallic: f32,
    /// α di GGX = roughness², limitato per stabilità numerica.
    pub alpha: f32,
    pub ior: f32,
    /// F0 del dielettrico derivato da IOR (prima della scala `specular`).
    pub f0_dielectric: f32,
    /// 2 × specular (0.5 → 1, cioè F0 fisico da IOR).
    pub specular_scale: f32,
    pub transmission: f32,
    pub emission: Vec3,
    /// Roughness originale (serve per modularla con la texture).
    pub roughness: f32,
}

/// Texture di un materiale, già caricate.
#[derive(Debug, Default)]
pub struct MaterialTextures {
    pub base_color: Option<Arc<Texture>>,
    pub roughness: Option<Arc<Texture>>,
    pub normal: Option<Arc<Texture>>,
    pub normal_strength: f32,
    pub metallic: Option<Arc<Texture>>,
    pub height: Option<Arc<Texture>>,
    pub ao: Option<Arc<Texture>>,
    pub opacity: Option<Arc<Texture>>,
    pub normal_directx: bool,
    pub roughness_is_gloss: bool,
    /// Profondità del rilievo in metri.
    pub bump_m: f32,
    pub ao_strength: f32,
    pub mapping: TextureMapping,
}

impl MaterialTextures {
    /// Carica le texture del materiale (dalla cache). Le texture mancanti vengono
    /// segnalate nel log e ignorate: il render prosegue senza.
    pub fn load(m: &Material) -> Option<Arc<MaterialTextures>> {
        if !m.has_textures() {
            return None;
        }
        let load = |p: &Option<std::path::PathBuf>, srgb: bool| {
            p.as_ref().and_then(|p| match crate::texture::load_cached(p, srgb) {
                Ok(t) => Some(t),
                Err(e) => {
                    tracing::error!("materiale «{}»: {e:#}", m.name);
                    None
                }
            })
        };
        let m = m.sanitized();
        let t = MaterialTextures {
            base_color: load(&m.base_color_texture, true),
            roughness: load(&m.roughness_texture, false),
            normal: load(&m.normal_texture, false),
            normal_strength: m.normal_strength,
            metallic: load(&m.metallic_texture, false),
            height: load(&m.height_texture, false),
            ao: load(&m.ao_texture, false),
            opacity: load(&m.opacity_texture, false),
            normal_directx: m.normal_directx,
            roughness_is_gloss: m.roughness_is_gloss,
            bump_m: m.bump_mm * 1e-3,
            ao_strength: m.ao_strength,
            mapping: m.mapping,
        };
        let any = [&t.base_color, &t.roughness, &t.normal, &t.metallic, &t.height, &t.ao, &t.opacity]
            .iter()
            .any(|x| x.is_some());
        any.then(|| Arc::new(t))
    }
}

impl RenderMaterial {
    pub fn from_material(m: &Material) -> Self {
        let m = m.sanitized();
        let f0 = ((m.ior - 1.0) / (m.ior + 1.0)).powi(2);
        Self {
            base_color: Vec3::from_array(m.base_color),
            metallic: m.metallic,
            alpha: (m.roughness * m.roughness).max(1e-3),
            ior: m.ior,
            f0_dielectric: f0,
            specular_scale: m.specular * 2.0,
            transmission: m.transmission,
            emission: Vec3::from_array(m.emission) * m.emission_strength,
            roughness: m.roughness,
        }
    }
}

#[derive(Clone, Debug)]
pub enum RenderLightKind {
    Point {
        position: Vec3,
    },
    Spot {
        position: Vec3,
        direction: Vec3,
        cos_inner: f32,
        cos_outer: f32,
    },
    /// `direction` è la direzione di propagazione della luce.
    Directional {
        direction: Vec3,
    },
    /// Rettangolo con centro, normale di emissione e semi-assi.
    Area {
        center: Vec3,
        normal: Vec3,
        u: Vec3,
        v: Vec3,
        half_w: f32,
        half_h: f32,
        area: f32,
    },
}

#[derive(Clone, Debug)]
pub struct RenderLight {
    pub kind: RenderLightKind,
    /// Intensità (Point/Spot), irradianza (Directional) o radianza (Area).
    pub emission: Vec3,
    pub cast_shadows: bool,
    pub visible_to_camera: bool,
}

/// Geometria compilata: triangoli in spazio mondo con BVH e attributi per
/// primitiva (nell'ordine originale, indicizzati da `TriangleHit::prim`).
/// Dipende solo da mesh, trasformazioni e visibilità: cambiare materiali,
/// luci o camera non richiede di ricostruirla.
pub struct RenderGeometry {
    /// Identificativo univoco (per processo): chiave delle cache che dipendono dalla geometria.
    pub id: u64,
    pub accel: TriangleAccel,
    pub geometric_normals: Vec<Vec3>,
    /// Normali di shading ai tre vertici (uguali alla geometrica se la mesh non ne ha).
    pub shading_normals: Vec<[Vec3; 3]>,
    /// Indice del nodo di scena di provenienza.
    pub tri_node: Vec<u32>,
    /// Coordinate UV ai vertici (zero se la mesh non ne ha).
    pub tri_uvs: Vec<[Vec2; 3]>,
    /// Tangente ∂p/∂u in spazio mondo (zero se la mesh non ha UV).
    pub tri_tangents: Vec<Vec3>,
    /// Flag per primitiva (vedi [`crate::displace::FLAG_BOX_UV`]).
    pub tri_flags: Vec<u32>,
    /// Posizione di ogni primitiva (ordine originale) dentro `accel.tris` (ordine BVH).
    pub prim_to_slot: Vec<u32>,
    pub bounds: Aabb,
}

impl RenderGeometry {
    /// Vertici del triangolo `prim` in spazio mondo.
    #[inline]
    pub fn vertices(&self, prim: usize) -> [Vec3; 3] {
        let t = &self.accel.tris[self.prim_to_slot[prim] as usize];
        [t.v0, t.v0 + t.e1, t.v0 + t.e2]
    }
}

/// Triangoli emissivi, campionati in proporzione alla potenza emessa (NEE).
#[derive(Debug)]
pub struct EmissiveSet {
    pub prims: Vec<u32>,
    pub areas: Vec<f32>,
    pub distribution: Distribution1D,
    /// Per ogni primitiva: indice in `prims`, o `u32::MAX` se non emissiva.
    pub index_of_prim: Vec<u32>,
}

impl EmissiveSet {
    /// PDF d'area (per unità di superficie) di campionare un punto del triangolo `prim`.
    #[inline]
    pub fn area_pdf(&self, prim: usize) -> f32 {
        match self.index_of_prim.get(prim) {
            Some(&i) if i != u32::MAX => {
                let i = i as usize;
                self.distribution.pdf(i) / self.distribution.count() as f32 / self.areas[i]
            }
            _ => 0.0,
        }
    }
}

impl RenderGeometry {
    /// Appiattisce la gerarchia (solo componenti visibili), trasforma vertici e
    /// normali in spazio mondo e costruisce la BVH.
    pub fn build(scene: &Scene) -> RenderGeometry {
        let t0 = std::time::Instant::now();
        let mut tris = Vec::new();
        let mut gn = Vec::new();
        let mut sn = Vec::new();
        let mut tnode = Vec::new();
        let mut tuv = Vec::new();
        let mut ttan = Vec::new();
        let mut tflags = Vec::new();
        let displaced = displacement_plan(scene);
        for (i, node) in scene.nodes.iter().enumerate() {
            let Some(mi) = node.mesh else { continue };
            if !scene.is_visible(i) {
                continue;
            }
            let mesh = &scene.meshes[mi];
            let xf = scene.world_transform(i);
            let nxf = Mat3::from_mat4(xf).inverse().transpose();
            // Una trasformazione con determinante negativo (specchiatura) inverte il winding.
            let mirrored = Mat3::from_mat4(xf).determinant() < 0.0;
            let has_n = mesh.has_normals();
            let has_uv = mesh.uvs.len() == mesh.positions.len();
            if let Some((params, target)) = displaced.get(&i) {
                for t in crate::displace::tessellate(mesh, &xf, mirrored, params, *target) {
                    let p = t.p;
                    let ng = (p[1] - p[0]).cross(p[2] - p[0]);
                    let ng = if mirrored { -ng } else { ng }.normalize_or_zero();
                    if ng == Vec3::ZERO || !p.iter().all(|v| v.is_finite()) {
                        continue;
                    }
                    let (d1, d2) = (t.uv[1] - t.uv[0], t.uv[2] - t.uv[0]);
                    let det = d1.x * d2.y - d1.y * d2.x;
                    let tangent = if t.flags == 0 && det.abs() > 1e-12 {
                        ((p[1] - p[0]) * d2.y - (p[2] - p[0]) * d1.y) / det
                    } else {
                        Vec3::ZERO
                    };
                    tris.push(p);
                    gn.push(ng);
                    sn.push(t.n);
                    tnode.push(i as u32);
                    tuv.push(t.uv);
                    ttan.push(if tangent.is_finite() { tangent } else { Vec3::ZERO });
                    tflags.push(t.flags);
                }
                continue;
            }
            for idx in &mesh.indices {
                let p = idx.map(|k| xf.transform_point3(mesh.positions[k as usize]));
                let mut ng = (p[1] - p[0]).cross(p[2] - p[0]);
                if mirrored {
                    ng = -ng;
                }
                let ng = ng.normalize_or_zero();
                if ng == Vec3::ZERO || !p.iter().all(|v| v.is_finite()) {
                    continue; // degenere
                }
                let ns = if has_n {
                    idx.map(|k| (nxf * mesh.normals[k as usize]).normalize_or(ng))
                } else {
                    [ng; 3]
                };
                let (uv, tangent) = if has_uv {
                    let uv = idx.map(|k| mesh.uvs[k as usize]);
                    // ∂p/∂u dai differenziali del triangolo.
                    let (d1, d2) = (uv[1] - uv[0], uv[2] - uv[0]);
                    let det = d1.x * d2.y - d1.y * d2.x;
                    let t = if det.abs() > 1e-12 {
                        ((p[1] - p[0]) * d2.y - (p[2] - p[0]) * d1.y) / det
                    } else {
                        Vec3::ZERO
                    };
                    (uv, if t.is_finite() { t } else { Vec3::ZERO })
                } else {
                    ([Vec2::ZERO; 3], Vec3::ZERO)
                };
                tris.push(p);
                gn.push(ng);
                sn.push(ns);
                tnode.push(i as u32);
                tuv.push(uv);
                ttan.push(tangent);
                tflags.push(0);
            }
        }
        let accel = TriangleAccel::build(&tris);
        let bounds = accel.bvh.bounds();
        let mut prim_to_slot = vec![0u32; accel.bvh.prim_indices.len()];
        for (slot, &prim) in accel.bvh.prim_indices.iter().enumerate() {
            prim_to_slot[prim as usize] = slot as u32;
        }
        tracing::info!(
            "geometria compilata: {} triangoli, BVH {} nodi, in {:.0} ms",
            tris.len(),
            accel.bvh.nodes.len(),
            t0.elapsed().as_secs_f64() * 1e3
        );
        static NEXT_GEO: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        RenderGeometry {
            id: NEXT_GEO.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            accel,
            geometric_normals: gn,
            shading_normals: sn,
            tri_node: tnode,
            tri_uvs: tuv,
            tri_tangents: ttan,
            tri_flags: tflags,
            prim_to_slot,
            bounds,
        }
    }
}

/// Componenti da suddividere per il displacement, con parametri e lato dei
/// triangoli (allungato in modo uniforme se si supera il budget di triangoli).
fn displacement_plan(scene: &Scene) -> std::collections::HashMap<usize, (crate::displace::Params, f32)> {
    use crate::displace::{estimate, Params, BUDGET};
    let mut plan = std::collections::HashMap::new();
    for (i, node) in scene.nodes.iter().enumerate() {
        let Some(mi) = node.mesh else { continue };
        if !scene.is_visible(i) {
            continue;
        }
        let Some(m) = scene.materials.get(scene.resolve_material(i)) else {
            continue;
        };
        if !m.has_displacement() {
            continue;
        }
        let m = m.sanitized();
        let path = m.height_texture.as_ref().expect("height");
        let height = match crate::texture::load_cached(path, false) {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("displacement di «{}»: {e:#}", m.name);
                continue;
            }
        };
        let mesh = &scene.meshes[mi];
        let params = Params {
            height,
            mapping: m.mapping,
            amount_m: m.displacement_mm * 1e-3,
            mid: m.displacement_mid,
            use_uv: m.mapping.projection == forge_scene::Projection::Uv && mesh.uvs.len() == mesh.positions.len(),
        };
        let target = params.target_edge(mesh, &scene.world_transform(i));
        plan.insert(i, (params, target));
    }
    // Budget: se serve si allunga il lato di tutti finché il totale rientra.
    for _ in 0..12 {
        let total: usize = plan
            .iter()
            .map(|(&i, (_, t))| estimate(&scene.meshes[scene.nodes[i].mesh.unwrap()], &scene.world_transform(i), *t))
            .sum();
        if total <= BUDGET {
            if !plan.is_empty() {
                tracing::info!("displacement: {} componenti, circa {total} triangoli", plan.len());
            }
            break;
        }
        for (_, t) in plan.values_mut() {
            *t *= 2.0;
        }
    }
    plan
}

/// Scena compilata per il rendering, indipendente dal backend.
pub struct RenderScene {
    /// Identificativo univoco (per processo): i backend GPU lo usano per sapere
    /// quando ricaricare i dati della scena in memoria video.
    pub id: u64,
    /// Identifica tutto tranne la camera: due scene con lo stesso `content_id` hanno gli
    /// stessi dati in memoria video, quindi spostando la vista non si ricarica nulla.
    pub content_id: u64,
    pub geometry: Arc<RenderGeometry>,
    /// Materiale effettivo di ogni nodo della scena (indice in `materials`).
    pub node_material: Vec<u32>,
    pub materials: Vec<RenderMaterial>,
    /// Texture per materiale (stesso indice di `materials`).
    pub material_textures: Vec<Option<Arc<MaterialTextures>>>,
    pub lights: Vec<RenderLight>,
    pub environment: Environment,
    /// Mappa HDRI caricata (se l'ambiente è una HDRI), con intensità e rotazione in radianti.
    pub env_map: Option<(Arc<EnvironmentMap>, f32, f32)>,
    /// Superfici emissive campionabili come luci.
    pub emissive: Option<Arc<EmissiveSet>>,
    pub camera: Camera,
    /// Qualche materiale ha una mappa di opacità (ritaglio): le intersezioni vanno filtrate.
    pub has_cutout: bool,
    /// Nebbie volumetriche già calcolate.
    pub fogs: Vec<Arc<forge_volume::BakedFog>>,
}

impl RenderScene {
    /// Compila tutta la scena, BVH compresa.
    pub fn compile(scene: &Scene) -> RenderScene {
        Self::compile_with_geometry(scene, Arc::new(RenderGeometry::build(scene)))
    }

    /// Compila materiali, luci, ambiente e camera riutilizzando una geometria
    /// già costruita (operazione economica: nessuna ricostruzione della BVH).
    pub fn compile_with_geometry(scene: &Scene, geometry: Arc<RenderGeometry>) -> RenderScene {
        let mut materials: Vec<RenderMaterial> = scene.materials.materials.iter().map(RenderMaterial::from_material).collect();
        let mut material_textures: Vec<Option<Arc<MaterialTextures>>> =
            scene.materials.materials.iter().map(MaterialTextures::load).collect();
        let fallback = materials.len() as u32;
        materials.push(RenderMaterial::from_material(&Material::default()));
        material_textures.push(None);
        let node_material: Vec<u32> = (0..scene.nodes.len())
            .map(|i| {
                scene
                    .materials
                    .index_of(scene.resolve_material(i))
                    .map(|x| x as u32)
                    .unwrap_or(fallback)
            })
            .collect();

        let lights = scene
            .lights
            .iter()
            .filter(|l| l.enabled && l.intensity > 0.0)
            .map(|l| {
                let kind = match l.kind {
                    LightKind::Point => RenderLightKind::Point { position: l.position },
                    LightKind::Spot {
                        inner_angle_deg,
                        outer_angle_deg,
                    } => {
                        let outer = outer_angle_deg.clamp(0.1, 179.0) * 0.5;
                        let inner = inner_angle_deg.clamp(0.0, outer_angle_deg) * 0.5;
                        RenderLightKind::Spot {
                            position: l.position,
                            direction: l.direction(),
                            cos_inner: inner.to_radians().cos(),
                            cos_outer: outer.to_radians().cos(),
                        }
                    }
                    LightKind::Directional => RenderLightKind::Directional { direction: l.direction() },
                    LightKind::Area { width, height } => {
                        let n = l.direction();
                        let hint = if n.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
                        let u = hint.cross(n).normalize();
                        let v = n.cross(u);
                        let (w, h) = (width.max(1e-6), height.max(1e-6));
                        RenderLightKind::Area {
                            center: l.position,
                            normal: n,
                            u,
                            v,
                            half_w: w * 0.5,
                            half_h: h * 0.5,
                            area: w * h,
                        }
                    }
                };
                RenderLight {
                    kind,
                    emission: l.radiance_color(),
                    cast_shadows: l.cast_shadows,
                    visible_to_camera: l.visible_to_camera,
                }
            })
            .collect();

        let mut environment = scene.environment.clone();
        let env_map = match &scene.environment {
            Environment::Hdri {
                path,
                strength,
                rotation_deg,
            } => match crate::env_map::load_cached(path) {
                Ok(m) => Some((m, strength.max(0.0), rotation_deg.to_radians())),
                Err(e) => {
                    tracing::error!("HDRI non caricata ({e:#}): uso il cielo a gradiente predefinito");
                    environment = Environment::default();
                    None
                }
            },
            _ => None,
        };

        let emissive = build_emissive(&geometry, &node_material, &materials).map(Arc::new);

        let has_cutout = material_textures.iter().flatten().any(|t| t.opacity.is_some());
        let fogs = crate::fog::bake_fogs(scene, &geometry);
        let id = next_scene_id();
        RenderScene {
            has_cutout,
            fogs,
            id,
            content_id: id,
            emissive,
            geometry,
            node_material,
            materials,
            material_textures,
            lights,
            environment,
            env_map,
            camera: scene.render_camera().clone(),
        }
    }

    /// Stessa scena vista da un'altra camera: nessuna ricompilazione, e il backend GPU
    /// riusa i dati già caricati (serve per muovere la vista col render attivo).
    pub fn with_camera(&self, camera: Camera) -> RenderScene {
        RenderScene {
            id: next_scene_id(),
            content_id: self.content_id,
            geometry: self.geometry.clone(),
            node_material: self.node_material.clone(),
            materials: self.materials.clone(),
            material_textures: self.material_textures.clone(),
            lights: self.lights.clone(),
            environment: self.environment.clone(),
            env_map: self.env_map.clone(),
            emissive: self.emissive.clone(),
            camera,
            has_cutout: self.has_cutout,
            fogs: self.fogs.clone(),
        }
    }

    pub fn triangle_count(&self) -> usize {
        self.geometry.tri_node.len()
    }

    /// Radianza dell'ambiente in una direzione. `smooth` usa l'interpolazione
    /// bilineare della HDRI (solo per lo sfondo visto direttamente).
    #[inline]
    pub fn env_radiance(&self, dir: Vec3, smooth: bool) -> Vec3 {
        match &self.env_map {
            Some((m, s, r)) if smooth => m.radiance_smooth(dir, *r) * *s,
            Some((m, s, r)) => m.radiance(dir, *r) * *s,
            None => self.environment.radiance(dir),
        }
    }

    pub fn bounds(&self) -> Aabb {
        self.geometry.bounds
    }

    /// Indice del materiale del triangolo `prim`.
    #[inline]
    pub fn material_index(&self, prim: usize) -> usize {
        self.node_material[self.geometry.tri_node[prim] as usize] as usize
    }

    /// Materiale del triangolo `prim`.
    #[inline]
    pub fn material_of(&self, prim: usize) -> &RenderMaterial {
        &self.materials[self.material_index(prim)]
    }

    /// Picking: nodo di scena colpito da un raggio.
    pub fn pick(&self, origin: Vec3, dir: Vec3) -> Option<(usize, f32)> {
        pick(&self.geometry, origin, dir)
    }
}

/// Picking sulla sola geometria.
pub fn pick(geometry: &RenderGeometry, origin: Vec3, dir: Vec3) -> Option<(usize, f32)> {
    geometry
        .accel
        .intersect(origin, dir, 0.0, f32::INFINITY)
        .map(|h| (geometry.tri_node[h.prim as usize] as usize, h.t))
}

fn next_scene_id() -> u64 {
    static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

fn build_emissive(geo: &RenderGeometry, node_material: &[u32], materials: &[RenderMaterial]) -> Option<EmissiveSet> {
    if !materials.iter().any(|m| m.emission != Vec3::ZERO) {
        return None;
    }
    let mut prims = Vec::new();
    let mut areas = Vec::new();
    let mut power = Vec::new();
    for prim in 0..geo.tri_node.len() {
        let m = &materials[node_material[geo.tri_node[prim] as usize] as usize];
        if m.emission == Vec3::ZERO {
            continue;
        }
        let [a, b, c] = geo.vertices(prim);
        let area = 0.5 * (b - a).cross(c - a).length();
        if area <= 0.0 {
            continue;
        }
        prims.push(prim as u32);
        areas.push(area);
        power.push(forge_math::luminance(m.emission) * area);
    }
    if prims.is_empty() {
        return None;
    }
    let mut index_of_prim = vec![u32::MAX; geo.tri_node.len()];
    for (i, &p) in prims.iter().enumerate() {
        index_of_prim[p as usize] = i as u32;
    }
    tracing::info!("{} triangoli emissivi campionati come luci", prims.len());
    Some(EmissiveSet {
        prims,
        areas,
        distribution: Distribution1D::new(power),
        index_of_prim,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_test_scene() {
        let s = Scene::test_scene();
        let rs = RenderScene::compile(&s);
        assert_eq!(rs.triangle_count(), s.triangle_count());
        assert_eq!(rs.lights.len(), s.lights.len());
        // Il raggio centrale della camera colpisce qualcosa.
        assert!(rs.pick(s.camera.position, s.camera.forward()).is_some());
    }

    #[test]
    fn hidden_components_are_excluded() {
        let mut s = Scene::test_scene();
        let total = s.triangle_count();
        s.nodes[1].visible = false; // pavimento
        let rs = RenderScene::compile(&s);
        assert_eq!(rs.triangle_count(), total - 2);
    }
}
