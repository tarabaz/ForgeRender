---
id: anim.video
titolo: Salvare l'animazione e impostazioni video
parole: salva animazione, video, mp4, mov, mkv, avi, h264, h265, prores, ffv1, codec, crf, bitrate, ffmpeg, fotogrammi, esporta video, esportazione in background, riprendi
ui: anim.save, anim.video_settings
correlate: anim.timeline, export.viste, render.campioni
---
# Salvare l'animazione e impostazioni video

**Dove si trova:** Timeline → **Salva animazione**.

| Scelta | Cosa salva |
|---|---|
| Render di ogni fotogramma | Il render fotorealistico di ogni fotogramma dell'area di lavoro, con i campioni attuali |
| Anteprima (viewport) | Una registrazione veloce della viewport, per controllare il movimento (fumi e nebbie non ci sono) |
| Vista | La viewport o una camera di rendering |
| Anche il video | Oltre ai fotogrammi PNG, un file video |

**Salva** scrive i fotogrammi in una cartella nuova dentro **Rendering** per ogni animazione.

## Impostazioni video

L'ingranaggio (**Impostazioni video…**) apre contenitore, codec, compressione (qualità costante
**CRF** o **bitrate**), formato dei pixel, fotogrammi al secondo, risoluzione e file di
destinazione.

| Preset | Risultato |
|---|---|
| Standard | MP4: file piccoli, compressione alta, per condividere il video |
| Alta qualità | MP4 con compressione leggera: file circa tre volte più grandi |
| Molto alta | MP4 con compressione minima: quasi nessun difetto visibile |
| Per il montaggio | ProRes in MOV: file grandi, fluidi da montare |
| Senza perdita | FFV1 in MKV: ogni pixel identico ai fotogrammi, per l'archivio |
| Non compresso | AVI: file enormi, lo legge qualsiasi programma di montaggio |

MP4 H.264 e AVI non compresso funzionano subito; gli altri formati vogliono **ffmpeg** accanto al
programma o nel PATH, e senza sono disattivati. Il file video compare solo a lavoro finito, e
l'avviso finale riporta i parametri veri.

## Esportazioni lunghe

- **Massima velocità** usa tutta la potenza; **Lascia il PC usabile** dà al render priorità bassa
  e pause sulla GPU, così gli altri programmi restano fluidi (il render dura di più).
- L'esportazione **prosegue con la finestra ridotta a icona** e intanto il PC non va in
  sospensione.
- Un'esportazione interrotta **riprende dai fotogrammi già fatti**. Chiudere durante
  un'esportazione chiede prima.
- La barra di stato conta i fotogrammi e stima il tempo che manca per il render e poi per la
  codifica.
