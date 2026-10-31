//! Benchmark della proiezione Box (revisione 0.15.0): cinque scene con le stesse impostazioni,
//! su GPU e CPU, ognuna con il materiale in proiezione Box (fino a tre letture sfumate per
//! mappa) e in proiezione UV (una lettura): la differenza isola il costo della sfumatura.
//!
//! `cargo run -p forge_render_gpu --release --example bench_box -- [--gpu-spp N] [--cpu-spp N] [--csv FILE]`

use forge_math::{Mat4, Vec3};
use forge_render_core::{Accumulator, RenderBackend, RenderScene, RenderSettings};
use forge_render_cpu::CpuBackend;
use forge_render_gpu::GpuBackend;
use forge_scene::{primitives, Camera, ComponentId, Environment, Light, Material, MaterialId, Projection, Scene, TextureMapping};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

const W: u32 = 960;
const H: u32 = 540;

fn arg(name: &str, default: u32) -> u32 {
    let a: Vec<String> = std::env::args().collect();
    a.iter()
        .position(|x| x == name)
        .and_then(|i| a.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Texture procedurale 1024 × 1024 (onde e rumore), salvata nella cartella temporanea.
fn texture(name: &str, seed: u32, rgb: bool) -> PathBuf {
    let dir = std::env::temp_dir().join("forge_bench_box");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    if path.is_file() {
        return path;
    }
    let n = 1024;
    let img = image::RgbImage::from_fn(n, n, |x, y| {
        let h = |k: u32| {
            let v = (x * 7 + y * 13 + seed * 31 + k * 101).wrapping_mul(2_654_435_761) >> 24;
            let wave = ((x as f32 * 0.05 + seed as f32).sin() * (y as f32 * 0.03).cos() * 0.5 + 0.5) * 200.0;
            (wave + (v % 56) as f32) as u8
        };
        if rgb {
            image::Rgb([h(0), h(1), h(2)])
        } else {
            let v = h(0);
            image::Rgb([v, v, v])
        }
    });
    img.save(&path).unwrap();
    path
}

#[derive(Clone, Copy)]
enum Maps {
    ColorRough,
    NormalBump,
    Displacement,
    All,
}

fn material(maps: Maps, projection: Projection) -> Material {
    let mut m = Material {
        id: MaterialId::new("bench"),
        base_color: [0.9; 3],
        roughness: 0.7,
        base_color_texture: Some(texture("col.png", 1, true)),
        roughness_texture: Some(texture("rough.png", 2, false)),
        mapping: TextureMapping {
            projection,
            scale: 0.2,
            ..Default::default()
        },
        ..Default::default()
    };
    match maps {
        Maps::ColorRough => {}
        Maps::NormalBump | Maps::All => {
            m.normal_texture = Some(texture("nor.png", 3, true));
            m.height_texture = Some(texture("h.png", 4, false));
            m.bump_mm = 1.0;
        }
        Maps::Displacement => {
            m.height_texture = Some(texture("h.png", 4, false));
            m.displacement_mm = 4.0;
            m.displacement_mid = 0.5;
        }
    }
    m
}

fn base_scene() -> Scene {
    let mut s = Scene {
        environment: Environment::Uniform {
            color: [0.75, 0.8, 0.85],
            strength: 1.0,
        },
        ..Default::default()
    };
    s.lights.push(Light::point("P", Vec3::new(1.5, 3.0, 2.0), 60.0));
    s.camera = Camera {
        position: Vec3::new(0.0, 1.3, 2.6),
        target: Vec3::new(0.0, 0.35, 0.0),
        up: Vec3::Y,
        vfov_deg: 40.0,
        ..Default::default()
    };
    s
}

fn place(s: &mut Scene, name: &str, mesh: usize, xf: Mat4) {
    let cid = ComponentId(name.into());
    s.add_node(None, cid.clone(), name, xf, Some(mesh));
    s.assignments.insert(cid, MaterialId::new("bench"));
}

/// Scena `k` (0–4) con il materiale dato.
fn scene(k: usize, m: Material) -> (&'static str, Scene) {
    let mut s = base_scene();
    s.materials.upsert(m);
    let floor = s.add_mesh(primitives::plane(6.0, 6.0));
    place(&mut s, "pavimento", floor, Mat4::IDENTITY);
    let rot = Mat4::from_rotation_y(0.5) * Mat4::from_rotation_x(0.3);
    let name = match k {
        0..=2 => {
            let sphere = s.add_mesh(primitives::uv_sphere(0.4, 192, 96));
            place(&mut s, "sfera", sphere, Mat4::from_translation(Vec3::new(0.0, 0.4, 0.0)) * rot);
            ["colore+rugosità (sfera)", "normal+bump (sfera)", "displacement (sfera)"][k]
        }
        3 => {
            // "CAD": 40 × 40 pezzi (cilindri e blocchi ruotati), circa 650 mila triangoli.
            let cyl = s.add_mesh(primitives::cylinder(0.02, 0.08, 96));
            let blk = s.add_mesh(primitives::cuboid(Vec3::new(0.035, 0.05, 0.02)));
            for i in 0..40 {
                for j in 0..40 {
                    let p = Vec3::new(-1.0 + i as f32 * 0.05, 0.0, -1.2 + j as f32 * 0.05);
                    let r = Mat4::from_rotation_y((i * 7 + j * 3) as f32 * 0.1);
                    if (i + j) % 3 == 0 {
                        place(&mut s, &format!("b{i}-{j}"), blk, Mat4::from_translation(p + Vec3::Y * 0.025) * r);
                    } else {
                        place(&mut s, &format!("c{i}-{j}"), cyl, Mat4::from_translation(p) * r);
                    }
                }
            }
            "CAD complessa (1600 pezzi)"
        }
        _ => {
            let torus = s.add_mesh(primitives::torus(0.35, 0.13, 384, 192));
            place(&mut s, "toro", torus, Mat4::from_translation(Vec3::new(0.0, 0.3, 0.0)) * rot);
            "superficie curva (toro)"
        }
    };
    s.refresh_pivots();
    (name, s)
}

struct Run {
    secs: f64,
    spp: u32,
}

fn run(be: &mut dyn RenderBackend, rs: &RenderScene, spp: u32) -> Run {
    let st = RenderSettings {
        width: W,
        height: H,
        ..Default::default()
    };
    let mut acc = Accumulator::new(W, H);
    let mut pass = vec![[0.0; 3]; acc.pixel_count()];
    let cancel = AtomicBool::new(false);
    // Riscaldamento (caricamento della scena, compilazione): escluso dalla misura.
    be.render_pass(rs, &st, 0, 1, &mut pass, &cancel).unwrap();
    let t = Instant::now();
    let mut done = 0;
    while done < spp {
        let n = be.render_pass(rs, &st, 1 + done, spp - done, &mut pass, &cancel).unwrap();
        acc.add_samples(&pass, n);
        done += n;
    }
    Run {
        secs: t.elapsed().as_secs_f64(),
        spp,
    }
}

fn msps(r: &Run) -> f64 {
    (W * H) as f64 * r.spp as f64 / r.secs / 1e6
}

fn main() {
    let gpu_spp = arg("--gpu-spp", 256);
    let cpu_spp = arg("--cpu-spp", 8);
    let mut gpu = GpuBackend::new().ok();
    let mut cpu = CpuBackend::new(0).unwrap();
    if let Some(g) = &gpu {
        println!("GPU: {}", g.device_info());
    }
    println!("Risoluzione {W}x{H}; GPU {gpu_spp} campioni, CPU {cpu_spp} campioni (tutti i thread)\n");
    println!(
        "{:<28} {:>4} {:>9} {:>10} {:>10} {:>10} {:>10} {:>9}",
        "scena", "proj", "triangoli", "compila ms", "GPU ms/spp", "GPU Ms/s", "CPU ms/spp", "VRAM MB"
    );
    let mut csv =
        String::from("scena,proiezione,triangoli,compila_ms,gpu_ms_per_spp,gpu_msamples_s,cpu_ms_per_spp,cpu_msamples_s,vram_scena_mb\n");
    for k in 0..5 {
        let maps = [Maps::ColorRough, Maps::NormalBump, Maps::Displacement, Maps::All, Maps::All][k];
        for proj in [Projection::Box, Projection::Uv] {
            let (name, s) = scene(k, material(maps, proj));
            let t = Instant::now();
            let rs = RenderScene::compile(&s);
            let compile_ms = t.elapsed().as_secs_f64() * 1e3;
            let (g_ms, g_ms_s, vram) = match gpu.as_mut() {
                Some(g) => {
                    let r = run(g, &rs, gpu_spp);
                    (r.secs * 1e3 / r.spp as f64, msps(&r), g.scene_bytes() as f64 / 1e6)
                }
                None => (f64::NAN, f64::NAN, f64::NAN),
            };
            let c = run(&mut cpu, &rs, cpu_spp);
            let pname = if proj == Projection::Box { "box" } else { "uv" };
            println!(
                "{:<28} {:>4} {:>9} {:>10.0} {:>10.2} {:>10.1} {:>10.1} {:>9.1}",
                name,
                pname,
                rs.triangle_count(),
                compile_ms,
                g_ms,
                g_ms_s,
                c.secs * 1e3 / c.spp as f64,
                vram
            );
            csv += &format!(
                "{name},{pname},{},{compile_ms:.1},{g_ms:.3},{g_ms_s:.2},{:.2},{:.2},{vram:.2}\n",
                rs.triangle_count(),
                c.secs * 1e3 / c.spp as f64,
                msps(&c)
            );
        }
    }
    let a: Vec<String> = std::env::args().collect();
    if let Some(p) = a.iter().position(|x| x == "--csv").and_then(|i| a.get(i + 1)) {
        std::fs::write(p, csv).unwrap();
    }
}
