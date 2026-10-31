# Interfaccia Semplice: cosa si vede e cosa è preimpostato

🇬🇧 [Read in English](INTERFACCIA_SEMPLICE.md) (versione di riferimento)

Scheda di configurazione della **modalità Semplice** di ForgeRender. Serve per decidere insieme
come deve essere impostata per avere la resa migliore: modificate i valori nella colonna
**"Proposta"**, aggiungete le vostre note nella colonna **"Decisione"**, e i valori vengono
riportati nel codice (`crates/forge_app/src/presets.rs`).

Stato: **implementata** (fasi 1–3, report in `reports/2026-10-29_interfaccia-semplice/`). Il
selettore Semplice/Avanzata è in alto a sinistra; la Semplice è la modalità predefinita. I valori
di questa scheda sono **quelli del codice**.

## 1. Cosa si vede nella modalità Semplice (pannello di destra, dall'alto in basso)

Pulsanti grandi e con il testo scritto, non icone nascoste. Ogni pulsante ha un suggerimento che
spiega cosa fa.

| # | Sezione | Controllo | Cosa fa davvero | Decisione |
|---|---|---|---|---|
| 0 | Barra in alto | Selettore **Semplice / Avanzata** (predefinita: Semplice; ricordata nelle Impostazioni) | Cambia solo il pannello di destra; i valori restano gli stessi | |
| 1 | Prodotto | **Importa modello…** | Stesso import del menu File (GLB, STEP, OBJ, STL, SolidWorks su Windows) | |
| 1 | Prodotto | **Appoggia sulla base** | Porta il punto più basso del prodotto a quota 0, senza spostarlo in orizzontale (tutte le parti insieme) | |
| 1 | Prodotto | **Inquadra il prodotto** | Camera sul prodotto (senza pavimento e fondale); muove anche la camera di render attiva | |
| 1 | Prodotto | **Adatta la scena al prodotto** | Dopo aver spostato o scalato il prodotto: rifà fondale e luci sulle sue misure (tiene materiale del fondale e colori delle luci) e inquadra. Si annulla con "Annulla preset" | |
| 2 | Aspetto | 4 schede grandi: **Studio fotografico** (quella di partenza), **Sfondo chiaro**, **Sfondo scuro**, **Studio scenografico** | Applicano i valori della tabella 2 | |
| 2 | Aspetto | **Colore delle luci** (richiudibile): Calda 3200 K, Neutra 5500 K, Fredda 7500 K, più un selettore di colore per ogni luce | Cambia solo il colore; l'aspetto non diventa "Personalizzato" | |
| 2 | Aspetto | **Prepara scena prodotto** (pulsante principale) | Appoggia sulla base, aggiunge il fondale se manca, applica aspetto e qualità, inquadra | |
| 2 | Aspetto | **Annulla preset** | Ripristina luci, ambiente, visualizzazione, qualità, fondale e posizione com'erano prima dell'ultimo preset; le altre modifiche restano | |
| 2 | Aspetto | Indicatore **"Personalizzato"** | Compare se luci, ambiente o visualizzazione sono stati cambiati (anche in Avanzata). Non riporta indietro niente da solo | |
| 3 | Materiali | **Apri la libreria materiali** + istruzione "clicca un oggetto, poi un materiale" | Stessa libreria della modalità Avanzata | |
| 4 | Formato | **Orizzontale 16:9**, **Quadrato 1:1**, **Verticale 4:5**, **Storie 9:16** | Imposta la risoluzione (tabella 4) | |
| 4 | Qualità | **Bozza**, **Bilanciata**, **Alta** | Imposta campioni, rimbalzi e denoiser (tabella 3) | |
| 4 | Dispositivo di calcolo | Due pulsanti: **Scheda video (GPU)** e **Processore (CPU)** | Predefinita la GPU appena risulta funzionante (una volta sola: poi vale la scelta dell'utente o di Claude). Senza GPU utilizzabile il pulsante è disattivato | |
| 5 | Render | **Avvia render** (grande) con avanzamento | Come F5 | |
| 5 | Render | **Salva immagine…** | PNG, JPG, TIFF 16 bit o EXR (si sceglie nella finestra), come File → Esporta immagine… | |

**Non si vede nella Semplice** (resta tutto nella Avanzata): trasformazioni numeriche,
gizmo, singole luci, HDRI, nebbia ed effetti, obiettivo e profondità di campo, thread, seed,
rimbalzi, Russian roulette, clamp, tone mapping, esposizione, regole di assegnazione dei
materiali.

**Sempre attivi nella Semplice** (non si vedono, ma valgono sempre; in Avanzata sono nella
scheda Render, e tornando in Semplice si riapplicano — `presets::apply_simple_base`):

| Impostazione | Valore |
|---|---|
| Arresto a soglia di rumore | attivo, soglia 0,030 (da 0.23.0; prima 0,050), campioni minimi 32 |
| Denoiser: modalità | Anteprima veloce + finale (ripulisce anche durante il render) |
| Denoiser: CUDA | attivo (se la scheda non lo supporta, il denoiser usa da solo la CPU) |
| Denoiser: qualità finale / anteprima | Alta / Veloce |
| Denoiser: prima anteprima | a 8 campioni |
| Denoiser: intensità | 100 % (da 0.23.0; prima 90 %) |
| Denoiser: preserva i dettagli (albedo e normali) | sì |

Nota sull'arresto a soglia di rumore: nella scena del cocktail (nebbia, scena scura) era più
lento del render uniforme (`reports/2026-10-29_campionamento-adattivo/`); per le foto di prodotto
su fondale chiaro di solito conviene. In Avanzata resta spento di default.

## 2. Aspetti: valori impostati (nascosti nella Semplice, visibili in Avanzata)

Le luci sono i preset di `forge_scene::light::preset`, **dimensionati sul prodotto** (centro e
raggio dell'ingombro, esclusi pavimento e fondale).

| Parametro | Studio fotografico | Sfondo chiaro | Sfondo scuro | Studio scenografico | Decisione |
|---|---|---|---|---|---|
| Preset luci | `studio-fotografico`: softbox principale, riempimento, luce dall'alto, luce del fondale (tutte invisibili alla camera) | `studio`: softbox principale + riempimento | `tre-punti`: key, fill, rim | `cinematografica`: key calda 3200 K + controluce fredda 9000 K | |
| Fondale | **tela inclinata** (piano + curva + parete inclinata di 10° all'indietro) | curvo | curvo | curvo | |
| Ambiente | Uniforme bianco, intensità 0,12 | Uniforme bianco, intensità 0,9 | Uniforme, intensità 0 (nero) | Gradiente scurissimo, intensità 0,5 | |
| Ambiente visibile in camera | sì | sì | sì | sì | |
| Fondale: colore | grigio tela 0,80 | grigio chiaro 0,85 | quasi nero 0,03 | bruno scuro 0,05 | |
| Fondale: rugosità | 0,95 (tela) | 0,65 (opaco) | 0,35 (lucido, riflessi) | 0,30 | |
| Tone mapping | ACES filmico | ACES filmico | ACES filmico | ACES filmico | |
| Esposizione | 0 EV | 0 EV | 0 EV | +0,3 EV | |

Misure della tela (dal prodotto, `e` = raggio dell'ingombro): altezza 2,5·e (almeno 2,5 m),
larghezza 7·e (almeno 6 m), curva 0,3·altezza, piede 0,35·e dietro il prodotto.

**Misurato** (test `looks_change_the_rendered_image`, sfera su fondale, render CPU 48×32,
8 campioni): luminosità media 0,773 con "Sfondo chiaro", 0,039 con "Sfondo scuro".

## 3. Qualità: valori impostati

| Parametro | Bozza | Bilanciata | Alta | Decisione |
|---|---|---|---|---|
| Campioni per pixel | 64 | 256 | 1024 | |
| Rimbalzi massimi | 8 | 12 | 16 | |
| Denoiser | anteprima + finale | anteprima + finale | anteprima + finale | |

Restano come sono (si cambiano in Avanzata): thread (automatico), Russian roulette (dal
rimbalzo 3), clamp indiretto (0), seed. Denoiser e arresto a soglia di rumore: vedi "Sempre
attivi nella Semplice" sopra.

## 4. Formati

| Nome | Rapporto | Risoluzione | Decisione |
|---|---|---|---|
| Orizzontale | 16:9 | 1920 × 1080 | |
| Quadrato | 1:1 | 1080 × 1080 (Instagram) | |
| Verticale | 4:5 | 1080 × 1350 (Instagram) | |
| Storie | 9:16 | 1080 × 1920 | |

## 5. "Prepara scena prodotto", passo per passo

1. Salva i valori che sta per cambiare (per "Annulla preset").
2. Appoggia il prodotto sulla base (quota 0), senza spostarlo in orizzontale.
3. Se non c'è un fondale, ne aggiunge uno dimensionato sul prodotto (stessa regola di
   Oggetti → Fondale da studio). Se c'è già, lo riusa.
4. Applica l'aspetto scelto (tabella 2) e la qualità scelta (tabella 3).
5. Inquadra il prodotto.

Ripeterla non aggiunge un secondo fondale. Il materiale del fondale è "Fondale (preset)": si
può cambiare con la libreria come ogni altro materiale.

## 6. Domande aperte

- Serve un quarto aspetto, per esempio **"Su tavolo di legno"** (pavimento con materiale legno
  della libreria)? Va bene solo se il materiale è nella libreria del PC.
- Bozza a 128 campioni (da 0.21.0; prima 64, giudicata troppo bassa da Valerio).
- L'aspetto "Sfondo scuro" mette l'ambiente a 0: i metalli riflettono solo luci e fondale. Va bene,
  o preferite un riflesso leggero (ambiente 0,05)?
