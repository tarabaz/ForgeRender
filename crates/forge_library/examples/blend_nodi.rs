//! Stampa materiali, nodi e collegamenti di un file `.blend` (anche dentro i gruppi di nodi):
//! serve a capire come è fatto un materiale prima di convertirlo.
//!
//! ```text
//! cargo run --release -p forge_library --example blend_nodi -- file.blend
//! ```

use forge_library::blend::{Blend, Inst};

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("file .blend");
    let b = Blend::open(std::path::Path::new(&path))?;
    for m in b.blocks.iter().filter(|x| &x.code[..2] == b"MA").map(|x| b.at(x)) {
        let name = m.sub("id").map(|i| i.str("name")).unwrap_or_default();
        println!("MATERIALE {name}");
        if let Some(tree) = m.deref("nodetree") {
            dump(&tree, 1);
        }
    }
    if std::env::args().any(|a| a == "--campi") {
        for ty in ["Image", "bNode", "bNodeSocket", "PackedFile"] {
            let names: Vec<String> = b.fields(ty).iter().map(|f| format!("{}:{}", f.name, f.ty)).collect();
            println!("{ty}: {}", names.join(", "));
        }
    }
    for im in b.blocks.iter().filter(|x| &x.code[..2] == b"IM").map(|x| b.at(x)) {
        let name = im.sub("id").map(|i| i.str("name")).unwrap_or_default();
        let packed = im.deref("packedfile").is_some() || !im.list("packedfiles").is_empty();
        println!("IMMAGINE {name}  file «{}»  impacchettata: {packed}", im.str("filepath"));
    }
    Ok(())
}

fn dump(tree: &Inst, depth: usize) {
    let pad = "  ".repeat(depth);
    for n in tree.list("nodes") {
        let idname = n.str("idname");
        println!("{pad}nodo «{}» {idname}", n.str("name"));
        for s in n.list("inputs") {
            let v = s.deref("default_value").map(|d| d.f32s("value")).unwrap_or_default();
            let link = s.deref("link").map(|l| {
                let from = l.deref("fromnode").map(|f| f.str("name")).unwrap_or_default();
                let sock = l.deref("fromsock").map(|f| f.str("identifier")).unwrap_or_default();
                format!(" ← «{from}».{sock}")
            });
            if link.is_some() || !v.is_empty() {
                println!("{pad}    in {} {:?}{}", s.str("identifier"), v, link.unwrap_or_default());
            }
        }
        if idname == "ShaderNodeGroup" {
            if let Some(g) = n.deref("id") {
                println!("{pad}  GRUPPO {}", g.sub("id").map(|i| i.str("name")).unwrap_or_default());
                dump(&g, depth + 2);
            }
        }
    }
}
