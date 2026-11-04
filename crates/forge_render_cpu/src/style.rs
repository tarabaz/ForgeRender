//! Stili non fotorealistici, alla prima superficie non speculare vista dalla camera (vetri e
//! specchi lisci riflettono e rifrangono come sempre): fumetto (cel shading) e illustrazione
//! tecnica (Gooch). I contorni sono a parte, sull'immagine finale
//! ([`forge_render_core::outline`]).

use crate::integrator::shadow;
use crate::lights::sample_light;
use forge_math::{luminance, Pcg32, Vec3};
use forge_render_core::{RenderMaterial, RenderScene, RenderStyle};

/// Luce "da disegnatore": in alto a sinistra della camera, un po' dalla parte di chi guarda.
pub fn key_light(scene: &RenderScene) -> Vec3 {
    let cam = &scene.camera;
    let f = cam.forward();
    let r = f.cross(cam.up).normalize_or(Vec3::X);
    let u = r.cross(f).normalize_or(Vec3::Y);
    (-r * 0.6 + u * 0.8 - f * 0.35).normalize()
}

/// Colore dello stile nel punto `p` (normale di shading `n`, verso la camera `wo`); `mono` =
/// bianco e nero.
#[allow(clippy::too_many_arguments)]
pub fn shade(
    scene: &RenderScene,
    style: RenderStyle,
    mono: bool,
    mat: &RenderMaterial,
    n: Vec3,
    wo: Vec3,
    p: Vec3,
    rng: &mut Pcg32,
) -> Vec3 {
    match (style, mono) {
        (RenderStyle::Photoreal, _) => Vec3::ZERO,
        // Le stesse fasce, in grigi (luminanza del colore).
        (RenderStyle::Toon, true) => Vec3::splat(luminance(toon(scene, mat, n, wo, p, rng))),
        (RenderStyle::Toon, false) => toon(scene, mat, n, wo, p, rng),
        (RenderStyle::Technical, true) => gooch_mono(scene, mat, n, wo),
        (RenderStyle::Technical, false) => gooch(scene, mat, n, wo),
    }
}

/// Illustrazione tecnica in bianco e nero: grigio scuro in ombra, quasi bianco in luce, senza
/// i colori dei materiali (come un disegno a matita).
fn gooch_mono(scene: &RenderScene, mat: &RenderMaterial, n: Vec3, wo: Vec3) -> Vec3 {
    let l = key_light(scene);
    let k = 0.5 * (1.0 + n.dot(l));
    let mut g = 0.18 + 0.74 * k;
    let h = (l + wo).normalize_or(n);
    g += 0.3 * (1.0 - mat.roughness).max(0.0) * n.dot(h).max(0.0).powf(60.0);
    Vec3::splat(g)
}

/// Fumetto: colore pieno del materiale in tre fasce (luce, mezz'ombra, ombra) dalla luce delle
/// lampade della scena con le loro ombre (pesate per intensità), più un riflesso netto sui
/// materiali lucidi. Senza lampade si usa la luce da disegnatore.
fn toon(scene: &RenderScene, mat: &RenderMaterial, n: Vec3, wo: Vec3, p: Vec3, rng: &mut Pcg32) -> Vec3 {
    let so = p + n * 1e-4;
    let (mut lit, mut total) = (0.0f32, 0.0f32);
    let mut best: Option<(f32, Vec3, f32)> = None;
    for light in scene.lights.iter().take(16) {
        let Some(ls) = sample_light(light, p, 0.5, 0.5) else { continue };
        let li = if ls.delta { ls.li } else { ls.li / ls.pdf.max(1e-8) };
        let w = luminance(li);
        if w <= 0.0 {
            continue;
        }
        total += w;
        let ndl = n.dot(ls.wi);
        if ndl <= 0.0 {
            continue;
        }
        let vis = shadow(scene, so, &ls, light.cast_shadows, rng);
        lit += w * ndl * vis;
        if vis > 0.0 && best.is_none_or(|b| w * ndl > b.0) {
            best = Some((w * ndl, ls.wi, vis));
        }
    }
    let t = if total > 0.0 {
        lit / total
    } else {
        let l = key_light(scene);
        best = Some((1.0, l, 1.0));
        n.dot(l).max(0.0)
    };
    let band = if t > 0.5 {
        1.0
    } else if t > 0.15 {
        0.6
    } else {
        0.3
    };
    let mut c = mat.base_color * band;
    // Riflesso netto: solo sui materiali lucidi, verso la luce principale.
    if let Some((_, wi, _)) = best {
        if mat.roughness < 0.5 {
            let h = (wi + wo).normalize_or(n);
            let size = 0.985 - 0.06 * mat.roughness;
            if n.dot(h) > size {
                c += Vec3::splat(0.6 * (1.0 - mat.roughness));
            }
        }
    }
    c
}

/// Illustrazione tecnica (Gooch): da freddo (bluastro, in ombra) a caldo (giallastro, in luce)
/// con la luce da disegnatore, tinti dal colore del materiale; nessuna ombra nera.
fn gooch(scene: &RenderScene, mat: &RenderMaterial, n: Vec3, wo: Vec3) -> Vec3 {
    let l = key_light(scene);
    let k = 0.5 * (1.0 + n.dot(l));
    let base = mat.base_color;
    let cool = Vec3::new(0.05, 0.1, 0.4) + base * 0.3;
    let warm = Vec3::new(0.45, 0.38, 0.08) + base * 0.6;
    let mut c = cool * (1.0 - k) + warm * k;
    let h = (l + wo).normalize_or(n);
    c += Vec3::splat(0.35 * (1.0 - mat.roughness).max(0.0) * n.dot(h).max(0.0).powf(60.0));
    c
}
