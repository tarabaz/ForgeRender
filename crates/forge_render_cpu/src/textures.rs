//! Applicazione delle texture del materiale nel punto colpito.

use forge_accel::TriangleHit;
use forge_math::{Vec2, Vec3};
use forge_render_core::{MaterialTextures, RenderGeometry, RenderMaterial};
use forge_scene::Projection;

/// Coordinate di texture e direzioni tangenti (∂p/∂u, ∂p/∂v) in spazio mondo.
fn coordinates(tex: &MaterialTextures, geo: &RenderGeometry, prim: usize, bu: f32, bv: f32, p: Vec3) -> (Vec2, Vec3, Vec3) {
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
    // La rotazione del mapping ruota anche il riferimento tangente.
    let (sn, cs) = tex.mapping.rotation_deg.to_radians().sin_cos();
    (uv, t * cs - b * sn, t * sn + b * cs)
}

/// Modifica il materiale locale e la normale di shading secondo le texture.
pub fn apply(tex: &MaterialTextures, geo: &RenderGeometry, hit: &TriangleHit, p: Vec3, mat: &mut RenderMaterial, ns: &mut Vec3) {
    let (uv, du, dv) = coordinates(tex, geo, hit.prim as usize, hit.u, hit.v, p);
    if let Some(t) = &tex.base_color {
        mat.base_color *= t.sample(uv);
    }
    if let Some(t) = &tex.roughness {
        let r = (t.sample(uv).x * mat.roughness).clamp(0.0, 1.0);
        mat.alpha = (r * r).max(1e-3);
    }
    if let Some(t) = &tex.normal {
        let c = t.sample(uv) * 2.0 - Vec3::ONE;
        let n = *ns;
        // Riferimento tangente ortonormale attorno alla normale di shading.
        let tt = (du - n * du.dot(n)).normalize_or_zero();
        if tt != Vec3::ZERO {
            let mut bb = n.cross(tt);
            if bb.dot(dv) < 0.0 {
                bb = -bb;
            }
            let perturbed = (tt * (c.x * tex.normal_strength) + bb * (c.y * tex.normal_strength) + n * c.z.max(1e-3)).normalize_or(n);
            *ns = perturbed;
        }
    }
}
