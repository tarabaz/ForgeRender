---
id: mat.ior
titolo: IOR (indice di rifrazione)
parole: rifrazione, indice, vetro, acqua, diamante, trasparenza, riflesso
ui: panel.materials.ior
correlate: mat.roughness
---
# IOR (indice di rifrazione)

**Dove si trova:** modalità Avanzata → scheda **Materiali** → cursore **IOR**.

**A cosa serve:** dice quanto la luce si piega quando entra nel materiale e quanto il materiale
riflette guardandolo di fronte. Conta soprattutto per i materiali trasparenti
(**Transmission** sopra 0) e per i riflessi delle superfici non metalliche.

| Materiale | IOR tipico |
|---|---|
| Aria | 1,0 |
| Acqua | 1,33 |
| Plastica, vetro comune | 1,45 – 1,5 |
| Vetro al piombo, cristallo | 1,6 – 1,7 |
| Diamante | 2,42 |

- **Intervallo:** da 1 a 3.
- **Predefinito:** 1,5 (plastica e vetro comune).
- **Effetto sul render:** con un IOR alto gli oggetti trasparenti deformano di più quello che sta
  dietro e i bordi riflettono di più. Con **Specular** a 0,5 (predefinito) anche il riflesso delle
  superfici opache segue l'IOR.

## Quando cambiarlo

- **Liquidi:** 1,33 per l'acqua, circa 1,36 per le bevande alcoliche.
- **Gioielli e pietre:** valori alti (diamante 2,42) danno i riflessi brillanti tipici.
- **Metalli:** l'IOR conta poco; il loro aspetto dipende da **Metallic** e
  [Roughness](mat.roughness).

## Errori comuni

- **Vetro che sembra uno specchio deformato:** IOR troppo alto per un vetro normale. Torna a 1,5.
- **IOR 1,0 su un oggetto trasparente:** l'oggetto diventa quasi invisibile, perché non piega la
  luce e non riflette.
