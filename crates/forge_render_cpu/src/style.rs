//! Stili non fotorealistici, alla prima superficie non speculare vista dalla camera (vetri e
//! specchi lisci riflettono e rifrangono come sempre): fumetto (cel shading) e illustrazione
//! tecnica (Gooch). I contorni sono a parte, sull'immagine finale
//! ([`forge_render_core::outline`]).

use crate::integrator::shadow;
use crate::lights::{intersect_area, sample_light};
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

/// Riflesso netto del fumetto: c'è dove la direzione speculare (lo specchio di `wo` sulla
/// normale) vede davvero la lampada, come nel render fotorealistico. Per le luci ad area si usa
/// la loro forma (il riflesso di una softbox è la softbox); per le altre un piccolo cerchio
/// attorno alla direzione della luce, più largo sui materiali meno lucidi.
fn sees_light(light: &forge_render_core::RenderLight, p: Vec3, n: Vec3, wo: Vec3, wi: Vec3, roughness: f32) -> bool {
    let r = (n * (2.0 * n.dot(wo)) - wo).normalize_or(n);
    if r.dot(n) <= 0.0 {
        return false;
    }
    if matches!(light.kind, forge_render_core::RenderLightKind::Area { .. }) {
        return intersect_area(light, p + n * 1e-4, r, f32::INFINITY).is_some_and(|(_, e, _)| e != Vec3::ZERO);
    }
    let a = roughness * roughness;
    let angle = (1.5 + 30.0 * a).to_radians();
    r.dot(wi) > angle.cos()
}

/// Forza del riflesso netto: la riflettanza del materiale (circa 4 % per plastiche e vernici, il
/// colore per i metalli), resa visibile come nel disegno a fasce ma senza superare il bianco
/// pieno, e più debole sui materiali meno lucidi.
fn toon_highlight(mat: &RenderMaterial) -> Vec3 {
    let f0d = (mat.specular_scale * mat.f0_dielectric).min(1.0);
    let f0 = Vec3::splat(f0d) * (1.0 - mat.metallic) + mat.base_color * mat.metallic;
    (f0 * 7.5).min(Vec3::ONE) * (0.6 * (1.0 - mat.roughness / 0.5).max(0.0))
}

/// Fumetto: colore pieno del materiale in tre fasce (luce, mezz'ombra, ombra) dalla luce delle
/// lampade della scena con le loro ombre (pesate per intensità), più un riflesso netto sui
/// materiali lucidi dove la lampada si riflette davvero ([`sees_light`]), forte quanto riflette
/// il materiale ([`toon_highlight`]). Senza lampade si usa la luce da disegnatore.
fn toon(scene: &RenderScene, mat: &RenderMaterial, n: Vec3, wo: Vec3, p: Vec3, rng: &mut Pcg32) -> Vec3 {
    let so = p + n * 1e-4;
    let (mut lit, mut total) = (0.0f32, 0.0f32);
    let glossy = mat.roughness < 0.5;
    let mut highlight = false;
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
        if glossy && vis > 0.0 && !highlight {
            highlight = sees_light(light, p, n, wo, ls.wi, mat.roughness);
        }
    }
    let t = if total > 0.0 {
        lit / total
    } else {
        let l = key_light(scene);
        // Luce da disegnatore: riflesso nella sua direzione.
        let r = (n * (2.0 * n.dot(wo)) - wo).normalize_or(n);
        let a = mat.roughness * mat.roughness;
        highlight = glossy && r.dot(l) > (1.5 + 30.0 * a).to_radians().cos();
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
    if highlight {
        c += toon_highlight(mat);
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

#[cfg(test)]
mod tests {
    use super::*;
    use forge_scene::{Material, MaterialId};

    fn mat(base: f32, metallic: f32, roughness: f32) -> RenderMaterial {
        RenderMaterial::from_material(&Material {
            id: MaterialId::new("m"),
            base_color: [base; 3],
            metallic,
            roughness,
            ..Default::default()
        })
    }

    /// Il riflesso del fumetto è forte quanto riflette il materiale: su una plastica scura e
    /// lucida è un grigio tenue (il 4 % di riflettanza), non la macchia bianca di prima (+0,6);
    /// sui metalli lucidi resta pieno; sui materiali ruvidi sparisce.
    #[test]
    fn toon_highlight_follows_the_reflectance() {
        let dark_plastic = toon_highlight(&mat(0.02, 0.0, 0.1));
        assert!(dark_plastic.x > 0.05 && dark_plastic.x < 0.2, "{dark_plastic:?}");
        let chrome = toon_highlight(&mat(0.9, 1.0, 0.05));
        assert!(chrome.x > 0.5, "{chrome:?}");
        assert_eq!(toon_highlight(&mat(0.5, 0.0, 0.6)), Vec3::ZERO);
    }
}
