//! Campionamento adattivo: stessa immagine del rendering uniforme (non distorto), molti
//! meno campioni a parità di qualità, fermata automatica quando tutto è sotto soglia.
//! Sono anche i "test sul rumore" di riferimento: errore misurato contro un'immagine
//! ad alti campioni.

use forge_render_core::{RenderScene, RenderSession, RenderSettings, SessionState};
use forge_render_cpu::CpuBackend;
use forge_scene::Scene;
use std::sync::Arc;
use std::time::Duration;

struct Run {
    image: Vec<[f32; 3]>,
    samples: u32,
    pixel_samples: u64,
    state: SessionState,
}

fn run(scene: &Arc<RenderScene>, spp: u32, threshold: f32, seed: u64) -> Run {
    let s = RenderSession::new();
    let st = RenderSettings {
        width: 96,
        height: 54,
        target_samples: spp,
        noise_threshold: threshold,
        adaptive_min_samples: 16,
        seed,
        ..Default::default()
    };
    let id = s.start(scene.clone(), st, Box::new(CpuBackend::new(4).unwrap()));
    let p = s.wait(id, Duration::from_secs(600));
    std::thread::sleep(Duration::from_millis(20));
    let f = s.latest_frame().unwrap();
    Run {
        image: f.linear.to_vec(),
        samples: p.samples,
        pixel_samples: p.pixel_samples,
        state: p.state,
    }
}

/// Errore quadratico medio relativo alla luminosità del riferimento (le zone scure contano
/// quanto le chiare, come per l'occhio).
fn rel_rmse(img: &[[f32; 3]], reference: &[[f32; 3]]) -> f64 {
    let n = img.len() as f64;
    (img.iter()
        .zip(reference)
        .map(|(a, r)| {
            let l = (r[0] + r[1] + r[2]) as f64 / 3.0 + 0.02;
            (0..3).map(|c| ((a[c] - r[c]) as f64 / l).powi(2)).sum::<f64>() / 3.0
        })
        .sum::<f64>()
        / n)
        .sqrt()
}

fn mean(img: &[[f32; 3]]) -> f64 {
    img.iter().map(|p| (p[0] + p[1] + p[2]) as f64).sum::<f64>() / (3.0 * img.len() as f64)
}

/// Scena di test con mezza inquadratura di cielo (zona facile) e mezza di oggetti e pavimento.
#[test]
fn adaptive_sampling_is_unbiased_and_cheaper() {
    let scene = Arc::new(RenderScene::compile(&sky_scene()));
    let reference = run(&scene, 2048, 0.0, 99).image;
    let uniform = run(&scene, 256, 0.0, 7);
    let adaptive = run(&scene, 2048, 0.04, 7);
    let px = (96 * 54) as f64;
    let (eu, ea) = (rel_rmse(&uniform.image, &reference), rel_rmse(&adaptive.image, &reference));
    let (su, sa) = (uniform.pixel_samples as f64 / px, adaptive.pixel_samples as f64 / px);
    eprintln!(
        "uniforme: {su:.0} campioni/pixel, errore {eu:.4} | adattivo (soglia 0.04): {sa:.1} campioni/pixel (max {}), errore {ea:.4}",
        adaptive.samples
    );
    // Non distorto: la media dell'immagine coincide con il riferimento.
    let (mr, ma) = (mean(&reference), mean(&adaptive.image));
    assert!((ma - mr).abs() < 0.01 * mr, "media {ma:.4} invece di {mr:.4}");
    // Fermata automatica molto prima del massimo.
    assert_eq!(adaptive.state, SessionState::Completed);
    assert!(adaptive.samples < 1024, "non si è fermato da solo: {}", adaptive.samples);
    // Efficienza: costo × errore² (campioni necessari per un dato errore) più basso
    // dell'uniforme di almeno il 20 % (misurato: circa −45 %).
    let (cost_u, cost_a) = (su * eu * eu, sa * ea * ea);
    assert!(cost_a < 0.8 * cost_u, "adattivo {cost_a:.4} contro uniforme {cost_u:.4}");
}

#[test]
fn without_threshold_every_pixel_gets_every_sample() {
    let scene = Arc::new(RenderScene::compile(&Scene::test_scene()));
    let r = run(&scene, 8, 0.0, 1);
    assert_eq!(r.samples, 8);
    assert_eq!(r.pixel_samples, 8 * 96 * 54);
}

/// Scena di test con la camera alzata: metà cielo (facile), metà oggetti e pavimento.
fn sky_scene() -> Scene {
    let mut s = Scene::test_scene();
    s.camera.position = forge_math::Vec3::new(0.3, 1.2, 4.2);
    s.camera.target = forge_math::Vec3::new(0.0, 1.0, 0.0);
    s
}
