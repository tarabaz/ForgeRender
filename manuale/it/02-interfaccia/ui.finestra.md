---
id: ui.finestra
titolo: La finestra principale
parole: finestra, disposizione, menu, barra degli strumenti, pannelli, barra di stato, albero, componenti, schede
ui: ui.window, ui.statusbar
correlate: ui.viewport, ui.impostazioni, intro.semplice
---
# La finestra principale

| Zona | Dove | Cosa contiene |
|---|---|---|
| Barra dei menu | in alto | File, Modifica, Scena, Oggetti, Vista, Rendering, Libreria, Strumenti, Guida; a destra l'interruttore **Semplice / Avanzata** e l'ingranaggio delle **Impostazioni** |
| Barra degli strumenti | sotto i menu | a sinistra Nuovo, Apri, Salva, Importa e i menu **Aggiungi**, **Luci**, **Camera**, **Ambiente**, **Materiale**; a destra **Avvia render**, Stop, Riavvia, Esporta e l'interruttore **AI MCP** |
| Pannello Scena | a sinistra | un solo albero: gli oggetti del modello, poi i gruppi **Luci**, **Camere** ed **Effetti** |
| Area centrale | al centro | le linguette **Viewport** e **Render** in alto, con **Riavvio automatico** a destra |
| Pannello delle impostazioni | a destra | Semplice: i 5 passi. Avanzata: le schede **Selezione**, **Scena** e **Render** |
| Parte bassa | in basso | stato del render e contatori; **Timeline** e **Console**, ridotte a una riga finché non c'è un'animazione |

## Menu

- **File:** nuova scena vuota, scene demo, apri, progetti recenti, salva, salva con nome, salva
  il pacchetto completo (`.frpack`), una cartella o uno ZIP con tutti i file, importa modello,
  esporta immagine, esporta passaggi di render, viste e turntable, esci.
- **Modifica:** annulla, ripeti, duplica, elimina, nascondi, mostra tutto, **Impostazioni…**,
  ripristina impostazioni predefinite.
- **Scena:** prepara scena prodotto, adatta la scena al prodotto, annulla preset, aspetto,
  appoggia sulla base, inquadra il prodotto, ambiente, effetti, aggiungi una luce.
- **Oggetti:** posizione e dimensioni, materiali, luci.
- **Vista:** inquadra tutto, inquadra selezione, prospettica / ortografica, isometrica, viste
  standard, Viewport o Render, camera e obiettivo.
- **Rendering:** avvia / pausa / riprendi (F5), stop, riavvia, dispositivo di calcolo, qualità,
  formato, riavvio automatico, impostazioni di render e denoiser, viste e turntable, esporta.
- **Libreria:** la mia libreria, scarica da ambientCG / Poly Haven, importa materiale da Blender,
  cartella della libreria.
- **Strumenti:** interruttore AI MCP e **Integrazione AI…** (vedi [Controllo con l'IA](ai.mcp)).
- **Guida:** manuale (F1), scorciatoie da tastiera, benvenuto e informazioni.

## Pannelli

I pannelli si allargano trascinandone il bordo. Le righe lunghe scorrono dentro il pannello invece
di allargarlo.

**Pannello Scena.** Un clic su un oggetto lo seleziona (Ctrl o Maiusc lo aggiungono alla
selezione), la casella accanto al nome lo nasconde o lo mostra, il tasto destro ha altri comandi.
Sotto gli oggetti i gruppi **Luci**, **Camere** ed **Effetti** si aprono e si chiudono con la
freccia; il **+** a destra aggiunge una luce, una camera o un effetto. Ciò che è selezionato è
evidenziato in arancione.

**Pannello delle impostazioni (modalità Avanzata).** Ha tre schede:

- **Selezione** mostra ciò che hai scelto nella viewport o nell'albero, senza cercare la scheda
  giusta: per un oggetto posizione e misure e, sotto, il suo materiale; per una luce, una camera
  o un effetto le sue proprietà.
- **Scena** raccoglie ciò che vale per tutta la scena, in sotto-schede: **Luci** (preset e luci
  nuove), **Ambiente**, **Effetti**, **Camera** e **Materiali** (tutti i materiali della scena,
  anche senza niente di selezionato).
- **Render** contiene le impostazioni del calcolo. I valori da tecnico (roulette russa, limite
  della luce indiretta, seme casuale, thread della CPU) sono nella sezione **Avanzate**, chiusa.

**Parte bassa.** Senza animazioni la timeline resta ridotta a una riga: un clic su **Timeline** o
**Console** la apre, **Riduci** la richiude. Si apre da sola quando la timeline ha qualcosa.

## Barra di stato

Mostra l'avanzamento del render (campioni fatti sull'obiettivo, tempo trascorso, tempo
rimanente), il calcolo degli effetti come nebbia e fumo, e a destra il **backend** (il
processore è nel suggerimento). I contatori si scelgono in **Modifica → Impostazioni… → Barra di stato**: fino a 9
su 14 (FPS della viewport, fotogrammi, tempo rimanente, GPU, VRAM, RAM e altri).
