//! Contorni a inchiostro e profondità del tilt-shift calcolati a parte: il render parte senza
//! aspettarli e i livelli arrivano poco dopo, anche a render finito; una serie di avvii di
//! seguito tiene solo l'ultimo.

use forge_render_core::{OutlineSettings, RenderScene, RenderSession, RenderSettings};
use forge_render_cpu::CpuBackend;
use forge_scene::Scene;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn settings(spp: u32) -> RenderSettings {
    RenderSettings {
        width: 160,
        height: 90,
        target_samples: spp,
        outlines: OutlineSettings {
            enabled: true,
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Aspetta un fotogramma del render `id` con i contorni.
fn wait_outline(s: &RenderSession, id: u64) -> bool {
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(60) {
        if let Some(f) = s.latest_frame() {
            if f.job_id == id && f.outline.is_some() {
                return true;
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    false
}

#[test]
fn outlines_arrive_after_the_render_starts() {
    let scene = Arc::new(RenderScene::compile(&Scene::test_scene()));
    let s = RenderSession::new();
    let id = s.start(scene.clone(), settings(2), Box::new(CpuBackend::new(2).unwrap()));
    // Anche a render già finito (2 campioni) i contorni arrivano e si pubblicano.
    s.wait(id, Duration::from_secs(120));
    assert!(wait_outline(&s, id), "contorni mai arrivati");

    // Molti avvii di seguito (si trascina la camera): vale l'ultimo, e anche lui ha i contorni.
    let mut last = 0;
    for _ in 0..12 {
        last = s.start(
            Arc::new(scene.with_camera(scene.camera.clone())),
            settings(4),
            Box::new(CpuBackend::new(2).unwrap()),
        );
    }
    let p = s.wait(last, Duration::from_secs(120));
    assert_eq!(p.job_id, last);
    assert!(wait_outline(&s, last), "contorni dell'ultimo avvio mai arrivati");
}
