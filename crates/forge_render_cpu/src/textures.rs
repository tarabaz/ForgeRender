//! Applicazione delle texture del materiale nel punto colpito (mappe PBR) e
//! ritaglio con mappa di opacità. Le formule coincidono con `apply_textures`
//! dello shader GPU (`forge_render_gpu/src/path_tracer.wgsl`).

use forge_accel::TriangleHit;
use forge_math::{Vec2, Vec3};
use forge_render_core::texture::Texture;
use forge_render_core::{MaterialTextures, RenderGeometry, RenderMaterial, RenderScene};
use forge_scene::Projection;

/// Coordinate di texture e direzioni tangenti (∂p/∂u, ∂p/∂v) in spazio mondo,
/// più i metri per unità di UV (per la scala del rilievo).
fn coordinates(tex: &MaterialTextures, geo: &RenderGeometry, prim: usize, bu: f32, bv: f32, p: Vec3) -> (Vec2, Vec3, Vec3, f32) {
    let tangent = geo.tri_tangents[prim];
    let use_uv = tex.mapping.projection == Projection::Uv && tangent != Vec3::ZERO;
    let (base, t, b, is_box) = if use_uv {
        let [a, b, c] = geo.tri_uvs[prim];
        let uv = a * (1.0 - bu - bv) + b * bu + c * bv;
        let n = geo.geometric_normals[prim];
        let t = tangent.normalize_or_zero();
        (uv, t, n.cross(t), false)
    } else {
        // Proiezione cubica lungo l'asse dominante della normale geometrica (in metri).
        let n = geo.geometric_normals[prim].abs();
        if n.x >= n.y && n.x >= n.z {
            (Vec2::new(p.z, p.y), Vec3::Z, Vec3::Y, true)
        } else if n.y >= n.z {
            (Vec2::new(p.x, p.z), Vec3::X, Vec3::Z, true)
        } else {
            (Vec2::new(p.x, p.y), Vec3::X, Vec3::Y, true)
        }
    };
    let uv = tex.mapping.apply(base, is_box);
    // Box: una ripetizione = `scale` metri. UV: senza dimensioni reali si assume 1 m.
    let meters_per_uv = if is_box { tex.mapping.scale.max(1e-6) } else { 1.0 };
    // La rotazione del mapping ruota anche il riferimento tangente.
    let (sn, cs) = tex.mapping.rotation_deg.to_radians().sin_cos();
    (uv, t * cs - b * sn, t * sn + b * cs, meters_per_uv)
}

/// Gradiente della mappa height (per unità di UV), con differenze centrate di un texel.
fn height_gradient(h: &Texture, uv: Vec2) -> (f32, f32) {
    let (eu, ev) = (1.0 / h.width.max(1) as f32, 1.0 / h.height.max(1) as f32);
    let s = |du: f32, dv: f32| h.sample(uv + Vec2::new(du, dv)).x;
    ((s(eu, 0.0) - s(-eu, 0.0)) / (2.0 * eu), (s(0.0, ev) - s(0.0, -ev)) / (2.0 * ev))
}

/// Riferimento tangente ortonormale attorno a `n`, orientato come (du, dv).
fn tangent_frame(n: Vec3, du: Vec3, dv: Vec3) -> Option<(Vec3, Vec3)> {
    let tt = (du - n * du.dot(n)).normalize_or_zero();
    if tt == Vec3::ZERO {
        return None;
    }
    let mut bb = n.cross(tt);
    if bb.dot(dv) < 0.0 {
        bb = -bb;
    }
    Some((tt, bb))
}

/// Modifica il materiale locale e la normale di shading secondo le texture.
pub fn apply(tex: &MaterialTextures, geo: &RenderGeometry, hit: &TriangleHit, p: Vec3, mat: &mut RenderMaterial, ns: &mut Vec3) {
    let (uv, du, dv, m_per_uv) = coordinates(tex, geo, hit.prim as usize, hit.u, hit.v, p);
    if let Some(t) = &tex.base_color {
        mat.base_color *= t.sample(uv);
    }
    if let Some(t) = &tex.ao {
        let ao = t.sample(uv).x;
        mat.base_color *= 1.0 + (ao - 1.0) * tex.ao_strength;
    }
    if let Some(t) = &tex.roughness {
        let mut r = t.sample(uv).x;
        if tex.roughness_is_gloss {
            r = 1.0 - r;
        }
        let r = (r * mat.roughness).clamp(0.0, 1.0);
        mat.alpha = (r * r).max(1e-3);
    }
    if let Some(t) = &tex.metallic {
        mat.metallic = (mat.metallic * t.sample(uv).x).clamp(0.0, 1.0);
    }
    if let Some(t) = &tex.normal {
        let mut c = t.sample(uv) * 2.0 - Vec3::ONE;
        if tex.normal_directx {
            c.y = -c.y;
        }
        if let Some((tt, bb)) = tangent_frame(*ns, du, dv) {
            let n = *ns;
            *ns = (tt * (c.x * tex.normal_strength) + bb * (c.y * tex.normal_strength) + n * c.z.max(1e-3)).normalize_or(n);
        }
    }
    if let Some(h) = tex.height.as_ref().filter(|_| tex.bump_m > 0.0) {
        // Bump mapping: la normale si inclina contro il gradiente dell'altezza (in metri).
        let (gu, gv) = height_gradient(h, uv);
        if let Some((tt, bb)) = tangent_frame(*ns, du, dv) {
            let k = tex.bump_m / m_per_uv;
            let n = *ns;
            *ns = (n - tt * (gu * k) - bb * (gv * k)).normalize_or(n);
        }
    }
}

/// Vero se il punto è opaco (nessuna mappa di opacità, oppure opacità ≥ 50 %).
#[inline]
pub fn opaque_at(scene: &RenderScene, prim: u32, t: f32, u: f32, v: f32, o: Vec3, d: Vec3) -> bool {
    let mi = scene.material_index(prim as usize);
    let Some(tex) = &scene.material_textures[mi] else { return true };
    let Some(op) = &tex.opacity else { return true };
    let (uv, ..) = coordinates(tex, &scene.geometry, prim as usize, u, v, o + d * t);
    op.sample(uv).x >= 0.5
}

/// Intersezione più vicina, attraversando le parti ritagliate dalle mappe di opacità.
#[inline]
pub fn intersect(scene: &RenderScene, o: Vec3, d: Vec3, t_min: f32, t_max: f32) -> Option<TriangleHit> {
    let accel = &scene.geometry.accel;
    if scene.has_cutout {
        accel.intersect_filtered(o, d, t_min, t_max, |prim, t, u, v| opaque_at(scene, prim, t, u, v, o, d))
    } else {
        accel.intersect(o, d, t_min, t_max)
    }
}

/// Raggio d'ombra: le parti ritagliate lasciano passare la luce.
#[inline]
pub fn occluded(scene: &RenderScene, o: Vec3, d: Vec3, t_min: f32, t_max: f32) -> bool {
    let accel = &scene.geometry.accel;
    if scene.has_cutout {
        accel.occluded_filtered(o, d, t_min, t_max, |prim, t, u, v| opaque_at(scene, prim, t, u, v, o, d))
    } else {
        accel.occluded(o, d, t_min, t_max)
    }
}
