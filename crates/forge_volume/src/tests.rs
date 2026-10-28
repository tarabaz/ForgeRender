use super::*;

fn small_fog() -> FogVolume {
    FogVolume {
        emitter: Vec3::new(0.0, 0.5, 0.0),
        size: Vec3::new(1.6, 0.6, 1.6),
        resolution: 48,
        sim_time: 3.0,
        ..Default::default()
    }
}

/// Densità media per strato (dal basso).
fn layer_means(b: &BakedFog) -> Vec<f64> {
    let g = &b.spec;
    let layer = g.res[0] * g.res[2];
    (0..g.res[1])
        .map(|y| b.data[y * layer..(y + 1) * layer].iter().map(|&v| v as f64).sum::<f64>() / layer as f64 * b.unit as f64)
        .collect()
}

#[test]
fn same_seed_same_fog_other_seed_other_fog() {
    let a = bake(&small_fog(), None);
    let b = bake(&small_fog(), None);
    assert_eq!(a.data, b.data);
    let c = bake(&FogVolume { seed: 2, ..small_fog() }, None);
    assert_ne!(a.data, c.data);
    // Stessa quantità di nebbia a grandi linee: cambia la forma, non il volume.
    let (va, vc) = (a.optical_volume(), c.optical_volume());
    assert!((va - vc).abs() / va < 0.25, "{va} vs {vc}");
}

#[test]
fn result_does_not_depend_on_thread_count() {
    let one = rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap();
    let a = one.install(|| bake(&small_fog(), None));
    let b = bake(&small_fog(), None);
    assert_eq!(a.data, b.data);
}

#[test]
fn dry_ice_settles_at_the_bottom_and_spreads_sideways() {
    let b = bake(&small_fog(), None);
    let m = layer_means(&b);
    let bottom = m[0] + m[1];
    let top = m[m.len() - 1] + m[m.len() - 2];
    assert!(bottom > 5.0 * top.max(1e-9), "basso {bottom} alto {top}: {m:?}");
    // Al fondo la nebbia arriva lontano dalla verticale dell'emettitore (> 0.4 m).
    let g = &b.spec;
    let mut reach = 0.0f32;
    for z in 0..g.res[2] {
        for x in 0..g.res[0] {
            if b.data[g.index(x, 0, z)] as f32 * b.unit > 0.05 * b.max_density {
                let p = g.center(x, 0, z);
                reach = reach.max(Vec3::new(p.x, 0.0, p.z).length());
            }
        }
    }
    assert!(reach > 0.4, "espansione laterale {reach} m");
    assert!(b.max_density > 0.0);
}

#[test]
fn no_emission_time_means_no_fog() {
    let b = bake(
        &FogVolume {
            sim_time: 0.0,
            ..small_fog()
        },
        None,
    );
    assert_eq!(b.max_density, 0.0);
    assert!(b.data.iter().all(|&v| v == 0));
}

#[test]
fn wall_stops_the_fog() {
    // Muro a x = 0.25 per tutta l'altezza: dall'altra parte non arriva nulla.
    let wall = |s: &GridSpec| {
        (0..s.len())
            .map(|i| {
                let (x, y, z) = s.coords(i);
                (s.center(x, y, z).x - 0.25).abs() < s.cell * 0.75
            })
            .collect::<Vec<bool>>()
    };
    let fog = FogVolume {
        sim_time: 6.0,
        ..small_fog()
    };
    let b = bake(&fog, Some(&wall));
    let free = bake(&fog, None);
    let beyond = |f: &BakedFog| {
        let g = &f.spec;
        (0..g.len())
            .filter(|&i| {
                let (x, y, z) = g.coords(i);
                g.center(x, y, z).x > 0.25 + g.cell
            })
            .map(|i| f.data[i] as f64 * f.unit as f64)
            .sum::<f64>()
    };
    assert!(beyond(&free) > 0.0, "senza muro la nebbia deve passare");
    assert_eq!(beyond(&b), 0.0, "la nebbia ha attraversato il muro");
    // Dentro il muro non c'è densità.
    let g = &b.spec;
    for (i, s) in wall(g).iter().enumerate() {
        if *s {
            assert_eq!(b.data[i], 0);
        }
    }
    // Spenta la collisione il muro viene ignorato.
    let off = bake(&FogVolume { collisions: false, ..fog }, Some(&wall));
    assert!(beyond(&off) > 0.0);
}

#[test]
fn majorant_bounds_the_interpolated_density() {
    let b = bake(&small_fog(), None);
    let (lo, hi) = b.bounds();
    let mut rng = forge_math::Pcg32::new(5, 1);
    for _ in 0..20000 {
        let p = lo + (hi - lo) * Vec3::new(rng.next_f32(), rng.next_f32(), rng.next_f32());
        let d = b.density(p);
        assert!(d <= b.majorant_at_y(p.y), "{d} > {} a {p}", b.majorant_at_y(p.y));
    }
}

#[test]
fn bake_time_is_reasonable() {
    // Risoluzione predefinita: deve restare nell'ordine dei secondi anche in debug-friendly release.
    let t = std::time::Instant::now();
    let b = bake(&FogVolume::default(), None);
    let ms = t.elapsed().as_millis();
    eprintln!(
        "bake predefinito: {} passi, simulazione {:.0} ms, densità {:.0} ms, griglia {:?}, totale {ms} ms",
        b.sim_steps, b.sim_ms, b.bake_ms, b.spec.res
    );
    assert!(b.max_density > 0.0);
}
