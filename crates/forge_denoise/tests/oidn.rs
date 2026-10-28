//! Test con la libreria OIDN reale. Senza OIDN i test vengono saltati, a meno
//! che `FORGE_REQUIRE_OIDN=1` (impostato in CI) non li renda obbligatori.

use forge_denoise::OidnDenoiser;
use forge_render_core::{DenoiseQuality, Denoiser, RenderBackend, RenderScene, RenderSettings};
use forge_render_cpu::CpuBackend;
use forge_scene::Scene;
use std::sync::atomic::AtomicBool;

fn denoiser() -> Option<OidnDenoiser> {
    match OidnDenoiser::load() {
        Ok(d) => Some(d),
        Err(e) => {
            assert!(
                std::env::var_os("FORGE_REQUIRE_OIDN").is_none(),
                "OIDN richiesto ma non caricato: {e:#}"
            );
            eprintln!("OIDN non disponibile, test saltato: {e:#}");
            None
        }
    }
}

fn render(scene: &RenderScene, settings: &RenderSettings, spp: u32) -> Vec<[f32; 3]> {
    let mut be = CpuBackend::new(std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)).unwrap();
    let n = (settings.width * settings.height) as usize;
    let (mut sum, mut pass) = (vec![[0.0f32; 3]; n], vec![[0.0f32; 3]; n]);
    let cancel = AtomicBool::new(false);
    for s in 0..spp {
        be.render_pass(scene, settings, s, 1, &mut pass, &cancel).unwrap();
        for (a, p) in sum.iter_mut().zip(&pass) {
            for c in 0..3 {
                a[c] += p[c];
            }
        }
    }
    sum.iter().map(|p| p.map(|v| v / spp as f32)).collect()
}

/// Errore quadratico medio dopo una compressione semplice (x/(1+x)), così i
/// pixel molto luminosi non dominano la misura.
fn mse(a: &[[f32; 3]], b: &[[f32; 3]]) -> f64 {
    let t = |v: f32| (v / (1.0 + v)) as f64;
    a.iter()
        .zip(b)
        .map(|(x, y)| (0..3).map(|c| (t(x[c]) - t(y[c])).powi(2)).sum::<f64>())
        .sum::<f64>()
        / (a.len() * 3) as f64
}

#[test]
fn denoised_image_is_closer_to_reference() {
    let Some(mut d) = denoiser() else { return };
    let scene = RenderScene::compile(&Scene::test_scene());
    let settings = RenderSettings {
        width: 160,
        height: 96,
        ..Default::default()
    };
    let reference = render(&scene, &settings, 512);
    let noisy = render(&scene, &settings, 8);
    let guides = d.guides(&scene, &settings, &AtomicBool::new(false)).unwrap();
    let with_guides = d.denoise(&noisy, Some(&guides), 160, 96, DenoiseQuality::High, false).unwrap();
    let no_guides = d.denoise(&noisy, None, 160, 96, DenoiseQuality::High, false).unwrap();
    let (e_noisy, e_g, e_ng) = (mse(&noisy, &reference), mse(&with_guides, &reference), mse(&no_guides, &reference));
    eprintln!(
        "MSE rumorosa {e_noisy:.3e} | ripulita con guide {e_g:.3e} | senza guide {e_ng:.3e} ({})",
        d.name()
    );
    assert!(
        e_g < 0.5 * e_noisy,
        "il denoiser deve almeno dimezzare l'errore: {e_g} vs {e_noisy}"
    );
    assert!(e_ng < e_noisy);
    assert!(with_guides.iter().all(|p| p.iter().all(|v| v.is_finite() && *v >= 0.0)));
}

#[test]
fn all_qualities_and_cuda_fallback_work() {
    let Some(mut d) = denoiser() else { return };
    let img: Vec<[f32; 3]> = (0..64 * 32).map(|i| [((i * 7919) % 101) as f32 / 100.0; 3]).collect();
    for q in [DenoiseQuality::Fast, DenoiseQuality::Balanced, DenoiseQuality::High] {
        let out = d.denoise(&img, None, 64, 32, q, false).unwrap();
        assert_eq!(out.len(), img.len());
    }
    // Con CUDA richiesto ma non disponibile (es. nessuna GPU NVIDIA) si ripiega sulla CPU.
    let out = d.denoise(&img, None, 64, 32, DenoiseQuality::Fast, true).unwrap();
    assert_eq!(out.len(), img.len());
    eprintln!("dispositivo con CUDA richiesto: {} (errore CUDA: {:?})", d.name(), d.cuda_error());
}
