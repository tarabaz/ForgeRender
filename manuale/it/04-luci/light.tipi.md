---
id: light.tipi
titolo: Pannello Luci e tipi di luce
parole: luce, luci, lampada, area, softbox, puntiforme, lampadina, spot, faretto, direzionale, sole, portale, laser, intensità, preset, raggio d'azione, ombre, fascio
ui: panel.lights, scene.add_light
correlate: light.dimensione, light.colore, env.ambiente, fx.foschia
---
# Pannello Luci e tipi di luce

**Dove si trova:** modalità Avanzata → **Scena → Luci** (preset e luci nuove); la luce scelta compare nella scheda **Selezione**. Le luci sono elencate nel pannello Scena a
sinistra: clicca una luce per modificarla.

## Preset

**Preset:** aggiunge un gruppo di luci pronto, adattato alle dimensioni della scena, **accanto a
quelle che ci sono già** (non ne toglie nessuna):

| Preset | Luci |
|---|---|
| Studio neutro | Softbox principale e riempimento |
| Studio fotografico (4 luci) | Softbox principale, riempimento, luce dall'alto e luce del fondale |
| Tre punti luce | Key, fill e rim |
| Softbox | Un grande softbox dall'alto |
| Illuminazione industriale | Tre plafoniere |
| Cinematografica | Key calda e controluce fredda |

## Tipi di luce

| Tipo | Intensità in | Uso |
|---|---|---|
| Area (rettangolare) | Radianza (W/sr/m²) | Un softbox: ombre morbide, la luce della foto di prodotto |
| Puntiforme | Intensità (W/sr) | Una lampadina piccola: ombre nette |
| Spot | Intensità (W/sr) | Un cono di luce; può mostrare il fascio visibile nell'aria |
| Direzionale | Irradianza (W/m²) | Il sole: raggi paralleli, stessa forza ovunque |
| Portale (finestra) | — | Non emette: dice al renderer da dove entra la luce dell'HDRI attraverso una finestra, molta meno grana negli interni |
| Laser | Potenza (W) | Laser da laboratorio rosso, verde, blu, viola o bianco con fascio visibile, rifratto da vetri e prismi |

## Controlli comuni

| Controllo | Valori | Cosa fa |
|---|---|---|
| Accesa | sì / no | Spegne la luce senza eliminarla |
| Punta su | oggetto | La luce segue un oggetto (**Aggancia alla selezione**, **Sgancia**) |
| Colore e Temperatura | 2700 K … 9000 K | Vedi [Colore della luce](light.colore) |
| Intensità | da 0 in su | Luminosità, nell'unità del tipo |
| Larghezza / Altezza (m) | area e portale | Misure del rettangolo: vedi [Dimensione e ombre morbide](light.dimensione) |
| Apertura cono (°) | 1 – 170 | Apertura dello spot |
| Sfumatura bordo (%) | 0 – 100 | 0 = bordo netto, 100 = sfuma dal centro |
| Fascio visibile | spot | Fascio volumetrico con **intensità del fascio** (0,02 – 5) e **lunghezza del fascio** |
| Raggio d'azione | Limitato: distanza massima (m) e sfumatura del raggio (%) | Puntiforme e spot: la luce si ferma a una distanza, disegnata come una bolla nella viewport |
| Posizione / Bersaglio | metri | Dove sta la luce e dove punta; **Punta al centro della scena** |
| Proietta ombre | sì / no | Spento: la luce attraversa gli oggetti |
| Visibile alla camera | sì / no | Se la luce stessa compare nell'immagine |
| Duplica / Elimina | | |

## Puntare nella viewport

Seleziona uno spot o una luce direzionale: una linea va al suo bersaglio, disegnato come un
**rombo**. Trascina il rombo per puntare la luce mentre la sorgente resta ferma; i campi seguono
e lo spostamento si può annullare.
