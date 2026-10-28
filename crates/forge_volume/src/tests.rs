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
    let a = bake(&small_fog(), None, None);
    let b = bake(&small_fog(), None, None);
    assert_eq!(a.data, b.data);
    let c = bake(&FogVolume { seed: 2, ..small_fog() }, None, None);
    assert_ne!(a.data, c.data);
    // Stessa quantità di nebbia a grandi linee: cambia la forma, non il volume.
    let (va, vc) = (a.optical_volume(), c.optical_volume());
    assert!((va - vc).abs() / va < 0.25, "{va} vs {vc}");
}

#[test]
fn result_does_not_depend_on_thread_count() {
    let one = rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap();
    let a = one.install(|| bake(&small_fog(), None, None));
    let b = bake(&small_fog(), None, None);
    assert_eq!(a.data, b.data);
}

#[test]
fn dry_ice_settles_at_the_bottom_and_spreads_sideways() {
    let b = bake(&small_fog(), None, None);
    let m = layer_means(&b);
    let bottom = m[0] + m[1];
    let top = m[m.len() - 1] + m[m.len() - 2];
    assert!(bottom > 5.0 * top.max(1e-9), "basso {bottom} alto {top}: {m:?}");
    // Al fondo la nebbia arriva lontano dalla verticale dell'emettitore: dopo ~1 s di caduta
    // il tappeto avanza a 0.1–0.2 m/s (frenato dall'attrito col fondo), quindi > 0.3 m.
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
    assert!(reach > 0.3, "espansione laterale {reach} m");
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
    let b = bake(&fog, Some(&wall), None);
    let free = bake(&fog, None, None);
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
    let off = bake(&FogVolume { collisions: false, ..fog }, Some(&wall), None);
    assert!(beyond(&off) > 0.0);
}

#[test]
fn majorant_bounds_the_interpolated_density() {
    let b = bake(&small_fog(), None, None);
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
    let b = bake(&FogVolume::default(), None, None);
    let ms = t.elapsed().as_millis();
    eprintln!(
        "bake predefinito: {} passi, simulazione {:.0} ms, densità {:.0} ms, griglia {:?}, totale {ms} ms",
        b.sim_steps, b.sim_ms, b.bake_ms, b.spec.res
    );
    assert!(b.max_density > 0.0);
}

/// Tazza di 4.5 cm di raggio e 6 cm di altezza sul pavimento (y = 0), parete ~2 celle.
fn cup_solid(s: &GridSpec) -> Vec<bool> {
    (0..s.len())
        .map(|i| {
            let (x, y, z) = s.coords(i);
            let p = s.center(x, y, z);
            let r = Vec3::new(p.x, 0.0, p.z).length();
            p.y < 0.0 || (p.y < 0.06 && (r - 0.0435).abs() < s.cell * 0.6 + 0.0015)
        })
        .collect()
}

fn cup_fog() -> FogVolume {
    FogVolume {
        emitter: Vec3::new(0.0, 0.05, 0.0),
        size: Vec3::new(0.5, 0.3, 0.5),
        source_radius: 0.03,
        emission_rate: 0.3,
        sim_time: 5.0,
        resolution: 48,
        turbulence: 0.3,
        ..Default::default()
    }
}

/// La nebbia riempie la tazza, trabocca, cola lungo la parete e forma il tappeto sul pavimento.
#[test]
fn fog_fills_the_cup_and_cascades_to_the_floor() {
    let b = bake(&cup_fog(), Some(&cup_solid), None);
    let g = &b.spec;
    let (mut inside_low, mut floor_out, mut n_in, mut n_out) = (0.0f64, 0.0f64, 0, 0);
    for i in 0..g.len() {
        let (x, y, z) = g.coords(i);
        let p = g.center(x, y, z);
        let r = Vec3::new(p.x, 0.0, p.z).length();
        let d = b.data[i] as f64 * b.unit as f64;
        if r < 0.035 && p.y > 0.0 && p.y < 0.03 {
            inside_low += d;
            n_in += 1;
        }
        if r > 0.07 && r < 0.2 && p.y > 0.0 && p.y < 0.03 {
            floor_out += d;
            n_out += 1;
        }
    }
    let (inside_low, floor_out) = (inside_low / n_in as f64, floor_out / n_out as f64);
    // Densità piena ≈ `density` (60); il massimo locale arriva a ~2×.
    assert!(inside_low > 0.1 * b.max_density as f64, "tazza non piena: {inside_low}");
    assert!(
        floor_out > 0.02 * b.max_density as f64,
        "niente tappeto fuori dalla tazza: {floor_out}"
    );
}

/// Con più "risalita" una parte maggiore della nebbia sta sopra l'emettitore (sbuffi).
#[test]
fn rise_lifts_part_of_the_fog() {
    let above = |rise: f32| {
        let b = bake(&FogVolume { rise, ..cup_fog() }, Some(&cup_solid), None);
        let g = &b.spec;
        let (mut up, mut all) = (0.0f64, 0.0f64);
        for i in 0..g.len() {
            let (x, y, z) = g.coords(i);
            let d = b.data[i] as f64;
            all += d;
            if g.center(x, y, z).y > 0.12 {
                up += d;
            }
        }
        up / all.max(1e-9)
    };
    let (low, high) = (above(0.0), above(1.0));
    eprintln!("sopra 12 cm: risalita 0 {low:.4}, risalita 1 {high:.4}");
    assert!(high > 3.0 * low && high > 0.02, "risalita 0: {low:.4}, risalita 1: {high:.4}");
}

/// Densità, colore, anisotropia e scala del dettaglio non rifanno la simulazione;
/// i parametri del moto sì. Il dettaglio rifatto da una simulazione in cache è identico
/// a quello calcolato da zero.
#[test]
fn appearance_changes_reuse_the_simulation() {
    let base = small_fog();
    let looks = FogVolume {
        density: 200.0,
        albedo: [0.3, 0.4, 0.9],
        anisotropy: -0.4,
        scale: 0.6,
        name: "altro".into(),
        ..base.clone()
    };
    assert_eq!(sim_key(&base), sim_key(&looks));
    for moved in [
        FogVolume { seed: 9, ..base.clone() },
        FogVolume {
            sim_time: 2.0,
            ..base.clone()
        },
        FogVolume { rise: 0.9, ..base.clone() },
        FogVolume {
            turbulence: 0.1,
            ..base.clone()
        },
    ] {
        assert_ne!(sim_key(&base), sim_key(&moved));
    }
    let s = simulate_fog(&base, None, None);
    let a = bake_detail(&looks, &s, None, None);
    let b = bake(&looks, None, None);
    assert_eq!(a.data, b.data);
    assert_eq!(a.albedo, b.albedo);
}

/// La qualità della griglia decide le celle della simulazione; lo spostamento della zona
/// sposta il riquadro ma non l'emettitore, e rifà la simulazione.
#[test]
fn grid_quality_and_zone_offset() {
    let base = FogVolume {
        sim_time: 0.5,
        ..small_fog()
    };
    let coarse = simulate_fog(
        &FogVolume {
            sim_cells: 32,
            ..base.clone()
        },
        None,
        None,
    );
    let fine = simulate_fog(
        &FogVolume {
            sim_cells: 64,
            resolution: 120,
            ..base.clone()
        },
        None,
        None,
    );
    assert_eq!(*coarse.spec.res.iter().max().unwrap(), 32);
    assert_eq!(*fine.spec.res.iter().max().unwrap(), 64);
    let moved = FogVolume {
        zone_offset: Vec3::new(0.0, 0.2, 0.0),
        ..base.clone()
    };
    let (a, b) = (base.bounds(), moved.bounds());
    assert!((b.0.y - a.0.y - 0.2).abs() < 1e-6 && (b.1.y - a.1.y - 0.2).abs() < 1e-6);
    assert_eq!(moved.emitter, base.emitter);
    assert_ne!(sim_key(&base), sim_key(&moved));
}
