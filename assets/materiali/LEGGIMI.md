# Mappe dei materiali predefiniti

Una cartella per materiale predefinito (`<id>` come in `forge_scene::material::presets`), con
`colore.jpg` (sRGB), `rugosita.jpg` e `normal.jpg` (lineari, normal OpenGL), 1024 px. Il
programma le carica da `materiali\` accanto all'eseguibile (pacchetti) o da qui (compilazione
locale); i parametri (lato della ripetizione, colore base, rugosità) sono in `MAPPED` in
`crates/forge_scene/src/material.rs`.

Si rifanno con:

```text
cargo run --release -p forge_render_core --example materiali_predefiniti -- <Libreria\Materiali> assets\materiali
```

| Cartella | Fonte | Licenza |
|---|---|---|
| `legno-rovere` | Poly Haven, [Oak Veneer 01](https://polyhaven.com/a/oak_veneer_01) | CC0 |
| `pelle-nera` | Poly Haven, [Leather White](https://polyhaven.com/a/leather_white) (colore reso grigio, tinta dal materiale) | CC0 |
| `marmo-bianco` | Generato dallo script (venature su fondo bianco) | parte di ForgeRender |

Solo mappe con licenza che ne permetta la ridistribuzione (CC0 o generate): finiscono nel
programma e nel repository.
