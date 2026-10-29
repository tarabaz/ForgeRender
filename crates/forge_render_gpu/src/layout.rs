//! Serializzazione della [`RenderScene`] in un unico buffer di parole a 32 bit
//! ("bindless"): BVH, triangoli, primitive, materiali, texture, luci, HDRI,
//! emissivi e tabella GGX stanno in un solo storage buffer, indirizzato con gli
//! offset scritti nei parametri. Lo shader (`path_tracer.wgsl`) legge le stesse
//! strutture con lo stesso layout.

use forge_render_core::{RenderLightKind, RenderScene};
use forge_scene::{Environment, Projection};
use std::collections::HashMap;
use std::sync::Arc;

/// Numero di parole dei parametri (uniform `array<vec4<u32>, 20>`).
pub const PARAM_WORDS: usize = 80;

// Indici dei parametri (devono coincidere con le costanti dello shader).
pub const P_WIDTH: usize = 0;
pub const P_HEIGHT: usize = 1;
pub const P_FIRST_SAMPLE: usize = 2;
pub const P_N_SAMPLES: usize = 3;
pub const P_SEED_LO: usize = 4;
pub const P_SEED_HI: usize = 5;
pub const P_MAX_BOUNCES: usize = 6;
pub const P_RR_START: usize = 7;
pub const P_FLAGS: usize = 8;
pub const P_N_LIGHTS: usize = 9;
pub const P_ENV_KIND: usize = 10;
pub const P_N_EMISSIVE: usize = 11;
pub const P_CLAMP: usize = 12;
pub const P_ENV_STRENGTH: usize = 13;
pub const P_ENV_ROT: usize = 14;
pub const P_N_NODES: usize = 15;
pub const P_CAM_POS: usize = 16;
pub const P_TAN_HALF: usize = 19;
pub const P_CAM_R: usize = 20;
pub const P_ASPECT: usize = 23;
pub const P_CAM_U: usize = 24;
pub const P_CAM_F: usize = 28;
pub const P_ENV_C0: usize = 32;
pub const P_ENV_C1: usize = 36;
pub const P_ENV_C2: usize = 40;
pub const P_OFF_NODES: usize = 44;
pub const P_OFF_TRIS: usize = 45;
pub const P_OFF_PRIMS: usize = 46;
pub const P_OFF_MATS: usize = 47;
pub const P_OFF_TEXINFO: usize = 48;
pub const P_OFF_TEXPIX: usize = 49;
pub const P_OFF_LIGHTS: usize = 50;
pub const P_OFF_ENVPIX: usize = 51;
pub const P_OFF_ENV_MFUNC: usize = 52;
pub const P_OFF_ENV_MCDF: usize = 53;
pub const P_OFF_ENV_CFUNC: usize = 54;
pub const P_OFF_ENV_CCDF: usize = 55;
pub const P_OFF_ENV_CINT: usize = 56;
pub const P_OFF_EM_PRIMS: usize = 57;
pub const P_OFF_EM_FUNC: usize = 58;
pub const P_OFF_EM_CDF: usize = 59;
pub const P_OFF_EM_AREA: usize = 60;
pub const P_OFF_MSLUT: usize = 61;
pub const P_ENV_W: usize = 62;
pub const P_ENV_H: usize = 63;
pub const P_ENV_MINT: usize = 64;
pub const P_EM_INTEGRAL: usize = 65;
pub const P_N_SELECT: usize = 66;
/// 1 se qualche materiale ha una mappa di opacità (ritaglio nelle intersezioni).
pub const P_HAS_CUTOUT: usize = 67;
// Obiettivo: raggio della lente (0 = pinhole), fuoco, lamelle, rotazione, decentramento.
pub const P_LENS_R: usize = 68;
pub const P_FOCUS: usize = 69;
pub const P_BLADES: usize = 70;
pub const P_BLADE_ROT: usize = 71;
pub const P_SHIFT_X: usize = 72;
pub const P_SHIFT_Y: usize = 73;
/// Nebbie volumetriche: offset delle intestazioni e numero.
pub const P_OFF_FOGS: usize = 74;
pub const P_N_FOGS: usize = 75;
/// Portali di luce: offset, numero, area totale.
pub const P_OFF_PORTALS: usize = 76;
pub const P_N_PORTALS: usize = 77;
pub const P_PORTAL_AREA: usize = 78;
/// Campionamento adattivo: riquadri per riga della maschera (attiva col bit 2 di P_FLAGS).
pub const P_TILES_X: usize = 79;
/// Portale: centro(3), mezza larghezza, normale(3), mezza altezza, u(3), 0, v(3), 0.
pub const PORTAL_WORDS: usize = 16;

pub const NODE_WORDS: usize = 8;
pub const TRI_WORDS: usize = 12;
pub const PRIM_WORDS: usize = 28;
/// Materiale: … 30–32 assorbimento del volume (1/m), 33 priorità, 34–35 liberi.
pub const MAT_WORDS: usize = 36;
pub const LIGHT_WORDS: usize = 24;
/// Intestazione di una nebbia: min(3), lato cella, risoluzione(3), unità di densità,
/// albedo(3), anisotropia, offset dei dati (u16 a coppie), offset dei majorant per blocco,
/// lato del blocco in celle, 1 libera.
pub const FOG_WORDS: usize = 16;

pub struct SceneBuffer {
    pub words: Vec<u32>,
    /// Parametri che dipendono solo dalla scena (offset, conteggi, ambiente).
    pub params: [u32; PARAM_WORDS],
}

struct Writer {
    w: Vec<u32>,
}

impl Writer {
    fn f(&mut self, v: f32) {
        self.w.push(v.to_bits());
    }
    fn u(&mut self, v: u32) {
        self.w.push(v);
    }
    fn v3(&mut self, v: forge_math::Vec3) {
        self.f(v.x);
        self.f(v.y);
        self.f(v.z);
    }
    fn pos(&self) -> u32 {
        self.w.len() as u32
    }
}

fn set_f(p: &mut [u32; PARAM_WORDS], i: usize, v: f32) {
    p[i] = v.to_bits();
}

pub fn build(scene: &RenderScene) -> SceneBuffer {
    let geo = &*scene.geometry;
    let mut p = [0u32; PARAM_WORDS];
    let mut w = Writer {
        w: Vec::with_capacity(geo.tri_node.len() * (TRI_WORDS + PRIM_WORDS) + geo.accel.bvh.nodes.len() * NODE_WORDS),
    };

    // --- BVH ---
    p[P_OFF_NODES] = w.pos();
    p[P_N_NODES] = geo.accel.bvh.nodes.len() as u32;
    for n in &geo.accel.bvh.nodes {
        for v in n.min {
            w.f(v);
        }
        w.u(n.left_or_first);
        for v in n.max {
            w.f(v);
        }
        w.u(n.count);
    }
    assert_eq!(w.pos() as usize - p[P_OFF_NODES] as usize, geo.accel.bvh.nodes.len() * NODE_WORDS);
    // --- Triangoli in ordine BVH (con l'indice della primitiva originale) ---
    p[P_OFF_TRIS] = w.pos();
    for (slot, t) in geo.accel.tris.iter().enumerate() {
        w.v3(t.v0);
        w.u(geo.accel.bvh.prim_indices[slot]);
        w.v3(t.e1);
        w.u(0);
        w.v3(t.e2);
        w.u(0);
    }
    assert_eq!(w.pos() as usize - p[P_OFF_TRIS] as usize, geo.accel.tris.len() * TRI_WORDS);
    // --- Primitive in ordine originale ---
    p[P_OFF_PRIMS] = w.pos();
    for prim in 0..geo.tri_node.len() {
        let [n0, n1, n2] = geo.shading_normals[prim];
        w.v3(n0);
        w.u(scene.material_index(prim) as u32);
        w.v3(n1);
        w.u(0);
        w.v3(n2);
        w.u(0);
        w.v3(geo.geometric_normals[prim]);
        w.u(0);
        for uv in geo.tri_uvs[prim] {
            w.f(uv.x);
            w.f(uv.y);
        }
        w.v3(geo.tri_tangents[prim]);
        let em = scene.emissive.as_ref().map(|e| e.index_of_prim[prim]).unwrap_or(u32::MAX);
        w.u(em);
        // Posizione del triangolo nell'ordine BVH (per campionare i vertici degli emissivi).
        w.u(geo.prim_to_slot[prim]);
        // Flag (coordinate box della superficie originale per il displacement).
        w.u(geo.tri_flags[prim]);
    }
    assert_eq!(w.pos() as usize - p[P_OFF_PRIMS] as usize, geo.tri_node.len() * PRIM_WORDS);

    // --- Texture (deduplicate per identità) ---
    let mut tex_index: HashMap<*const forge_render_core::texture::Texture, i32> = HashMap::new();
    let mut textures: Vec<Arc<forge_render_core::texture::Texture>> = Vec::new();
    let mut tex_of = |t: &Option<Arc<forge_render_core::texture::Texture>>| -> i32 {
        match t {
            None => -1,
            Some(t) => *tex_index.entry(Arc::as_ptr(t)).or_insert_with(|| {
                textures.push(t.clone());
                (textures.len() - 1) as i32
            }),
        }
    };

    // --- Materiali ---
    p[P_OFF_MATS] = w.pos();
    for (i, m) in scene.materials.iter().enumerate() {
        let _ = i;
        let tx = scene.material_textures.get(i).and_then(|t| t.clone());
        let (tb, tr, tn, ns, mapping) = match &tx {
            Some(t) => (
                tex_of(&t.base_color),
                tex_of(&t.roughness),
                tex_of(&t.normal),
                t.normal_strength,
                Some(t.mapping),
            ),
            None => (-1, -1, -1, 1.0, None),
        };
        let (tm, th, ta, to, flags, bump, ao_s) = match &tx {
            Some(t) => (
                tex_of(&t.metallic),
                tex_of(&t.height),
                tex_of(&t.ao),
                tex_of(&t.opacity),
                t.normal_directx as u32 | (t.roughness_is_gloss as u32) << 1,
                t.bump_m,
                t.ao_strength,
            ),
            None => (-1, -1, -1, -1, 0, 0.0, 0.0),
        };
        w.v3(m.base_color);
        w.f(m.metallic);
        w.f(m.alpha);
        w.f(m.ior);
        w.f(m.f0_dielectric);
        w.f(m.specular_scale);
        w.f(m.transmission);
        w.f(m.roughness);
        w.v3(m.emission);
        w.u(tb as u32);
        w.u(tr as u32);
        w.u(tn as u32);
        w.f(ns);
        let mp = mapping.unwrap_or_default();
        w.u(if mp.projection == Projection::Box { 1 } else { 0 });
        w.f(mp.scale);
        let (sn, cs) = mp.rotation_deg.to_radians().sin_cos();
        w.f(sn);
        w.f(cs);
        w.f(mp.offset[0]);
        w.f(mp.offset[1]);
        w.u(tm as u32);
        w.u(th as u32);
        w.u(ta as u32);
        w.u(to as u32);
        w.u(flags);
        w.f(bump);
        w.f(ao_s);
        w.v3(m.sigma_a);
        w.f(m.priority);
        w.u(0);
        w.u(0);
        assert_eq!(
            (w.pos() - p[P_OFF_MATS]) as usize,
            (i + 1) * MAT_WORDS,
            "record materiale di lunghezza errata"
        );
    }
    p[P_OFF_TEXINFO] = w.pos();
    let info_pos = w.pos() as usize;
    for _ in &textures {
        for _ in 0..4 {
            w.u(0);
        }
    }
    p[P_OFF_TEXPIX] = w.pos();
    for (k, t) in textures.iter().enumerate() {
        let off = w.pos() - p[P_OFF_TEXPIX];
        w.w[info_pos + 4 * k] = off;
        w.w[info_pos + 4 * k + 1] = t.width as u32;
        w.w[info_pos + 4 * k + 2] = t.height as u32;
        w.w[info_pos + 4 * k + 3] = t.is_srgb() as u32;
        for px in t.data() {
            w.u(u32::from_le_bytes([px[0], px[1], px[2], 255]));
        }
    }

    // --- Luci ---
    p[P_OFF_LIGHTS] = w.pos();
    p[P_N_LIGHTS] = scene.lights.len() as u32;
    for l in &scene.lights {
        let z = forge_math::Vec3::ZERO;
        let (kind, a, b, c, d, hw, hh, area, ci, co) = match l.kind {
            RenderLightKind::Point { position } => (0, position, z, z, z, 0.0, 0.0, 0.0, 0.0, 0.0),
            RenderLightKind::Spot {
                position,
                direction,
                cos_inner,
                cos_outer,
            } => (1, position, direction, z, z, 0.0, 0.0, 0.0, cos_inner, cos_outer),
            RenderLightKind::Directional { direction } => (2, z, direction, z, z, 0.0, 0.0, 0.0, 0.0, 0.0),
            RenderLightKind::Area {
                center,
                normal,
                u,
                v,
                half_w,
                half_h,
                area,
            } => (3, center, normal, u, v, half_w, half_h, area, 0.0, 0.0),
        };
        w.u(kind);
        w.u(l.cast_shadows as u32);
        w.u(l.visible_to_camera as u32);
        w.u(0);
        w.v3(l.emission);
        w.f(hw);
        w.v3(a);
        w.f(hh);
        w.v3(b);
        w.f(area);
        w.v3(c);
        w.f(ci);
        w.v3(d);
        w.f(co);
    }
    assert_eq!(w.pos() as usize - p[P_OFF_LIGHTS] as usize, scene.lights.len() * LIGHT_WORDS);

    // --- Portali ---
    p[P_OFF_PORTALS] = w.pos();
    p[P_N_PORTALS] = scene.portals.list.len() as u32;
    set_f(&mut p, P_PORTAL_AREA, scene.portals.total_area);
    for q in &scene.portals.list {
        w.v3(q.center);
        w.f(q.half_w);
        w.v3(q.normal);
        w.f(q.half_h);
        w.v3(q.u);
        w.f(0.0);
        w.v3(q.v);
        w.f(0.0);
    }

    // --- Ambiente ---
    match &scene.environment {
        Environment::Uniform { color, strength } => {
            p[P_ENV_KIND] = 0;
            set_f(&mut p, P_ENV_STRENGTH, *strength);
            for (k, c) in color.iter().enumerate() {
                set_f(&mut p, P_ENV_C0 + k, *c);
            }
        }
        Environment::Gradient {
            zenith,
            horizon,
            ground,
            strength,
        } => {
            p[P_ENV_KIND] = 1;
            set_f(&mut p, P_ENV_STRENGTH, *strength);
            for k in 0..3 {
                set_f(&mut p, P_ENV_C0 + k, zenith[k]);
                set_f(&mut p, P_ENV_C1 + k, horizon[k]);
                set_f(&mut p, P_ENV_C2 + k, ground[k]);
            }
        }
        Environment::Hdri { .. } => {}
    }
    if let Some((map, strength, rot)) = &scene.env_map {
        p[P_ENV_KIND] = 2;
        set_f(&mut p, P_ENV_STRENGTH, *strength);
        set_f(&mut p, P_ENV_ROT, *rot);
        p[P_ENV_W] = map.width as u32;
        p[P_ENV_H] = map.height as u32;
        p[P_OFF_ENVPIX] = w.pos();
        for px in map.pixels() {
            w.v3(*px);
        }
        p[P_OFF_ENV_MFUNC] = w.pos();
        for &v in map.marginal().func() {
            w.f(v);
        }
        p[P_OFF_ENV_MCDF] = w.pos();
        for &v in map.marginal().cdf() {
            w.f(v);
        }
        set_f(&mut p, P_ENV_MINT, map.marginal().integral());
        p[P_OFF_ENV_CFUNC] = w.pos();
        for d in map.conditional() {
            for &v in d.func() {
                w.f(v);
            }
        }
        p[P_OFF_ENV_CCDF] = w.pos();
        for d in map.conditional() {
            for &v in d.cdf() {
                w.f(v);
            }
        }
        p[P_OFF_ENV_CINT] = w.pos();
        for d in map.conditional() {
            w.f(d.integral());
        }
    }

    // --- Superfici emissive ---
    if let Some(em) = &scene.emissive {
        p[P_N_EMISSIVE] = em.prims.len() as u32;
        p[P_OFF_EM_PRIMS] = w.pos();
        for &x in &em.prims {
            w.u(x);
        }
        p[P_OFF_EM_FUNC] = w.pos();
        for &x in em.distribution.func() {
            w.f(x);
        }
        p[P_OFF_EM_CDF] = w.pos();
        for &x in em.distribution.cdf() {
            w.f(x);
        }
        p[P_OFF_EM_AREA] = w.pos();
        for &x in &em.areas {
            w.f(x);
        }
        set_f(&mut p, P_EM_INTEGRAL, em.distribution.integral());
    }

    // --- Tabella della compensazione multi-scattering GGX ---
    p[P_OFF_MSLUT] = w.pos();
    for &x in forge_render_cpu::bsdf::ggx_albedo_table() {
        w.f(x);
    }

    // --- Nebbie: intestazioni, poi densità a 16 bit (due per parola) e majorant per blocco ---
    p[P_N_FOGS] = scene.fogs.len() as u32;
    p[P_OFF_FOGS] = w.pos();
    let header_at = w.pos() as usize;
    for _ in 0..scene.fogs.len() * FOG_WORDS {
        w.u(0);
    }
    for (k, f) in scene.fogs.iter().enumerate() {
        let off_data = w.pos();
        for pair in f.data.chunks(2) {
            w.u(pair[0] as u32 | (*pair.get(1).unwrap_or(&0) as u32) << 16);
        }
        let off_major = w.pos();
        for &m in &f.macro_majorant {
            w.f(m);
        }
        let h = &mut w.w[header_at + k * FOG_WORDS..header_at + (k + 1) * FOG_WORDS];
        let g = &f.spec;
        let words = [
            g.min.x.to_bits(),
            g.min.y.to_bits(),
            g.min.z.to_bits(),
            g.cell.to_bits(),
            g.res[0] as u32,
            g.res[1] as u32,
            g.res[2] as u32,
            f.unit.to_bits(),
            f.albedo.x.to_bits(),
            f.albedo.y.to_bits(),
            f.albedo.z.to_bits(),
            f.anisotropy.to_bits(),
            off_data,
            off_major,
            forge_volume::MACRO_CELLS as u32,
            0,
        ];
        h.copy_from_slice(&words);
    }

    p[P_N_SELECT] = scene.lights.len() as u32 + scene.samples_environment() as u32 + scene.emissive.is_some() as u32;
    p[P_HAS_CUTOUT] = scene.has_cutout as u32;
    if w.w.is_empty() {
        w.u(0);
    }
    SceneBuffer { words: w.w, params: p }
}
