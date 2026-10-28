//! Importazione glTF 2.0 (`.gltf` / `.glb`), il formato di scambio esportato da
//! Blender: gerarchia, mesh, materiali PBR metallic/roughness (con trasmissione,
//! IOR ed emissione), texture, camera e luci (`KHR_lights_punctual`).
//!
//! Convenzioni: glTF è in metri con Y verso l'alto (come la scena); le UV hanno
//! origine in alto a sinistra e vengono capovolte; le luci sono in unità
//! fotometriche (candele, lux) e vengono convertite in radiometriche dividendo
//! per 683 lm/W.

use crate::{ImportError, ImportOptions, ImportReport};
use forge_math::{Mat4, Vec2, Vec3};
use forge_scene::{Camera, ComponentId, Light, LightKind, Material, MaterialId, Mesh, Projection, Scene, TextureMapping};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const LM_PER_W: f32 = 683.0;

/// Luce ad area esportata da Blender a parte (glTF non le supporta).
#[derive(Clone, Debug, serde::Deserialize)]
pub struct AreaLightRecord {
    pub name: String,
    /// Matrice mondo 4×4 column-major, già in convenzione glTF (Y verso l'alto).
    pub matrix: [f32; 16],
    pub width: f32,
    pub height: f32,
    pub color: [f32; 3],
    /// Potenza in watt (convenzione di Blender).
    pub power: f32,
}

pub fn import(scene: &mut Scene, path: &Path, opts: &ImportOptions) -> Result<ImportReport, ImportError> {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("modello.glb").to_string();
    let dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    import_named(scene, path, &name, &dir, opts, Vec::new())
}

/// Importa `path` usando `file_name` come radice degli id e `asset_dir` come cartella
/// in cui salvare le texture incorporate (serve quando il glTF è un file temporaneo
/// esportato da un `.blend`: le texture vanno accanto al `.blend`).
pub fn import_named(
    scene: &mut Scene,
    path: &Path,
    file_name: &str,
    asset_dir: &Path,
    opts: &ImportOptions,
    area_lights: Vec<AreaLightRecord>,
) -> Result<ImportReport, ImportError> {
    let (doc, buffers, images) = gltf::import(path).map_err(|e| {
        let msg = e.to_string();
        if msg.contains("received 0 bytes") {
            ImportError::Parse(format!(
                "il file è incompleto: contiene la descrizione del modello ma non i dati (geometria e texture).\n\
                 Riesportalo dal programma di origine e controlla che non pesi solo pochi KB; da SolidWorks, se il .glb\n\
                 esce vuoto, prova \"Extended Reality (*.gltf)\" e copia tutta la cartella (file .gltf, .bin e immagini).\n\
                 (dettaglio: {msg})"
            ))
        } else {
            ImportError::Parse(format!("glTF: {msg}"))
        }
    })?;
    let mut warnings = Vec::new();
    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("modello")
        .to_string();

    // --- Texture: le immagini incorporate vengono salvate come PNG accanto al file. ---
    let tex_dir = asset_dir.join(format!("{stem}_texture"));
    let mut saved: HashMap<(usize, &'static str), PathBuf> = HashMap::new();
    let mut save_image = |index: usize, kind: &'static str, warnings: &mut Vec<String>| -> Option<PathBuf> {
        if let Some(p) = saved.get(&(index, kind)) {
            return Some(p.clone());
        }
        let data = images.get(index)?;
        let rgb = to_rgb8(data, kind)?;
        if std::fs::create_dir_all(&tex_dir).is_err() {
            warnings.push(format!("impossibile creare {}", tex_dir.display()));
            return None;
        }
        let out = tex_dir.join(format!("{kind}_{index}.png"));
        match rgb.save(&out) {
            Ok(()) => {
                saved.insert((index, kind), out.clone());
                Some(out)
            }
            Err(e) => {
                warnings.push(format!("texture {index} non salvata: {e}"));
                None
            }
        }
    };

    // --- Materiali ---
    let mut material_ids: Vec<MaterialId> = Vec::new();
    let mut materials_created = Vec::new();
    for (i, m) in doc.materials().enumerate() {
        let name = m.name().map(str::to_string).unwrap_or_else(|| format!("materiale-{i}"));
        let id = MaterialId::new(format!("gltf:{stem}:{name}"));
        let pbr = m.pbr_metallic_roughness();
        let bc = pbr.base_color_factor();
        let mut mat = Material {
            id: id.clone(),
            name: name.clone(),
            base_color: [bc[0], bc[1], bc[2]],
            metallic: pbr.metallic_factor(),
            roughness: pbr.roughness_factor(),
            category: "Importati".into(),
            ..Default::default()
        };
        if let Some(t) = m.transmission() {
            mat.transmission = t.transmission_factor();
        }
        if let Some(ior) = m.ior() {
            mat.ior = ior;
        }
        let e = m.emissive_factor();
        if e.iter().any(|&c| c > 0.0) {
            let strength = m.emissive_strength().unwrap_or(1.0);
            let max = e.iter().cloned().fold(0.0f32, f32::max);
            mat.emission = e.map(|c| c / max);
            mat.emission_strength = max * strength;
        }
        let mut uses_texture = false;
        if let Some(info) = pbr.base_color_texture() {
            mat.base_color_texture = save_image(info.texture().source().index(), "colore", &mut warnings);
            uses_texture |= mat.base_color_texture.is_some();
        }
        if let Some(info) = pbr.metallic_roughness_texture() {
            // glTF: roughness nel canale G, metallic nel B. ForgeRender legge la roughness da R.
            mat.roughness_texture = save_image(info.texture().source().index(), "roughness", &mut warnings);
            uses_texture |= mat.roughness_texture.is_some();
            if mat.metallic > 0.0 {
                mat.metallic_texture = save_image(info.texture().source().index(), "metallic", &mut warnings);
            }
        }
        if let Some(o) = m.occlusion_texture() {
            // glTF: occlusione nel canale R.
            mat.ao_texture = save_image(o.texture().source().index(), "occlusione", &mut warnings);
            mat.ao_strength = o.strength().clamp(0.0, 1.0) * 0.5;
            uses_texture |= mat.ao_texture.is_some();
        }
        if let Some(n) = m.normal_texture() {
            mat.normal_texture = save_image(n.texture().source().index(), "normal", &mut warnings);
            mat.normal_strength = n.scale();
            uses_texture |= mat.normal_texture.is_some();
        }
        if uses_texture {
            mat.mapping = TextureMapping {
                projection: Projection::Uv,
                scale: 1.0,
                rotation_deg: 0.0,
                offset: [0.0; 2],
            };
        }
        let mat = mat.sanitized();
        if scene.materials.get(&id).is_none() {
            materials_created.push(name);
        }
        scene.materials.upsert(mat);
        material_ids.push(id);
    }

    // --- Gerarchia ---
    let root = scene.add_node(None, ComponentId(file_name.to_string()), file_name, Mat4::IDENTITY, None);
    let gscene = doc.default_scene().or_else(|| doc.scenes().next()).ok_or(ImportError::Empty)?;
    let mut ctx = Ctx {
        buffers: &buffers,
        material_ids: &material_ids,
        components: 0,
        triangles: 0,
        camera: None,
        lights: Vec::new(),
        warnings: &mut warnings,
    };
    for node in gscene.nodes() {
        visit(scene, &mut ctx, &node, root, Mat4::IDENTITY);
    }
    let (components, triangles, mut camera, mut lights) = (ctx.components, ctx.triangles, ctx.camera, ctx.lights);
    if triangles == 0 {
        scene.remove_subtree(root);
        return Err(ImportError::Empty);
    }

    for a in area_lights {
        let m = Mat4::from_cols_slice(&a.matrix);
        let pos = m.transform_point3(Vec3::ZERO);
        // In Blender la luce ad area emette lungo −Z locale.
        let dir = m.transform_vector3(-Vec3::Z).normalize_or(-Vec3::Y);
        let sx = m.transform_vector3(Vec3::X).length();
        let sy = m.transform_vector3(Vec3::Y).length();
        let (w, h) = ((a.width * sx).max(1e-4), (a.height * sy).max(1e-4));
        // Emettitore lambertiano: radianza = potenza / (π · area).
        let radiance = a.power / (std::f32::consts::PI * w * h);
        let mut l = Light::area(&a.name, pos, pos + dir, w, h, radiance);
        l.color = a.color;
        lights.push(l);
    }

    // glTF è già Y-up: l'opzione "asse verticale" vale solo se forzata a Z.
    if opts.up == crate::UpAxis::Z {
        let r = Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2);
        scene.nodes[root].local_transform = r;
        for l in &mut lights {
            l.position = r.transform_point3(l.position);
            l.target = r.transform_point3(l.target);
        }
        if let Some(c) = &mut camera {
            c.position = r.transform_point3(c.position);
            c.target = r.transform_point3(c.target);
            c.up = r.transform_vector3(c.up);
        }
    }
    scene.refresh_pivots();
    tracing::info!(
        "importato {file_name}: {components} oggetti, {triangles} triangoli, {} materiali, {} luci, camera: {}",
        material_ids.len(),
        lights.len(),
        if camera.is_some() { "sì" } else { "no" }
    );
    Ok(ImportReport {
        root,
        components,
        triangles,
        removed_triangles: 0,
        generated_normals: false,
        materials_created,
        warnings,
        duration: Default::default(),
        camera,
        lights,
    })
}

struct Ctx<'a> {
    buffers: &'a [gltf::buffer::Data],
    material_ids: &'a [MaterialId],
    components: usize,
    triangles: usize,
    camera: Option<Camera>,
    lights: Vec<Light>,
    warnings: &'a mut Vec<String>,
}

fn visit(scene: &mut Scene, ctx: &mut Ctx, node: &gltf::Node, parent: usize, parent_world: Mat4) {
    let local = Mat4::from_cols_array_2d(&node.transform().matrix());
    let world = parent_world * local;
    let name = node.name().map(str::to_string).unwrap_or_else(|| format!("nodo-{}", node.index()));
    let id = ComponentId(format!("{}/{}", scene.nodes[parent].id.0, name.replace('/', "∕")));
    let idx = scene.add_node(Some(parent), id, name.clone(), local, None);

    if let Some(mesh) = node.mesh() {
        for (pi, prim) in mesh.primitives().enumerate() {
            if prim.mode() != gltf::mesh::Mode::Triangles {
                ctx.warnings.push(format!("{name}: primitiva non triangolare ignorata"));
                continue;
            }
            let reader = prim.reader(|b| ctx.buffers.get(b.index()).map(|d| &d.0[..]));
            let Some(pos) = reader.read_positions() else { continue };
            let positions: Vec<Vec3> = pos.map(Vec3::from_array).collect();
            let normals: Vec<Vec3> = reader.read_normals().map(|n| n.map(Vec3::from_array).collect()).unwrap_or_default();
            // glTF: origine UV in alto a sinistra; ForgeRender (come OBJ) in basso a sinistra.
            let uvs: Vec<Vec2> = reader
                .read_tex_coords(0)
                .map(|t| t.into_f32().map(|[u, v]| Vec2::new(u, 1.0 - v)).collect())
                .unwrap_or_default();
            let indices: Vec<[u32; 3]> = match reader.read_indices() {
                Some(i) => i.into_u32().collect::<Vec<_>>().as_chunks::<3>().0.to_vec(),
                None => (0..positions.len() as u32 / 3).map(|t| [3 * t, 3 * t + 1, 3 * t + 2]).collect(),
            };
            let mut m = Mesh {
                name: format!("{name}#{pi}"),
                normals: if normals.len() == positions.len() { normals } else { Vec::new() },
                uvs: if uvs.len() == positions.len() { uvs } else { Vec::new() },
                positions,
                indices,
            };
            m.remove_invalid_triangles();
            if m.indices.is_empty() {
                continue;
            }
            if !m.has_normals() {
                m.compute_normals(40.0);
            }
            ctx.triangles += m.triangle_count();
            ctx.components += 1;
            let mi = scene.add_mesh(m);
            let sub_name = if mesh.primitives().len() == 1 {
                format!("{name} (mesh)")
            } else {
                format!("{name} (parte {})", pi + 1)
            };
            let sub_id = ComponentId(format!("{}/{}", scene.nodes[idx].id.0, sub_name));
            let sub = scene.add_node(Some(idx), sub_id.clone(), sub_name, Mat4::IDENTITY, Some(mi));
            if let Some(mat) = prim.material().index().and_then(|i| ctx.material_ids.get(i)) {
                scene.assignments.insert(scene.nodes[sub].id.clone(), mat.clone());
            }
        }
    }

    if let Some(cam) = node.camera() {
        if ctx.camera.is_none() {
            // La camera glTF guarda lungo −Z locale con +Y in alto.
            let position = world.transform_point3(Vec3::ZERO);
            let fwd = world.transform_vector3(-Vec3::Z).normalize_or(-Vec3::Z);
            let up = world.transform_vector3(Vec3::Y).normalize_or(Vec3::Y);
            match cam.projection() {
                gltf::camera::Projection::Perspective(p) => {
                    ctx.camera = Some(Camera {
                        position,
                        target: position + fwd,
                        up,
                        vfov_deg: p.yfov().to_degrees(),
                        ..Default::default()
                    });
                }
                gltf::camera::Projection::Orthographic(o) => {
                    // Il renderer è prospettico: stessa direzione di vista, campo visivo
                    // moderato e distanza tale da inquadrare la stessa altezza (2·ymag)
                    // al centro del volume visibile (es. la "current camera" di SolidWorks).
                    const VFOV: f32 = 30.0;
                    let center = position + fwd * 0.5 * (o.znear() + o.zfar());
                    let dist = o.ymag().abs().max(1e-6) / (VFOV.to_radians() * 0.5).tan();
                    ctx.camera = Some(Camera {
                        position: center - fwd * dist,
                        target: center,
                        up,
                        vfov_deg: VFOV,
                        ..Default::default()
                    });
                }
            }
        }
    }

    if let Some(light) = node.light() {
        let position = world.transform_point3(Vec3::ZERO);
        let dir = world.transform_vector3(-Vec3::Z).normalize_or(-Vec3::Y);
        let color = light.color();
        let lname = light.name().map(str::to_string).unwrap_or_else(|| name.clone());
        let base = Light::point(&lname, position, light.intensity() / LM_PER_W);
        let l = match light.kind() {
            gltf::khr_lights_punctual::Kind::Point => base,
            gltf::khr_lights_punctual::Kind::Spot {
                inner_cone_angle,
                outer_cone_angle,
            } => Light {
                kind: LightKind::Spot {
                    inner_angle_deg: 2.0 * inner_cone_angle.to_degrees(),
                    outer_angle_deg: 2.0 * outer_cone_angle.to_degrees(),
                },
                target: position + dir,
                ..base
            },
            gltf::khr_lights_punctual::Kind::Directional => Light {
                kind: LightKind::Directional,
                target: position + dir,
                ..base
            },
        };
        ctx.lights.push(Light { color, ..l });
    }

    for child in node.children() {
        visit(scene, ctx, &child, idx, world);
    }
}

/// Converte un'immagine glTF in RGB8. Per la mappa metallic/roughness copia il
/// canale G (roughness) in tutti i canali, perché ForgeRender legge la roughness da R.
fn to_rgb8(data: &gltf::image::Data, kind: &str) -> Option<image::RgbImage> {
    use gltf::image::Format;
    let (w, h) = (data.width, data.height);
    let px = &data.pixels;
    let n = (w * h) as usize;
    let get: Box<dyn Fn(usize) -> [u8; 3]> = match data.format {
        Format::R8G8B8 => Box::new(|i| [px[3 * i], px[3 * i + 1], px[3 * i + 2]]),
        Format::R8G8B8A8 => Box::new(|i| [px[4 * i], px[4 * i + 1], px[4 * i + 2]]),
        Format::R8 => Box::new(|i| [px[i]; 3]),
        Format::R8G8 => Box::new(|i| [px[2 * i], px[2 * i + 1], 0]),
        Format::R16G16B16 => Box::new(|i| [px[6 * i + 1], px[6 * i + 3], px[6 * i + 5]]),
        Format::R16G16B16A16 => Box::new(|i| [px[8 * i + 1], px[8 * i + 3], px[8 * i + 5]]),
        _ => return None,
    };
    let mut img = image::RgbImage::new(w, h);
    for (i, p) in img.pixels_mut().enumerate().take(n) {
        let c = get(i);
        *p = image::Rgb(match kind {
            "roughness" => [c[1]; 3],
            "metallic" => [c[2]; 3],
            "occlusione" => [c[0]; 3],
            _ => c,
        });
    }
    Some(img)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// glTF minimo scritto a mano: un triangolo con materiale emissivo e trasparente,
    /// una camera e una luce puntiforme.
    #[test]
    fn orthographic_camera_becomes_perspective_with_same_view() {
        let dir = std::env::temp_dir().join(format!("forge_gltf_ortho_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("o.gltf");
        // Triangolo + camera ortografica 2 m davanti, che guarda verso −Z.
        let tri: Vec<u8> = [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        std::fs::write(dir.join("o.bin"), &tri).unwrap();
        std::fs::write(
            &p,
            r#"{"asset":{"version":"2.0"},"scene":0,"scenes":[{"nodes":[0,1]}],
              "nodes":[{"mesh":0},{"camera":0,"translation":[0.0,0.0,2.0]}],
              "cameras":[{"type":"orthographic","orthographic":{"xmag":0.5,"ymag":0.4,"znear":1.0,"zfar":3.0}}],
              "meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],
              "accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[0,0,0],"max":[1,1,0]}],
              "bufferViews":[{"buffer":0,"byteLength":36}],"buffers":[{"uri":"o.bin","byteLength":36}]}"#,
        )
        .unwrap();
        let mut scene = Scene::default();
        let opts = ImportOptions {
            unit: forge_scene::LengthUnit::Meter,
            ..Default::default()
        };
        let r = import(&mut scene, &p, &opts).unwrap();
        let cam = r.camera.unwrap();
        // Stessa direzione (−Z) e centro del volume visibile (z = 0) come bersaglio.
        assert!((cam.forward() - Vec3::new(0.0, 0.0, -1.0)).length() < 1e-4, "{:?}", cam.forward());
        assert!(cam.target.length() < 1e-4, "{:?}", cam.target);
        assert!(cam.position.z > 1.0);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn truncated_glb_gives_clear_error() {
        let dir = std::env::temp_dir().join(format!("forge_glb_trunc_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("vuoto.glb");
        // Struttura di un GLB esportato con il blocco binario vuoto.
        let mut json = br#"{"asset":{"version":"2.0"},"buffers":[{"byteLength":64}]}"#.to_vec();
        while !json.len().is_multiple_of(4) {
            json.push(b' ');
        }
        let total = 12 + 8 + json.len() + 8;
        let mut glb = Vec::new();
        glb.extend(b"glTF");
        glb.extend(2u32.to_le_bytes());
        glb.extend((total as u32).to_le_bytes());
        glb.extend((json.len() as u32).to_le_bytes());
        glb.extend(b"JSON");
        glb.extend(&json);
        glb.extend(0u32.to_le_bytes());
        glb.extend(b"BIN\0");
        std::fs::write(&p, glb).unwrap();
        let e = import(&mut Scene::default(), &p, &ImportOptions::default())
            .unwrap_err()
            .to_string();
        assert!(e.contains("incompleto"), "{e}");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn imports_mesh_material_camera_light() {
        let dir = std::env::temp_dir().join(format!("forge_gltf_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut bin: Vec<u8> = Vec::new();
        for v in [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            bin.extend_from_slice(&v.to_le_bytes());
        }
        std::fs::write(dir.join("tri.bin"), &bin).unwrap();
        let json = r#"{
          "asset": {"version": "2.0"},
          "extensionsUsed": ["KHR_lights_punctual", "KHR_materials_transmission", "KHR_materials_ior"],
          "extensions": {"KHR_lights_punctual": {"lights": [{"type": "point", "intensity": 683.0, "color": [1.0, 0.5, 0.25], "name": "Lampada"}]}},
          "scene": 0,
          "scenes": [{"nodes": [0, 1, 2]}],
          "nodes": [
            {"name": "Triangolo", "mesh": 0, "translation": [0.0, 1.0, 0.0]},
            {"name": "Camera", "camera": 0, "translation": [0.0, 0.5, 5.0]},
            {"name": "Luce", "translation": [2.0, 3.0, 4.0], "extensions": {"KHR_lights_punctual": {"light": 0}}}
          ],
          "cameras": [{"type": "perspective", "perspective": {"yfov": 0.6981317, "znear": 0.1}}],
          "materials": [{"name": "Vetro", "pbrMetallicRoughness": {"baseColorFactor": [0.9, 0.95, 1.0, 1.0], "metallicFactor": 0.0, "roughnessFactor": 0.1},
                         "extensions": {"KHR_materials_transmission": {"transmissionFactor": 1.0}, "KHR_materials_ior": {"ior": 1.45}}}],
          "meshes": [{"name": "Tri", "primitives": [{"attributes": {"POSITION": 0}, "material": 0}]}],
          "buffers": [{"uri": "tri.bin", "byteLength": 36}],
          "bufferViews": [{"buffer": 0, "byteLength": 36}],
          "accessors": [{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0,0,0], "max": [1,1,0]}]
        }"#;
        let p = dir.join("scena.gltf");
        std::fs::write(&p, json).unwrap();
        let mut s = Scene::default();
        let r = import(&mut s, &p, &ImportOptions::default()).unwrap();
        assert_eq!((r.components, r.triangles), (1, 1));
        let tri = s.find(&ComponentId("scena.gltf/Triangolo/Triangolo (mesh)".into())).unwrap();
        assert!((s.bounds_of(tri).min.y - 1.0).abs() < 1e-6);
        let m = s.materials.get(s.resolve_material(tri)).unwrap();
        assert_eq!((m.transmission, m.ior), (1.0, 1.45));
        let cam = r.camera.unwrap();
        assert!((cam.position - Vec3::new(0.0, 0.5, 5.0)).length() < 1e-5);
        assert!((cam.vfov_deg - 40.0).abs() < 1e-3);
        assert!((cam.forward() - (-Vec3::Z)).length() < 1e-5);
        assert_eq!(r.lights.len(), 1);
        assert!((r.lights[0].intensity - 1.0).abs() < 1e-6, "683 cd → 1 W/sr");
        assert_eq!(r.lights[0].color, [1.0, 0.5, 0.25]);
        std::fs::remove_dir_all(dir).ok();
    }
}
