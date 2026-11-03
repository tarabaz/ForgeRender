//! Controllo di un progetto o pacchetto: `inspect_pack FILE` stampa cosa si carica;
//! `inspect_pack FILE --repack OUT.frpack` lo risalva come pacchetto (modelli, pezzi degli
//! assiemi STEP e conversioni compresi).
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let input = std::path::PathBuf::from(&args[1]);
    let project = if input.extension().is_some_and(|e| e == "frpack") {
        let dir = std::env::temp_dir().join("forge-inspect-pack");
        let _ = std::fs::remove_dir_all(&dir);
        forge_project::pack::unpack(&input, &dir)?
    } else {
        input
    };
    let t0 = std::time::Instant::now();
    let l = forge_project::load(&project)?;
    let s = &l.scene;
    let meshes = s.nodes.iter().filter(|n| n.mesh.is_some()).count();
    println!(
        "caricato in {:.1} s: nodi {} con mesh {} triangoli {}",
        t0.elapsed().as_secs_f32(),
        s.nodes.len(),
        meshes,
        s.triangle_count()
    );
    println!("avvisi {:?}", l.report.warnings);
    println!(
        "componenti mancanti {} sorgenti mancanti {:?}",
        l.report.missing_components.len(),
        l.report.missing_sources
    );
    if args.get(2).map(String::as_str) == Some("--repack") {
        let out = std::path::PathBuf::from(&args[3]);
        let r = forge_project::pack::save_pack(&out, s, &l.meta, &l.settings)?;
        println!("pacchetto {}: {r:?}", out.display());
    }
    Ok(())
}
