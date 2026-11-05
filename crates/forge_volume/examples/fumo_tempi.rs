//! Tempi della simulazione del fumo su riquadri tipici (fumo caldo, vapore, gas pesante).
//!   cargo run --release -p forge_volume --example fumo_tempi
use forge_math::Vec3;
use forge_volume::smoke_sim::{simulate_smoke, SmokeSimParams};
use forge_volume::GridSpec;

fn main() {
    let none = || false;
    let cases = [
        (
            "fumo caldo 0,8×1,4×0,8, 100 celle",
            Vec3::new(0.8, 1.4, 0.8),
            100,
            Vec3::new(0.0, 0.06, 0.0),
            0.08,
            1.0,
            4.0,
            0.3,
            3.0,
            4.0,
        ),
        (
            "gas pesante 1,6×0,5×1,6, 100 celle",
            Vec3::new(1.6, 0.5, 1.6),
            100,
            Vec3::new(0.0, 0.3, 0.0),
            0.07,
            0.0,
            4.0,
            3.0,
            4.0,
            1.5,
        ),
        (
            "riquadro grande 3×2×3, 136 celle",
            Vec3::new(3.0, 2.0, 3.0),
            136,
            Vec3::new(0.0, 0.06, 0.0),
            0.08,
            1.0,
            4.0,
            0.3,
            3.0,
            4.0,
        ),
    ];
    for (name, size, cells, off, r, temp, buoy, weight, time, vort) in cases {
        let lo = Vec3::new(-size.x * 0.5, 0.0, -size.z * 0.5);
        let hi = lo + size;
        let spec = GridSpec::covering(lo, hi, cells);
        let p = SmokeSimParams {
            spec,
            source: Vec3::new(0.0, 0.0, 0.0) + off,
            source_radius: r,
            emission: 3.0,
            temperature: temp,
            buoyancy: buoy,
            weight,
            exit_speed: if temp > 0.0 { 0.3 } else { -0.1 },
            cooling: 0.6,
            dissipation: 0.1,
            vorticity: vort,
            turbulence: 3.0,
            turbulence_scale: 0.15,
            wind: Vec3::ZERO,
            time,
            seed: 1,
            edge_fade: 0.2,
            box_center: (lo + hi) * 0.5,
            box_yaw: 0.0,
            box_half: size * 0.5,
            particles: false,
            patchiness: 0.0,
        };
        let f = simulate_smoke(&p, None, &|_| {}, &none).unwrap();
        let mass: f32 = f.density.iter().sum();
        println!("{name}: {:?} celle, {} passi, {:.0} ms, massa {mass:.1}", f.spec.res, f.steps, f.ms);
    }
}
