# Demo: cocktail con ghiaccio secco

Scena dimostrativa di ForgeRender: coppa da cocktail con nebbia di ghiaccio secco che
trabocca, bancone in noce, scaffale di bottiglie e lucine sfocate (bokeh) sullo sfondo.

## Come aprirla

1. Copia l'intera cartella `cocktail-ghiaccio-secco` dove vuoi (i percorsi sono relativi).
2. In ForgeRender: **File → Apri progetto…** e scegli `cocktail-ghiaccio-secco.frproject`.
3. Scheda **Render**, backend **GPU**, poi **Avvia**. Al primo avvio la nebbia viene simulata
   (qualche secondo), poi resta in cache finché non cambi i parametri del moto.

Impostazioni salvate: 1080×1350 (formato verticale 4:5, va bene per Instagram), 1024 campioni,
16 rimbalzi, camera "Demo" da 42 mm a f/2.8 con messa a fuoco sul bicchiere.

## Cosa c'è dentro

| Cartella | Contenuto |
|---|---|
| `modelli/bar_cocktail.glb` | coppa, cocktail, ciliegina, shaker, jigger, bancone, muro, mensole, bottiglie, lucine (generati con `sorgenti/genera_bar.py`) |
| `modelli/IridescentDishWithOlives.glb` | piatto di olive (coperchio nascosto) |
| `modelli/GlassHurricaneCandleHolder.glb` | portacandela in vetro |
| `textures/` | legno di noce procedurale: colore, rugosità, rilievo |

All'apertura ForgeRender estrae le texture dei .glb in cartelle `*_texture` accanto ai modelli:
si possono cancellare, vengono ricreate.

## Da provare

- Scheda **Effetti** → "Ghiaccio secco nel cocktail": cambia *Tempo simulato* o *Risalita* e
  guarda come cambia la colata; i preset in alto mostrano altri comportamenti.
- Scheda **Camera**: apri il diaframma (f/1.8) per lucine più grandi, o chiudilo per vedere lo scaffale.
- Luci: il **Controluce** dietro il bicchiere è quello che fa brillare la nebbia.

Crediti e licenze: vedi [CREDITI.md](CREDITI.md).
