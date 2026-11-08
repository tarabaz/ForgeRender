---
id: env.ambiente
titolo: Ambiente (uniforme, cielo a gradiente, HDRI)
parole: ambiente, sfondo, cielo, hdri, hdr, exr, gradiente, uniforme, intensità, rotazione, illuminazione ambientale, ibl
ui: panel.environment
correlate: light.tipi, render.esposizione, mat.metallic
---
# Ambiente (uniforme, cielo a gradiente, HDRI)

**Dove si trova:** modalità Avanzata → **Scena → Ambiente**; **Scena → Ambiente e HDRI…**.

**A cosa serve:** l'ambiente è la luce che arriva da tutto attorno alla scena, come dal cielo o
dalle pareti di una stanza. Illumina gli oggetti, si riflette sulle superfici lucide e, se vuoi,
fa da sfondo all'immagine.

| Tipo | Controlli | Uso |
|---|---|---|
| Uniforme | Colore, Intensità (0 – 10) | La stessa luce da ogni direzione: neutra, piatta |
| Cielo a gradiente | Colori di Zenit, Orizzonte, Suolo, Intensità (0 – 10) | Più chiaro sopra, più scuro sotto: naturale e morbido. È il predefinito (intensità 0,45) |
| Mappa HDRI | File (.hdr o .exr), Intensità (0 – 20), Rotazione (−180° – 180°) | Un ambiente vero fotografato: la luce e i riflessi più realistici |

I cursori dell'intensità sono logaritmici: la prima parte del cursore ha passi più fini.

![Gradiente con intensità 0,1: comandano le luci, ombre profonde](img/comune/env_strength_010.png) ![Intensità 0,45 (predefinita): equilibrata](img/comune/env_strength_045.png) ![Intensità 1,5: luminosa e piatta, poche ombre](img/comune/env_strength_150.png)

*Cielo a gradiente con intensità 0,1, 0,45 e 1,5, stesse luci da studio.*

## HDRI

- **HDRI della libreria:** gli HDRI scaricati da Poly Haven o salvati nella tua libreria sono
  elencati con l'anteprima; **Apri cartella** mostra dove sono.
- **Rotazione** gira l'ambiente attorno all'asse verticale: usala per spostare i riflessi sul
  prodotto o la direzione del sole dell'HDRI.
- Il renderer campiona apposta le zone luminose dell'HDRI (importance sampling): sole e finestre
  danno ombre pulite senza grana in più.
- Per gli interni illuminati da una finestra aggiungi una luce **Portale** sulla finestra: vedi
  [Tipi di luce](light.tipi).

## Ambiente visibile come sfondo

Spento, lo sfondo è nero (o il colore pieno dello [stile del render](render.stili)) ma l'ambiente
**continua a illuminare** la scena e a comparire nei riflessi. Utile per usare un HDRI solo per
la luce.

## Consigli

- Metalli lucidi e vetri hanno bisogno di qualcosa da riflettere: un HDRI li fa vivere molto più
  di un colore uniforme.
- Troppo ambiente appiattisce l'immagine (niente ombre); troppo poco lascia le ombre nere.
