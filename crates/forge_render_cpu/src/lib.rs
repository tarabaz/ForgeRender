//! Backend CPU: path tracer progressivo multithread.
//!
//! Il lavoro di una passata (1 campione per pixel) è distribuito per righe
//! sul pool di thread Rayon dedicato del backend (work stealing, nessun lock
//! durante il calcolo: ogni riga scrive solo nella propria porzione del buffer).
//! Il generatore casuale è inizializzato per (seed, pixel, campione), quindi il
//! risultato è identico qualunque sia il numero di thread.

pub mod bsdf;
mod guides;
mod integrator;
pub mod lights;
pub mod medium;
mod textures;

pub use guides::{render_guides, GUIDE_SAMPLES};
pub use integrator::radiance;

use forge_math::{Pcg32, Vec2, Vec3};
use forge_render_core::{RenderBackend, RenderScene, RenderSettings};
use forge_scene::Camera;
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

/// Punto dell'apertura nel disco unitario: cerchio (0 lamelle) o poligono regolare
/// con `blades` lati (bokeh poligonale), campionato uniformemente.
#[inline]
pub fn sample_aperture(blades: u32, rotation_rad: f32, u0: f32, u1: f32, u2: f32) -> Vec2 {
    if blades < 3 {
        return forge_math::sampling::concentric_disk(u1, u2);
    }
    let n = blades as f32;
    let k = (u0 * n).floor().min(n - 1.0);
    let step = std::f32::consts::TAU / n;
    let (a0, a1) = (rotation_rad + k * step, rotation_rad + (k + 1.0) * step);
    let (v0, v1) = (Vec2::new(a0.cos(), a0.sin()), Vec2::new(a1.cos(), a1.sin()));
    // Punto uniforme nel triangolo (centro, v0, v1).
    let su = u1.sqrt();
    v0 * (su * (1.0 - u2)) + v1 * (su * u2)
}

/// Raggio camera per il pixel `(x, y)`: jitter nel pixel, decentramento (shift) e,
/// con la profondità di campo, lente sottile messa a fuoco sul piano a `focus_distance`.
/// Le stesse formule sono nello shader GPU (`main` in `path_tracer.wgsl`).
#[inline]
pub fn camera_ray(cam: &Camera, width: u32, height: u32, x: u32, y: u32, rng: &mut Pcg32) -> (Vec3, Vec3) {
    let (w, h) = (width as f32, height as f32);
    let (jx, jy) = rng.next_2d();
    let ndc_x = 2.0 * (x as f32 + jx) / w - 1.0 + 2.0 * cam.lens.shift_x;
    let ndc_y = 1.0 - 2.0 * (y as f32 + jy) / h + 2.0 * cam.lens.shift_y;
    let dir = cam.ray_dir(ndc_x, ndc_y, w / h);
    let lr = cam.lens_radius();
    if lr <= 0.0 {
        return (cam.position, dir);
    }
    let (r, u, f) = cam.basis();
    let focus = cam.position + dir * (cam.lens.focus_distance.max(1e-4) / dir.dot(f).max(1e-6));
    let (u0, u1, u2) = (rng.next_f32(), rng.next_f32(), rng.next_f32());
    let a = sample_aperture(cam.lens.blades, cam.lens.blade_rotation_deg.to_radians(), u0, u1, u2) * lr;
    let o = cam.position + r * a.x + u * a.y;
    (o, (focus - o).normalize())
}

/// Calcola un campione per il pixel `(x, y)`.
#[inline]
pub fn render_pixel(scene: &RenderScene, settings: &RenderSettings, x: u32, y: u32, sample: u32) -> [f32; 3] {
    let mut rng = Pcg32::for_pixel(settings.seed, x, y, sample);
    let (o, dir) = camera_ray(&scene.camera, settings.width, settings.height, x, y, &mut rng);
    radiance(scene, settings, o, dir, &mut rng).to_array()
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
        _max_samples: u32,
        out: &mut [[f32; 3]],
        cancel: &AtomicBool,
    ) -> anyhow::Result<u32> {
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
        // Il backend CPU calcola un campione per passata: passate brevi, interruzioni immediate.
        Ok(if cancel.load(Ordering::Relaxed) { 0 } else { 1 })
    }
}
