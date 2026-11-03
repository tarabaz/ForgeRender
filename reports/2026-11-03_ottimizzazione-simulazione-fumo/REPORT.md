# Accelerazione della simulazione della nebbia — report delle misure

**Data:** 03/11/2026 · **Autore:** Claude Code · **Branch:** `sviluppo` (portato su `main` come 0.32.0)
**Piano di riferimento:** [`PLAN.md`](PLAN.md) (ChatGPT, 03/11/2026)
**Hardware:** PC di Valerio, AMD Ryzen 9 9900X (24 thread logici), Windows 11. La simulazione gira
solo sulla CPU.

## Scena e metodo

- Scena: demo del cocktail con ghiaccio secco (`samples/scene/cocktail-ghiaccio-secco`), con le sue
  collisioni vere (coppa, bancone), seed e parametri della demo; cambia solo la qualità
  (`sim_cells`/`resolution` dei preset Media, Alta, Molto alta, Ultra).
- Programma: `cargo run --release -p forge_project --example fog_bench -- CARTELLA 3` (nuovo):
  carica il progetto, misura separatamente ostacoli (voxelizzazione), simulazione e dettaglio;
  3 ripetizioni per qualità, mediana; salva il campo simulato per il confronto
  (`FOG_BENCH_REF=cartella`). Con `FORGE_FOG_PROFILE=1` stampa i tempi per fase.
- Qualità: massa totale della nebbia; differenza RMSE cella per cella; differenza relativa della
  forma su medie a blocchi (lato ≈ 1/24 della griglia), che è ciò che si vede nell'immagine.
  La nebbia è caotica: anche cambiare solo il passo temporale sposta i singoli filamenti, quindi
  il confronto per cella da solo non è significativo.

## Risultato (prima generazione, senza cache)

| Qualità | Griglia sim | Prima (mediana, 3 run) | Dopo (mediana, 3 run) | Guadagno | Massa | Forma (blocchi) |
|---|---|---|---|---|---|---|
| Media | 96×68×96 | 20 882 ms (21 311, 20 882, 20 553) | 8 006 ms (8 710, 7 803, 8 006) | 2,6× | 0,000358 → 0,000357 | 3,5% |
| Alta | 128×90×128 | 11 395 ms (11 180, 11 395, 11 668) | 3 659 ms (3 676, 3 546, 3 659) | 3,1× | 0,000494 → 0,000496 | 3,9% |
| Molto alta | 192×135×192 | 60 850 ms (61 428, 58 126, 60 850) | 14 489 ms (14 489, 14 376, 14 624) | 4,2× | 0,000580 → 0,000586 | 4,9% |
| Ultra | 256×180×256 | 182 341 ms (181 619, 184 796, 182 341) | 36 946 ms (36 946, 40 449, 36 413) | 4,9× | 0,000701 → 0,000702 | 6,0% |

Ostacoli e dettaglio non cambiano (ostacoli 2–14 ms, dettaglio 0,07–1 s). Il target minimo del
piano (5× a qualità alta) è raggiunto quasi: 4,9× a Ultra in mediana (una corsa singola con il
profilo attivo ha dato 35,1 s, 5,2×), 4,2× a Molto alta. Media e Alta guadagnano meno perché la
griglia è piccola e la nebbia ne occupa una parte maggiore.

## Collo di bottiglia misurato (profilo per fase, Ultra, prima)

`[sorgente 678, forze 34 236, trasporto velocità 19 665, pressione 56 747, trasporto nebbia
39 599, dissolvenza 3 231, velocità massima 26 288]` ms su 608 passi. Cause trovate:

1. ogni passo creava e azzerava campi grandi come **tutta la griglia** (12 M celle: forze,
   vortici, divergenza, trasporto, copia della velocità), più di 1 GB di memoria toccato per
   passo, anche con la nebbia su meno del 25% della griglia;
2. la **velocità massima** (per il passo temporale) scorreva tutta la griglia a ogni passo e
   contava anche velocità "ferme" rimaste fuori dalla nebbia;
3. la **pressione** (Gauss–Seidel rosso/nero, 16 iterazioni) leggeva per ogni cella una tabella
   di 28 byte con gli indici dei vicini, 32 volte per passo (≈ 2,6 GB letti per passo);
4. il riquadro di lavoro (24% della griglia) è in gran parte aria vuota: le celle entro 6 dalla
   nebbia sono il 7,7–13%, quelle vicine (3 celle) il 5,5–7,5%.

## Interventi (in ordine, con l'effetto misurato a Ultra)

1. **Campi del riquadro** (`Local`): ogni campo di un passo è grande quanto il riquadro della
   nebbia; velocità massima e dissolvenza solo nel riquadro; buffer della griglia riusati invece
   che ricreati; fuori dal riquadro i campi restano a zero (`clear_outside`). Fisica identica.
   182 → 104 s. Differenza per cella 0,003 RMSE (solo dal passo temporale, che non conta più le
   velocità ferme fuori dalla nebbia).
2. **Tabella dei vicini compatta** per la pressione: 2 byte per cella (quali vicini sono liberi
   e quante facce non sono solide), indici calcolati con gli spostamenti; somma nello stesso
   ordine. Risultato identico al bit. Pressione 53 → 20 s; totale 104 → 69 s.
3. **Dominio per tratti di riga** (`spans_of`, `face_rows`, `update_rows`, `fill_spans`): forze,
   trasporti e gradiente solo sulle celle entro il margine di 6 celle dalla nebbia (fuori non
   facevano niente: parte identica); la pressione si risolve su quel dominio, con p = 0 oltre
   (aria aperta, come prima ai bordi del riquadro). Unico cambiamento fisico. 69 → 51 s.
4. **Buffer riusati** tra i passi per i campi del dominio (non azzerati: fuori dai tratti non si
   leggono) e **velocità massima** misurata nel passaggio che aggiorna già le velocità
   (`update_rows_max`). Risultato identico. 51 → 39 s.
5. **Dilatazione a finestra scorrevole** su righe intere (`dilate`, `dilate_mask`) invece che a
   salti lungo y; il dominio si ottiene allargando le celle vicine invece che ripartendo dalla
   nebbia. Risultato identico al bit. 39 → 35–37 s.

Profilo finale a Ultra (con profilo attivo): `[1 586, 9 582, 7 958, 7 019, 6 695, 360, 380]` ms.

## Margine della pressione: perché 6 celle

| Margine | Ultra | Forma Ultra | Molto alta | Forma Molto alta |
|---|---|---|---|---|
| 6 (scelto, come il riquadro di prima) | 39,1 s | 6,0% | 16,2 s | 4,9% |
| 9 | 44,1 s | 5,5% | 20,6 s | 3,4% |
| 12 | 47,9 s | 5,0% | 19,2 s | 3,2% |

Allargare il margine costa il 13–22% e recupera poco: gran parte della differenza viene dal
comportamento caotico della nebbia, non dal margine. Massa invariata in tutti i casi.

## Test eseguiti

- `cargo test --release -p forge_volume`: 16 test passati dopo ogni intervento (stesso seed
  stesso risultato, indipendente dal numero di thread, la nebbia si posa e si allarga, il muro la
  ferma, riempie la coppa e cola, la risalita, il majorant).
- Suite completa del workspace (`cargo test --workspace --exclude forge_step --release`) e
  `cargo clippy ... -D warnings` prima del commit (vedi la nota in `CLAUDE_HANDOFF.md`).

## Fatto anche in questo giro (interfaccia)

- Calcolo della nebbia **in anticipo**: 0,8 s dopo l'ultima modifica della nebbia o della scena
  parte da solo in sottofondo (prima prepara la BVH se serve); il render, se premuto prima, parte
  appena finisce. Un calcolo in anticipo non blocca il pulsante Render.
- **Barra di avanzamento** nella barra in basso, a destra dei campioni: «Nebbia: simulazione 45%»
  (ostacoli, simulazione, dettaglio; «Nebbia 1 di 2» con più nebbie); «costruzione BVH» solo per
  la BVH vera. Modulo `forge_volume::progress`.

## Limiti e prossimi passi

- A Media e Alta il guadagno è 2,6–3,1×: lì il costo fisso per passo (dilatazioni sul riquadro)
  pesa di più. Ridurlo con la dilatazione solo sui tratti del passo prima.
- La pressione resta Gauss–Seidel a 16 iterazioni: un multigrid o un arresto sul residuo
  potrebbero dimezzarla ancora, ma cambiano il campo di velocità (da misurare come sopra).
- GPU (wgpu): non valutata in questo giro; con tutto limitato al dominio, il costo residuo è
  ≈ 60 ms per passo a Ultra, distribuito su più fasi.
- Cache su disco: non fatta (non conta per la prima generazione, come chiesto dal piano).
- Confronto visivo delle immagini renderizzate: non fatto in questo giro; i numeri sopra sono
  sul campo simulato.
