//! Backend CPU: path tracer progressivo multithread.
//!
//! Il lavoro di una passata (1 campione per pixel) è distribuito per righe
//! sul pool di thread Rayon dedicato del backend (work stealing, nessun lock
//! durante il calcolo: ogni riga scrive solo nella propria porzione del buffer).
//! Il generatore casuale è inizializzato per (seed, pixel, campione), quindi il
//! risultato è identico qualunque sia il numero di thread.

pub mod bsdf;
mod integrator;
pub mod lights;

pub use integrator::radiance;

use forge_math::Pcg32;
use forge_render_core::{RenderBackend, RenderScene, RenderSettings};
use rayon::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct CpuBackend {
    pool: rayon::ThreadPool,
    threads: usize,
}

impl CpuBackend {
    pub fn new(threads: usize) -> anyhow::Result<Self> {
        let threads = threads.max(1);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .thread_name(|i| format!("forge-cpu-{i}"))
            .build()?;
        Ok(Self { pool, threads })
    }

    pub fn from_settings(settings: &RenderSettings) -> anyhow::Result<Self> {
        Self::new(settings.effective_threads())
    }

    pub fn threads(&self) -> usize {
        self.threads
    }
}

/// Calcola un campione per il pixel `(x, y)`.
#[inline]
pub fn render_pixel(scene: &RenderScene, settings: &RenderSettings, x: u32, y: u32, sample: u32) -> [f32; 3] {
    let mut rng = Pcg32::for_pixel(settings.seed, x, y, sample);
    let (w, h) = (settings.width as f32, settings.height as f32);
    let (jx, jy) = rng.next_2d();
    let ndc_x = 2.0 * (x as f32 + jx) / w - 1.0;
    let ndc_y = 1.0 - 2.0 * (y as f32 + jy) / h;
    let dir = scene.camera.ray_dir(ndc_x, ndc_y, w / h);
    radiance(scene, settings, scene.camera.position, dir, &mut rng).to_array()
}

impl RenderBackend for CpuBackend {
    fn name(&self) -> String {
        "CPU path tracer".into()
    }

    fn device_info(&self) -> String {
        format!("{} thread", self.threads)
    }

    fn render_pass(
        &mut self,
        scene: &RenderScene,
        settings: &RenderSettings,
        sample_index: u32,
        out: &mut [[f32; 3]],
        cancel: &AtomicBool,
    ) -> anyhow::Result<bool> {
        let w = settings.width as usize;
        anyhow::ensure!(out.len() == w * settings.height as usize, "buffer di uscita di dimensione errata");
        self.pool.install(|| {
            out.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
                if cancel.load(Ordering::Relaxed) {
                    return;
                }
                for (x, px) in row.iter_mut().enumerate() {
                    *px = render_pixel(scene, settings, x as u32, y as u32, sample_index);
                }
            });
        });
        Ok(!cancel.load(Ordering::Relaxed))
    }
}
