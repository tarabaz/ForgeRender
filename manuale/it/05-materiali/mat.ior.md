---
id: mat.ior
titolo: IOR (indice di rifrazione)
parole: rifrazione, indice, vetro, acqua, diamante, trasparenza, riflesso
ui: panel.materials.ior
correlate: mat.transmission, mat.roughness, mat.specular
---
# IOR (indice di rifrazione)

**Dove si trova:** modalità Avanzata → scheda **Selezione** (materiale del pezzo scelto) o **Scena → Materiali** → cursore **Indice di rifrazione (IOR)**.

**A cosa serve:** dice quanto la luce si piega entrando nel materiale e quanto il materiale
riflette guardato di fronte. Conta soprattutto per i materiali trasparenti (**Transmission**
sopra 0) e per i riflessi delle superfici non metalliche.

![IOR 1,1: quasi nessuna deviazione, riflessi deboli](img/comune/mat_ior_110.png) ![IOR 1,5: vetro comune](img/comune/mat_ior_150.png) ![IOR 2,4: diamante, forte deviazione e riflessi](img/comune/mat_ior_240.png)

*Vetro trasparente (Transmission 1, roughness 0) con IOR 1,1, 1,5 e 2,4: guarda quanto si
deforma il nucleo nero dentro e quanto riflettono i bordi.*

| Materiale | IOR tipico |
|---|---|
| Aria | 1,0 |
| Acqua | 1,33 |
| Plastica, vetro comune | 1,45 – 1,5 |
| Vetro al piombo, cristallo | 1,6 – 1,7 |
| Diamante | 2,42 |

- **Intervallo:** da 1 a 3.
- **Predefinito:** 1,5 (plastica e vetro comune).
- **Effetto sul render:** con IOR alto gli oggetti trasparenti deformano di più quello che c'è
  dietro e i bordi riflettono di più. Con **Specular** a 0,5 (predefinito) anche il riflesso delle
  superfici opache segue l'IOR.

## Quando cambiarlo

- **Liquidi:** 1,33 per l'acqua, circa 1,36 per i superalcolici.
- **Gioielli e pietre:** valori alti (diamante 2,42) danno i tipici riflessi scintillanti.
- **Metalli:** l'IOR conta poco; il loro aspetto dipende da **Metallic** e
  [Roughness](mat.roughness).

## Errori comuni

- **Vetro che sembra uno specchio deformante:** IOR troppo alto per un vetro comune. Torna a 1,5.
- **IOR 1,0 su un oggetto trasparente:** l'oggetto diventa quasi invisibile, perché non devia né
  riflette la luce.
