---
id: fx.foschia
titolo: Foschia d'ambiente
parole: foschia, nebbia, caligine, raggi di luce, fasci, luce volumetrica, densità, atmosfera
ui: panel.effects.haze
correlate: fx.effetti, light.tipi, render.campioni
---
# Foschia d'ambiente

**Dove si trova:** scheda Effetti → **Aggiungi effetto** → **Foschia**.

**A cosa serve:** riempie la scena (o una fascia) di una nebbia leggera che ogni luce attraversa:
spot, sole e lampade disegnano raggi visibili, con dentro le ombre degli oggetti.

## Densità

**Densità** (0,001 – 50, logaritmica) è quanto è spessa l'aria, in 1/m. Accanto al cursore il
pannello mostra la distanza a cui un oggetto sparirebbe nella foschia ("si vede fino a ~").

![Densità 0,3: fascio leggero, si vede fino a circa 10 m](img/comune/fx_haze_density_low.png) ![Densità 1: fascio netto, circa 3 m](img/comune/fx_haze_density_mid.png) ![Densità 3: fascio denso, circa 1 m](img/comune/fx_haze_density_high.png)

*Nebbia bassa illuminata da uno spot dall'alto, densità 0,3, 1 e 3.*

## Tipi

| Tipo | Aspetto |
|---|---|
| Nebbia bassa | Si stende a terra e si dirada salendo, con banchi e riccioli |
| Nuvole di gas | Aria quasi limpida con ammassi densi qua e là |
| Fumo tossico a terra | Uno strato di pochi centimetri che aleggia sul pavimento, raccolto in filamenti |
| Strati sospesi | Veli orizzontali a quote diverse, ondulati e bucati |
| Riccioli di vapore | Vortici dentro i vortici, con contrasto alto |

## Altri controlli

| Controllo | Cosa fa |
|---|---|
| Solo raggi delle luci | Acceso (predefinito): la foschia mostra solo la luce di spot, sole e lampade che la attraversa, senza velare né scurire gli oggetti. Spento: foschia fisica, che vela tutto con la luce dell'ambiente |
| Addensamenti (0 – 2) | 0 = uguale ovunque, 1 = ammassi marcati e vuoti, 2 = pochi ammassi molto concentrati |
| Altezza di partenza / Altezza massima | Una fascia sospesa, per esempio sotto un soffitto |
| Sbuffi | Qualche sbuffo casuale che sale oltre l'altezza massima |
| Colore | Bianco = aria neutra, caldo = polvere, freddo = nebbia |
| Raggi in controluce | Quanto sono luminosi i raggi guardando verso la luce |
| Grandezza dei vortici, Forza dei vortici, Dettaglio frattale, Ruvidità | La forma di banchi e riccioli |
| Qualità (1 – 4) | Da una griglia leggera da bozza a 24 milioni di celle per il dettaglio fine |
| Forma: Nuova | Un'altra disposizione dei vortici |

## Consigli

- I raggi si vedono meglio con uno **spot** o il **sole** e un ambiente basso.
- Una foschia densa vuole più campioni: i volumi sono la parte più rumorosa di un render.
