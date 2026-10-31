//! Applicazione delle texture del materiale nel punto colpito (mappe PBR) e
//! ritaglio con mappa di opacità. Le formule coincidono con `apply_textures`
//! dello shader GPU (`forge_render_gpu/src/path_tracer.wgsl`).

use forge_accel::TriangleHit;
use forge_math::{Vec2, Vec3};
use forge_render_core::texture::Texture;
use forge_render_core::{displace, MaterialTextures, RenderGeometry, RenderMaterial, RenderScene};
use forge_scene::Projection;

/// Una lettura della texture: coordinate, direzioni tangenti (∂p/∂u, ∂p/∂v) in spazio
/// mondo e peso nella miscela.
#[derive(Clone, Copy)]
struct Tap {
    uv: Vec2,
    du: Vec3,
    dv: Vec3,
    w: f32,
}

/// Letture della texture nel punto (una per gli UV, fino a tre per la proiezione box) e
/// metri per unità di UV (per la scala del rilievo).
///
/// La proiezione box è nello spazio dell'oggetto (la texture segue l'oggetto quando lo si
/// ruota o sposta) e le tre proiezioni si sfumano secondo la normale di shading `n`: sulle
/// superfici curve niente tasselli che cambiano da un triangolo all'altro.
fn taps(tex: &MaterialTextures, geo: &RenderGeometry, prim: usize, bu: f32, bv: f32, p: Vec3, n: Vec3) -> ([Tap; 3], usize, f32) {
    let tangent = geo.tri_tangents[prim];
    let flags = geo.tri_flags[prim];
    let frame = geo.node_frames.get(geo.tri_node[prim] as usize).copied().unwrap_or_default();
    let use_uv = tex.mapping.projection == Projection::Uv && tangent != Vec3::ZERO;
    // La rotazione del mapping ruota anche il riferimento tangente.
    let (sn, cs) = tex.mapping.rotation_deg.to_radians().sin_cos();
    let tap = |base: Vec2, t: Vec3, b: Vec3, is_box: bool, w: f32| Tap {
        uv: tex.mapping.apply(base, is_box),
        du: t * cs - b * sn,
        dv: t * sn + b * cs,
        w,
    };
    let none = Tap {
        uv: Vec2::ZERO,
        du: Vec3::X,
        dv: Vec3::Y,
        w: 0.0,
    };
    let mut out = [none; 3];
    // Box: una ripetizione = `scale` metri. UV: senza dimensioni reali si assume 1 m.
    let box_m = tex.mapping.scale.max(1e-6);
    // Superficie spostata con la proiezione box: punto e direzione della superficie originale
    // (ogni vertice porta l'altezza applicata e la direzione), così colore, rilievo e altezza
    // usano le stesse letture sfumate, nello stesso punto.
    let (p, n) = if flags & displace::FLAG_DISPLACED_BOX != 0 {
        let uv = geo.tri_uvs[prim];
        let v = geo.vertices(prim);
        let bw = [1.0 - bu - bv, bu, bv];
        let mut po = Vec3::ZERO;
        let mut dir = Vec3::ZERO;
        for k in 0..3 {
            let d = displace::unpack_dir(uv[k].y);
            po += (v[k] - d * uv[k].x) * bw[k];
            dir += d * bw[k];
        }
        (po, dir.normalize_or(n))
    } else {
        (p, n)
    };
    if use_uv && flags & displace::FLAG_DISPLACED_BOX == 0 {
        let [a, b, c] = geo.tri_uvs[prim];
        let ng = geo.geometric_normals[prim];
        let t = tangent.normalize_or_zero();
        out[0] = tap(a * (1.0 - bu - bv) + b * bu + c * bv, t, ng.cross(t), false, 1.0);
        (out, 1, 1.0)
    } else {
        let pl = frame.to_local(p);
        let w = displace::box_weights(frame.dir_to_local(n));
        let mut k = 0;
        for (axis, &wa) in w.iter().enumerate() {
            if wa > 0.0 {
                let (t, b) = displace::box_frame(axis);
                out[k] = tap(
                    displace::box_coords(pl, axis),
                    frame.dir_to_world(t),
                    frame.dir_to_world(b),
                    true,
                    wa,
                );
                k += 1;
            }
        }
        (out, k.max(1), box_m)
    }
}

/// Media pesata delle letture della texture.
#[inline]
fn sample(t: &Texture, taps: &[Tap]) -> Vec3 {
    if let [one] = taps {
        return t.sample(one.uv);
    }
    taps.iter().map(|k| t.sample(k.uv) * k.w).sum()
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
    let (all, count, m_per_uv) = taps(tex, geo, hit.prim as usize, hit.u, hit.v, p, *ns);
    let taps = &all[..count];
    if let Some(t) = &tex.base_color {
        mat.base_color *= sample(t, taps);
    }
    if let Some(t) = &tex.ao {
        let ao = sample(t, taps).x;
        mat.base_color *= 1.0 + (ao - 1.0) * tex.ao_strength;
    }
    if let Some(t) = &tex.roughness {
        let mut r = sample(t, taps).x;
        if tex.roughness_is_gloss {
            r = 1.0 - r;
        }
        let r = (r * mat.roughness).clamp(0.0, 1.0);
        mat.alpha = (r * r).max(1e-3);
    }
    if let Some(t) = &tex.metallic {
        mat.metallic = (mat.metallic * sample(t, taps).x).clamp(0.0, 1.0);
    }
    let bump = tex.height.as_ref().filter(|_| tex.bump_m > 0.0);
    if tex.normal.is_none() && bump.is_none() {
        return;
    }
    // Normal map e rilievo per ogni proiezione, poi le normali si sfumano con i pesi.
    let n0 = *ns;
    let mut blended = Vec3::ZERO;
    for k in taps {
        let mut n = n0;
        if let Some(t) = &tex.normal {
            let mut c = t.sample(k.uv) * 2.0 - Vec3::ONE;
            if tex.normal_directx {
                c.y = -c.y;
            }
            if let Some((tt, bb)) = tangent_frame(n, k.du, k.dv) {
                n = (tt * (c.x * tex.normal_strength) + bb * (c.y * tex.normal_strength) + n * c.z.max(1e-3)).normalize_or(n);
            }
        }
        if let Some(h) = bump {
            // Bump mapping: la normale si inclina contro il gradiente dell'altezza (in metri).
            let (gu, gv) = height_gradient(h, k.uv);
            if let Some((tt, bb)) = tangent_frame(n, k.du, k.dv) {
                let s = tex.bump_m / m_per_uv;
                n = (n - tt * (gu * s) - bb * (gv * s)).normalize_or(n);
            }
        }
        blended += n * k.w;
    }
    *ns = blended.normalize_or(n0);
}

/// Vero se il punto è opaco (nessuna mappa di opacità, oppure opacità ≥ 50 %).
#[inline]
pub fn opaque_at(scene: &RenderScene, prim: u32, t: f32, u: f32, v: f32, o: Vec3, d: Vec3) -> bool {
    let mi = scene.material_index(prim as usize);
    let Some(tex) = &scene.material_textures[mi] else { return true };
    let Some(op) = &tex.opacity else { return true };
    let geo = &scene.geometry;
    let [n0, n1, n2] = geo.shading_normals[prim as usize];
    let n = (n0 * (1.0 - u - v) + n1 * u + n2 * v).normalize_or(geo.geometric_normals[prim as usize]);
    let (all, count, _) = taps(tex, geo, prim as usize, u, v, o + d * t, n);
    sample(op, &all[..count]).x >= 0.5
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
