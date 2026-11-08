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
| Barra dei menu | in alto | File, Modifica, Scena, Oggetti, Vista, Rendering, Libreria, Strumenti, Guida |
| Barra degli strumenti | sotto i menu | interruttore Semplice / Avanzata, i comandi più usati, menu **Aggiungi**, **Avvia render**, interruttore **AI MCP** |
| Pannello Scena | a sinistra | l'albero dei componenti del modello, poi camere di rendering, luci ed effetti |
| Area centrale | al centro | **Viewport** (vista in tempo reale) o **Render** (l'immagine del path tracer) |
| Pannello delle impostazioni | a destra | Semplice: i 5 passi. Avanzata: le schede Oggetto, Materiali, Luci, Ambiente, Effetti, Camera, Render |
| Timeline / Console | in basso | chiavi dell'animazione (vedi [Timeline](anim.timeline)) e messaggi del programma |
| Barra di stato | in fondo | campioni, tempo, tempo rimanente, effetti in calcolo, dispositivo |

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
di allargarlo. Nel pannello Scena un clic seleziona un componente (Ctrl o Maiusc lo aggiungono
alla selezione), la casella accanto al nome lo nasconde o lo mostra, il tasto destro ha altri
comandi.

## Barra di stato

Mostra l'avanzamento del render (campioni fatti sull'obiettivo, tempo trascorso, tempo
rimanente), il calcolo degli effetti come nebbia e fumo, e a destra il **backend** e il
processore. I contatori si scelgono in **Modifica → Impostazioni… → Barra di stato**: fino a 9
su 15 (FPS della viewport, fotogrammi, tempo rimanente, GPU, VRAM, RAM e altri).
