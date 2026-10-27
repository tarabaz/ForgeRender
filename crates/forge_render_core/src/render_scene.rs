use forge_accel::TriangleAccel;
use forge_math::{Aabb, Mat3, Vec3};
use forge_scene::{Camera, Environment, LightKind, Material, Scene};
use std::sync::Arc;

/// Materiale con i parametri derivati già calcolati per l'integratore.
#[derive(Clone, Debug)]
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
    pub accel: TriangleAccel,
    pub geometric_normals: Vec<Vec3>,
    /// Normali di shading ai tre vertici (uguali alla geometrica se la mesh non ne ha).
    pub shading_normals: Vec<[Vec3; 3]>,
    /// Indice del nodo di scena di provenienza.
    pub tri_node: Vec<u32>,
    pub bounds: Aabb,
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
                tris.push(p);
                gn.push(ng);
                sn.push(ns);
                tnode.push(i as u32);
            }
        }
        let accel = TriangleAccel::build(&tris);
        let bounds = accel.bvh.bounds();
        tracing::info!(
            "geometria compilata: {} triangoli, BVH {} nodi, in {:.0} ms",
            tris.len(),
            accel.bvh.nodes.len(),
            t0.elapsed().as_secs_f64() * 1e3
        );
        RenderGeometry {
            accel,
            geometric_normals: gn,
            shading_normals: sn,
            tri_node: tnode,
            bounds,
        }
    }
}

/// Scena compilata per il rendering, indipendente dal backend.
pub struct RenderScene {
    pub geometry: Arc<RenderGeometry>,
    /// Materiale effettivo di ogni nodo della scena (indice in `materials`).
    pub node_material: Vec<u32>,
    pub materials: Vec<RenderMaterial>,
    pub lights: Vec<RenderLight>,
    pub environment: Environment,
    pub camera: Camera,
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
        let fallback = materials.len() as u32;
        materials.push(RenderMaterial::from_material(&Material::default()));
        let node_material = (0..scene.nodes.len())
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

        RenderScene {
            geometry,
            node_material,
            materials,
            lights,
            environment: scene.environment.clone(),
            camera: scene.camera.clone(),
        }
    }

    pub fn triangle_count(&self) -> usize {
        self.geometry.tri_node.len()
    }

    pub fn bounds(&self) -> Aabb {
        self.geometry.bounds
    }

    /// Materiale del triangolo `prim`.
    #[inline]
    pub fn material_of(&self, prim: usize) -> &RenderMaterial {
        &self.materials[self.node_material[self.geometry.tri_node[prim] as usize] as usize]
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
