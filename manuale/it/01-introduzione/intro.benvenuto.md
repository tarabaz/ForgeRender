---
id: intro.benvenuto
titolo: Benvenuto in ForgeRender
parole: introduzione, inizio, guida, manuale, aiuto, indice
ui: help.manual, help.welcome
correlate: intro.primo-render, intro.semplice, keys.elenco
---
# Benvenuto in ForgeRender

ForgeRender crea **foto di prodotto** realistiche partendo da un modello 3D: importi il modello,
scegli fondale e luci, assegni i materiali e avvii il render. Il motore è un *path tracer*:
simula il percorso della luce, rimbalzo dopo rimbalzo, proprio come in uno studio fotografico
vero. Lavora sulla scheda video (GPU) o sul processore (CPU).

## Come usare questo manuale

- **Indice** a sinistra: i capitoli e, dentro ognuno, una pagina per ogni funzione.
- **Ricerca** in alto: scrivi una parola, anche in linguaggio comune ("opaco", "grana",
  "spostare"), e trovi la pagina giusta. Accenti e un errore di battitura non contano.
- **F1** sopra un controllo dell'interfaccia apre la sua pagina; se il controllo non ne ha una,
  si apre quella del pannello attivo.
- **Indietro / Avanti** scorrono le pagine già viste, **Home** riporta qui.
- **Immagini di confronto:** le pagine delle impostazioni principali mostrano la stessa scena
  con il cursore basso, medio e alto. Passa col mouse sopra un'immagine per vederla grande.

## Capitoli

| Capitolo | Cosa trovi |
|---|---|
| Introduzione | [Il primo render](intro.primo-render), [la modalità Semplice](intro.semplice) |
| Interfaccia | [La finestra](ui.finestra), [la viewport](ui.viewport), [le impostazioni](ui.impostazioni) |
| Scena e oggetti | [Importare](scene.importa), [spostare e ridimensionare](scene.oggetto), [progetti e pacchetti](scene.progetti) |
| Luci | [Tipi di luce](light.tipi), [dimensione e ombre](light.dimensione), [colore della luce](light.colore) |
| Materiali | [Il pannello Materiali](mat.pannello), [Roughness](mat.roughness), [Metallic](mat.metallic), [Specular](mat.specular), [IOR](mat.ior), [Transmission](mat.transmission), [Texture](mat.texture) |
| Ambiente | [Uniforme, cielo a gradiente, HDRI](env.ambiente) |
| Effetti | [Tutti gli effetti](fx.effetti), [foschia d'ambiente](fx.foschia) |
| Camera | [Obiettivo e focale](cam.obiettivo), [profondità di campo](cam.dof), [tilt-shift](cam.tiltshift) |
| Rendering | [Il pannello Render](render.pannello), [campioni](render.campioni), [denoiser](render.denoiser), [rimbalzi](render.rimbalzi), [esposizione](render.esposizione), [caustiche e spettrale](render.caustiche), [stili](render.stili) |
| Salvare le immagini | [Immagine e passaggi](export.immagine), [viste e turntable](export.viste) |
| Animazione | [Timeline](anim.timeline), [salvare l'animazione](anim.video) |
| Controllo con l'IA | [MCP](ai.mcp) |

## Semplice o Avanzata

Scegli la modalità in alto a destra:

- **Semplice**: un pannello guidato in 5 passi (prodotto, aspetto, materiali, formato e qualità,
  render). Le impostazioni tecniche sono già regolate per un buon risultato. Vedi
  [Modalità Semplice](intro.semplice).
- **Avanzata**: tutti i controlli, nelle schede Selezione (ciò che hai scelto), Scena (luci,
  ambiente, effetti, camera, materiali) e Render.

I valori sono gli stessi nelle due modalità: puoi preparare la scena in Semplice e rifinirla in
Avanzata.

> Le immagini di confronto le ha calcolate ForgeRender stesso sulla scheda video, sulla scena di
> prova del programma e sulla sfera delle anteprime dei materiali, a 640 × 480 pixel e con almeno
> 256 campioni (tranne i confronti sul numero di campioni e sulla soglia di rumore).
