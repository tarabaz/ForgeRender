//! Compila Open CASCADE (statico, tramite `occt-sys`) e lo shim C++ che legge
//! gli assiemi STEP con XCAF. La prima compilazione di OCCT richiede decine di
//! minuti; poi resta in `target/OCCT` e viene riutilizzata.

use std::path::PathBuf;

fn main() {
    // CMake 4 rifiuta i progetti che dichiarano compatibilità con versioni < 3.5, come il
    // CMakeLists di OCCT 7.8: si usa l'opzione di compatibilità indicata da CMake stesso.
    if std::env::var_os("CMAKE_POLICY_VERSION_MINIMUM").is_none() {
        // SAFETY: lo script di build è single-thread a questo punto.
        unsafe { std::env::set_var("CMAKE_POLICY_VERSION_MINIMUM", "3.5") };
    }
    occt_sys::build_occt();
    let occt = occt_sys::occt_path();
    let include = occt.join("include");
    let lib = occt.join("lib");
    println!("cargo:warning=OCCT in {}", occt.display());

    let shim = PathBuf::from("cpp/step_shim.cpp");
    if shim.exists() {
        cc::Build::new()
            .cpp(true)
            .file(&shim)
            .include(&include)
            // OCCT è compilato statico con questa macro: senza, su Windows gli header
            // dichiarano i simboli come `dllimport` e i dati statici (RTTI) non coincidono.
            .define("OCCT_STATIC_BUILD", None)
            .flag_if_supported("-std=c++17")
            .flag_if_supported("/std:c++17")
            .flag_if_supported("/EHsc")
            .warnings(false)
            .compile("forge_step_shim");
        println!("cargo:rerun-if-changed=cpp/step_shim.cpp");
    }

    println!("cargo:rustc-link-search=native={}", lib.display());
    // Ordine topologico (dai toolkit di livello più alto a quelli di base), ricavato dai
    // file EXTERNLIB di OCCT: necessario per i linker GNU/LLD, indifferente per MSVC.
    for tk in [
        "TKDESTEP",
        "TKXCAF",
        "TKVCAF",
        "TKV3d",
        "TKHLR",
        "TKMesh",
        "TKService",
        "TKXSBase",
        "TKDE",
        "TKCAF",
        "TKLCAF",
        "TKCDF",
        "TKBO",
        "TKShHealing",
        "TKPrim",
        "TKTopAlgo",
        "TKGeomAlgo",
        "TKBRep",
        "TKGeomBase",
        "TKG3d",
        "TKG2d",
        "TKMath",
        "TKernel",
    ] {
        println!("cargo:rustc-link-lib=static={tk}");
    }
    let target = std::env::var("TARGET").unwrap();
    if target.contains("windows") {
        for l in ["user32", "advapi32", "gdi32", "ws2_32", "shell32"] {
            println!("cargo:rustc-link-lib={l}");
        }
    } else if target.contains("linux") {
        println!("cargo:rustc-link-lib=stdc++");
        println!("cargo:rustc-link-lib=pthread");
        println!("cargo:rustc-link-lib=dl");
    }
}
