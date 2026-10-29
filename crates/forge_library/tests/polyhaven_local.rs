//! Import della cartella dei download Poly Haven (formato dello script
//! `download_polyhaven_materials.py`). File di test separato: è un processo a sé, quindi
//! può impostare `FORGE_LIBRARY_DIR` senza disturbare i test di `store`.

use forge_library::online::{AssetKind, OnlineAsset, Provider};
use forge_library::polyhaven_local::{self, Options, KEPT_MANIFEST, MANIFEST};
use forge_library::store::{self, Transfer};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn temp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("forge_phlocal_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn exr(p: &Path, v: [f32; 3]) {
    image::Rgb32FImage::from_pixel(4, 4, image::Rgb(v)).save(p).unwrap();
}

/// Cartella `<id>` come la scrive lo script: `maps/` con i file originali e `material.json`.
/// `missing`: mappe elencate nel manifest ma senza file.
fn fake_material(root: &Path, id: &str, name: &str, maps: &[(&str, &str)], missing: &[(&str, &str)]) -> PathBuf {
    let dir = root.join(id);
    let m = dir.join("maps");
    std::fs::create_dir_all(&m).unwrap();
    let mut entries = Vec::new();
    for (map, file) in maps.iter().chain(missing) {
        let path = m.join(file);
        if !missing.iter().any(|(_, f)| f == file) {
            match path.extension().and_then(|e| e.to_str()).unwrap() {
                "jpg" => image::RgbImage::from_pixel(4, 4, image::Rgb([150, 120, 90])).save(&path).unwrap(),
                "exr" if map.contains("nor") => exr(&path, [0.5, 0.5, 1.0]),
                "exr" if *map == "arm" => exr(&path, [1.0, 0.6, 0.0]),
                "exr" => exr(&path, [0.4, 0.4, 0.4]),
                "mtlx" => std::fs::write(&path, "<materialx/>").unwrap(),
                _ => std::fs::write(&path, "dati").unwrap(),
            }
        }
        entries.push(serde_json::json!({
            "map": map, "resolution": "2k", "format": file.rsplit('.').next().unwrap(),
            "file": format!("maps\\{file}"), "url": format!("https://dl.polyhaven.org/{file}"),
            "status": "downloaded", "bytes": 1
        }));
    }
    let manifest = serde_json::json!({
        "id": id, "name": name, "source": format!("https://polyhaven.com/a/{id}"), "license": "CC0",
        "requested_resolution": "2k", "maps": entries
    });
    std::fs::write(dir.join(MANIFEST), serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
    dir
}

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

#[test]
fn import_polyhaven_download_folder() {
    let lib = temp("lib");
    // SAFETY: unico test di questo processo.
    unsafe { std::env::set_var("FORGE_LIBRARY_DIR", &lib) };
    let dl = temp("dl");

    // Download in corso: cartella senza manifest, non va toccata.
    let busy = dl.join("aerial_rocks_02");
    std::fs::create_dir_all(busy.join("maps")).unwrap();
    std::fs::write(busy.join("maps/aerial_rocks_02_diff_2k.jpg"), "a metà").unwrap();

    let bark = fake_material(
        &dl,
        "bark_brown_01",
        "Bark Brown 01",
        &[
            ("blend", "bark_brown_01_2k.blend"),
            ("AO", "bark_brown_01_ao_2k.exr"),
            ("arm", "bark_brown_01_arm_2k.exr"),
            ("Bump", "bark_brown_01_bump_2k.exr"),
            ("Diffuse", "bark_brown_01_diff_2k.jpg"),
            ("Displacement", "bark_brown_01_disp_2k.exr"),
            ("nor_gl", "bark_brown_01_nor_gl_2k.exr"),
            ("Rough", "bark_brown_01_rough_2k.exr"),
            ("spec", "bark_brown_01_spec_2k.exr"),
            ("gltf", "bark_brown_01_2k.gltf"),
            ("mtlx", "bark_brown_01_2k.mtlx"),
        ],
        &[("nor_dx", "bark_brown_01_nor_dx_2k.exr")],
    );
    let plate = fake_material(
        &dl,
        "metal_plate_02",
        "Metal Plate 02",
        &[
            ("blend", "metal_plate_02_2k.blend"),
            ("AO", "metal_plate_02_ao_2k.exr"),
            ("arm", "metal_plate_02_arm_2k.exr"),
            ("Diffuse", "metal_plate_02_diff_2k.jpg"),
            ("Displacement", "metal_plate_02_disp_2k.exr"),
            ("Metal", "metal_plate_02_metal_2k.exr"),
            ("nor_dx", "metal_plate_02_nor_dx_2k.exr"),
            ("nor_gl", "metal_plate_02_nor_gl_2k.exr"),
            ("Rough", "metal_plate_02_rough_2k.exr"),
            ("gltf", "metal_plate_02_2k.gltf"),
            ("mtlx", "metal_plate_02_2k.mtlx"),
        ],
        &[],
    );
    // Voce del catalogo (metal_plate_02: 2000 mm, categorie e tag come sul sito);
    // bark_brown_01 manca apposta.
    let plate_info = OnlineAsset {
        provider: Provider::PolyHaven,
        kind: AssetKind::Material,
        id: "metal_plate_02".into(),
        name: "Metal Plate 02".into(),
        category: "metal".into(),
        categories: vec!["metal".into(), "man made".into(), "floor".into()],
        tags: vec!["rusty".into(), "steel".into()],
        thumbnail_url: String::new(),
        size_m: Some(2.0),
    };
    let catalog: HashMap<String, OnlineAsset> = [("metal_plate_02".to_string(), plate_info)].into();
    let mut log = Vec::new();

    // Solo elenco: niente cambia.
    let dry = polyhaven_local::import_all(
        &dl,
        &catalog,
        Options {
            dry_run: true,
            ..Default::default()
        },
        &mut |l| log.push(l.to_string()),
    )
    .unwrap();
    assert_eq!(dry.pending, ["bark_brown_01", "metal_plate_02"]);
    assert_eq!(dry.incomplete, std::slice::from_ref(&busy));
    assert!(dry.imported.is_empty() && store::list().is_empty());

    // Copia, al massimo 1: si importa solo bark_brown_01 e l'origine resta com'era.
    let before = names(&bark.join("maps"));
    let one = polyhaven_local::import_all(
        &dl,
        &catalog,
        Options {
            max: Some(1),
            ..Default::default()
        },
        &mut |l| log.push(l.to_string()),
    )
    .unwrap();
    assert_eq!(one.imported.len(), 1, "{log:?}");
    assert_eq!(one.pending, ["metal_plate_02"]);
    assert_eq!(names(&bark.join("maps")), before);
    assert!(bark.join(MANIFEST).is_file());
    let b = store::load(&one.imported[0].1).unwrap();
    assert_eq!(b.material.name, "Bark Brown 01 2K");
    assert_eq!(b.source, "Poly Haven bark_brown_01 (2K)");
    assert_eq!(b.material.mapping.scale, 1.0);
    // Solo le mappe collegate agli slot (disp, non bump), il MaterialX e il manifest.
    assert_eq!(
        names(&b.dir),
        [
            "bark_brown_01_2k.mtlx",
            "bark_brown_01_ao_2k.exr",
            "bark_brown_01_diff_2k.jpg",
            "bark_brown_01_disp_2k.exr",
            "bark_brown_01_nor_gl_2k.exr",
            "bark_brown_01_rough_2k.exr",
            "materiale.json",
            KEPT_MANIFEST,
        ]
    );
    assert!(b.material.height_texture.as_ref().unwrap().ends_with("bark_brown_01_disp_2k.exr"));
    assert!(b.misassigned.is_empty());
    assert!(b.notes.iter().any(|n| n.contains("non trovato nel catalogo")), "{:?}", b.notes);
    assert!(
        b.notes.iter().any(|n| n.contains("\"nor_dx\"") && n.contains("assente")),
        "{:?}",
        b.notes
    );

    // Spostamento di tutto il resto: bark_brown_01 è già in libreria e resta nei download.
    let all = polyhaven_local::import_all(
        &dl,
        &catalog,
        Options {
            transfer: Transfer::Move,
            ..Default::default()
        },
        &mut |l| log.push(l.to_string()),
    )
    .unwrap();
    assert_eq!(all.already_present, ["bark_brown_01"]);
    assert_eq!(all.imported.len(), 1, "{log:?}");
    assert!(all.failed.is_empty());
    let p = store::load(&all.imported[0].1).unwrap();
    assert_eq!(p.source, "Poly Haven metal_plate_02 (2K)");
    assert_eq!(p.material.mapping.scale, 2.0);
    assert!(p.notes.is_empty(), "{:?}", p.notes);
    assert_eq!(p.categories, ["metal", "man made", "floor"]);
    assert_eq!(p.tags, ["rusty", "steel"]);
    assert!(b.categories.is_empty(), "non nel catalogo: nessuna categoria inventata");
    assert!(p
        .material
        .metallic_texture
        .as_ref()
        .unwrap()
        .ends_with("metal_plate_02_metal_2k.exr"));
    assert!(!p.material.normal_directx);
    // Gli EXR spostati restano leggibili.
    let nor = image::open(p.material.normal_texture.as_ref().unwrap()).unwrap().to_rgb8();
    assert!(nor.get_pixel(0, 0).0[2] > 200);
    assert!(p.dir.join(KEPT_MANIFEST).is_file());
    assert!(!plate.join(MANIFEST).exists(), "il manifest è stato spostato");
    // Nei download restano solo le mappe non usate e i file per altri programmi.
    assert_eq!(
        names(&plate.join("maps")),
        [
            "metal_plate_02_2k.blend",
            "metal_plate_02_2k.gltf",
            "metal_plate_02_arm_2k.exr",
            "metal_plate_02_nor_dx_2k.exr"
        ]
    );
    let left = polyhaven_local::leftovers(&all.imported_dirs);
    let mut kinds: Vec<&str> = left.iter().map(|(k, _, _)| k.as_str()).collect();
    kinds.sort();
    assert_eq!(kinds, [".blend", ".gltf", "arm .exr", "nor_dx .exr"]);
    // La cartella in download non è stata toccata.
    assert_eq!(names(&busy.join("maps")), ["aerial_rocks_02_diff_2k.jpg"]);
    assert_eq!(store::list().len(), 2);

    // Materiale importato senza categorie: si completano dal catalogo, gli altri non cambiano.
    let bark_info = OnlineAsset {
        id: "bark_brown_01".into(),
        name: "Bark Brown 01".into(),
        category: "bark".into(),
        categories: vec!["bark".into(), "natural".into()],
        tags: vec!["tree".into()],
        size_m: Some(1.0),
        ..catalog["metal_plate_02"].clone()
    };
    let list = vec![catalog["metal_plate_02"].clone(), bark_info];
    let r = forge_library::online::fill_catalog_info(&[(Provider::PolyHaven, list)]);
    assert_eq!((r.updated, r.unchanged, r.local), (1, 1, 0), "{r:?}");
    assert!(r.not_found.is_empty());
    assert_eq!(store::load(&b.dir).unwrap().categories, ["bark", "natural"]);
    let again = forge_library::online::fill_catalog_info(&[(Provider::PolyHaven, Vec::new())]);
    assert_eq!(again.unchanged, 2);

    std::fs::remove_dir_all(lib).ok();
    std::fs::remove_dir_all(dl).ok();
}
