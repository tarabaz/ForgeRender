---
id: mat.transmission
titolo: Transmission (vetri e liquidi)
parole: trasmissione, trasparente, trasparenza, vetro, liquido, acqua, dispersione, abbe, colore in volume, priorità
ui: panel.materials.transmission
correlate: mat.ior, render.caustiche, mat.roughness
---
# Transmission (vetri e liquidi)

**Dove si trova:** modalità Avanzata → scheda **Selezione** (materiale del pezzo scelto) o **Scena → Materiali** → cursore **Trasparenza (transmission)**.

**A cosa serve:** rende il materiale trasparente come un vetro o un liquido: la luce lo attraversa
e si piega secondo l'[IOR](mat.ior). A 0 il materiale è opaco, a 1 è del tutto trasparente.

![Transmission 0: plastica azzurra opaca](img/comune/mat_transmission_000.png) ![Transmission 0,8: quasi vetro, ancora lattiginoso](img/comune/mat_transmission_080.png) ![Transmission 1: vetro colorato trasparente](img/comune/mat_transmission_100.png)

*Materiale azzurro, roughness 0,05, IOR 1,5, Transmission 0, 0,8 e 1.*

- **Intervallo:** da 0 a 1.
- **Predefinito:** 0.
- **I valori intermedi** mescolano una superficie opaca e un vetro: sembrano lattiginosi. Per un
  vetro vero usa 1 e regola l'aspetto con la roughness (vetro satinato) e con i controlli del
  colore qui sotto.

## Controlli che compaiono sopra 0

| Controllo | Valori | Cosa fa |
|---|---|---|
| Dispersione | Nessuna, Vetro crown (BK7), Vetro flint, Cristallo al piombo, Policarbonato, Diamante, oppure il **numero di Abbe** (0 – 200) | Apre la luce bianca nei colori (arcobaleni nei gioielli e nei prismi). Abbe più basso = più forte. Si vede solo con il render **Spettrale**: vedi [Caustiche e spettrale](render.caustiche) |
| Colore in volume | colore | Il colore che prende la luce dopo **Profondità colore** di percorso dentro l'oggetto. Per i liquidi usa questo e lascia bianco il colore base: più scuro dove è profondo, più chiaro sui bordi, come nella realtà |
| Profondità colore | da 0,01 cm (predefinito 10 cm) | La distanza a cui si arriva al colore in volume |
| Priorità volume | −100 – 100 | Liquido in un bicchiere: modella il liquido un filo più grande della cavità e dagli priorità più alta del vetro: il motore usa allora l'interfaccia vetro-liquido giusta |

## Consigli

- **Vetro trasparente:** Transmission 1, roughness 0 – 0,02, IOR 1,5, colore base bianco.
- **Vetro satinato:** lo stesso con roughness 0,2 – 0,5.
- **Vetro colorato:** meglio il colore in volume del colore base: le parti spesse diventano più
  scure.
- Il vetro vuole abbastanza [rimbalzi](render.rimbalzi): con troppo pochi diventa nero.
