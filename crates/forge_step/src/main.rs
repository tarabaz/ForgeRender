//! `forgerender-step`: converte un file STEP nel formato intermedio `.frmesh`.
//!
//! ```text
//! forgerender-step <input.step> <output.frmesh> [--quality bassa|media|alta]
//! ```
//!
//! Gira come processo separato dall'applicazione: un crash di Open CASCADE su un
//! file malformato non chiude ForgeRender, e la libreria LGPL resta confinata qui.
//! Lo stdout riporta righe `progresso: ...`; in caso di errore il codice d'uscita
//! è diverso da zero e il messaggio è su stderr.

use forge_import::frmesh::{FrMesh, Header, MeshData, NodeRecord};
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::PathBuf;

#[repr(C)]
struct FrStepCallbacks {
    user: *mut c_void,
    mesh: extern "C" fn(*mut c_void, *const f32, *const f32, c_int, *const u32, c_int) -> c_int,
    node: extern "C" fn(*mut c_void, c_int, *const c_char, *const f64, c_int, f32, f32, f32, c_int) -> c_int,
    progress: extern "C" fn(*mut c_void, *const c_char),
}

extern "C" {
    fn fr_step_convert(
        path_utf8: *const c_char,
        linear_deflection: f64,
        angular_deflection: f64,
        cb: *const FrStepCallbacks,
        unit_in_meters: *mut f64,
        error: *mut c_char,
        error_len: c_int,
    ) -> c_int;
    fn fr_step_write_test_assembly(path_utf8: *const c_char, error: *mut c_char, error_len: c_int) -> c_int;
    /// Uscita immediata della libreria C (glibc e UCRT), senza distruttori statici.
    fn _exit(code: c_int) -> !;
}

extern "C" fn on_mesh(user: *mut c_void, pos: *const f32, nrm: *const f32, nv: c_int, idx: *const u32, nt: c_int) -> c_int {
    // SAFETY: `user` è il `FrMesh` passato da `main`; i puntatori sono validi per
    // `nv`·3 float e `nt`·3 indici per tutta la durata della chiamata (garantito dallo shim).
    let out = unsafe { &mut *(user as *mut FrMesh) };
    let (nv, nt) = (nv.max(0) as usize, nt.max(0) as usize);
    let p = unsafe { std::slice::from_raw_parts(pos, nv * 3) };
    let n = unsafe { std::slice::from_raw_parts(nrm, nv * 3) };
    let i = unsafe { std::slice::from_raw_parts(idx, nt * 3) };
    out.meshes.push(MeshData {
        positions: p.as_chunks::<3>().0.to_vec(),
        normals: n.as_chunks::<3>().0.to_vec(),
        indices: i.as_chunks::<3>().0.to_vec(),
    });
    (out.meshes.len() - 1) as c_int
}

#[allow(clippy::too_many_arguments)]
extern "C" fn on_node(
    user: *mut c_void,
    parent: c_int,
    name: *const c_char,
    m: *const f64,
    has_color: c_int,
    r: f32,
    g: f32,
    b: f32,
    mesh: c_int,
) -> c_int {
    // SAFETY: come sopra; `name` è una stringa C terminata, `m` punta a 16 double.
    let out = unsafe { &mut *(user as *mut FrMesh) };
    let name = unsafe { CStr::from_ptr(name) }.to_string_lossy().into_owned();
    let mut matrix = [0.0f64; 16];
    matrix.copy_from_slice(unsafe { std::slice::from_raw_parts(m, 16) });
    out.header.nodes.push(NodeRecord {
        name,
        parent: (parent >= 0).then_some(parent as u32),
        matrix,
        color: (has_color != 0).then_some([r, g, b]),
        mesh: (mesh >= 0).then_some(mesh as u32),
    });
    (out.header.nodes.len() - 1) as c_int
}

extern "C" fn on_progress(_user: *mut c_void, msg: *const c_char) {
    // SAFETY: stringa C terminata fornita dallo shim.
    let msg = unsafe { CStr::from_ptr(msg) }.to_string_lossy();
    println!("progresso: {msg}");
}

fn main() {
    let code = run();
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    // I distruttori statici di OCCT compilato statico possono andare in crash all'uscita
    // su Windows: tutto è già scritto su disco, quindi si esce senza eseguirli.
    // SAFETY: nessuna risorsa Rust da rilasciare oltre ai buffer appena svuotati.
    unsafe { _exit(code) }
}

fn run() -> c_int {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // Genera un assieme STEP di prova (usato dai test automatici).
    if args.first().map(String::as_str) == Some("--write-test-assembly") {
        let Some(out) = args.get(1).and_then(|p| CString::new(p.as_bytes()).ok()) else {
            eprintln!("uso: forgerender-step --write-test-assembly <out.step>");
            return 2;
        };
        let mut err = vec![0 as c_char; 512];
        // SAFETY: puntatori validi per la durata della chiamata.
        let rc = unsafe { fr_step_write_test_assembly(out.as_ptr(), err.as_mut_ptr(), err.len() as c_int) };
        if rc != 0 {
            eprintln!("{}", unsafe { CStr::from_ptr(err.as_ptr()) }.to_string_lossy());
            return 1;
        }
        return 0;
    }
    if args.len() < 2 {
        eprintln!("uso: forgerender-step <input.step> <output.frmesh> [--quality bassa|media|alta]");
        return 2;
    }
    let input = PathBuf::from(&args[0]);
    let output = PathBuf::from(&args[1]);
    let quality = args
        .iter()
        .position(|a| a == "--quality")
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
        .unwrap_or("media");
    // Deflessione lineare relativa alla dimensione di ogni spigolo e deflessione angolare (radianti).
    let (lin, ang) = match quality {
        "bassa" => (0.01, 0.5),
        "alta" => (0.0015, 0.15),
        _ => (0.004, 0.3),
    };
    let Ok(path_c) = CString::new(input.to_string_lossy().as_bytes()) else {
        eprintln!("percorso non valido");
        return 2;
    };
    let t0 = std::time::Instant::now();
    let mut out = FrMesh {
        header: Header {
            unit_m: 0.001,
            source: input.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
            ..Default::default()
        },
        meshes: Vec::new(),
    };
    let cb = FrStepCallbacks {
        user: &mut out as *mut FrMesh as *mut c_void,
        mesh: on_mesh,
        node: on_node,
        progress: on_progress,
    };
    let mut unit = 0.001f64;
    let mut err = vec![0 as c_char; 1024];
    // SAFETY: tutti i puntatori restano validi per la durata della chiamata.
    let rc = unsafe { fr_step_convert(path_c.as_ptr(), lin, ang, &cb, &mut unit, err.as_mut_ptr(), err.len() as c_int) };
    if rc != 0 {
        let msg = unsafe { CStr::from_ptr(err.as_ptr()) }.to_string_lossy();
        eprintln!("{msg}");
        return 1;
    }
    out.header.unit_m = unit;
    if let Err(e) = out.write(&output) {
        eprintln!("scrittura di {} fallita: {e}", output.display());
        return 1;
    }
    let tris: usize = out.meshes.iter().map(|m| m.indices.len()).sum();
    println!(
        "fatto: {} nodi, {} mesh, {} triangoli, unità {} m, {:.2} s",
        out.header.nodes.len(),
        out.meshes.len(),
        tris,
        unit,
        t0.elapsed().as_secs_f64()
    );
    0
}
