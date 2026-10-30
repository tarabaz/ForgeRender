---
id: mat.roughness
titolo: Roughness (rugosità)
parole: rugosità, lucido, opaco, satinato, riflesso, superficie
ui: panel.materials.roughness
correlate: mat.ior
---
# Roughness (rugosità)

**Dove si trova:** modalità Avanzata → scheda **Materiali** → cursore **Roughness**.

**A cosa serve:** dice quanto è liscia o ruvida la superficie, a livello microscopico. Una
superficie liscia riflette un'immagine nitida, come uno specchio; una ruvida sparge la luce e
il riflesso diventa morbido, fino a sparire.

| Valore | Aspetto | Esempi |
|---|---|---|
| 0,0 | Specchio, riflesso nitido | cromo, vetro, vernice lucida |
| 0,2 – 0,4 | Lucido ma morbido | plastica lucida, metallo spazzolato fine |
| 0,5 | Satinato (valore predefinito) | plastica normale, legno verniciato |
| 0,8 – 1,0 | Opaco | gomma, cartone, intonaco |

- **Intervallo:** da 0 a 1.
- **Predefinito:** 0,5.
- **Effetto sul render:** cambia solo l'aspetto dei riflessi, non il colore. Funziona insieme a
  **Metallic**: un metallo con roughness 0 è uno specchio, con roughness 0,6 è un metallo
  sabbiato.

## Con una texture

Se nello slot **Roughness** delle texture c'è una mappa, il cursore viene messo a 1 e
**moltiplica** la mappa: a 1 vale la mappa così com'è, valori più bassi rendono tutto più lucido.
Alcuni siti forniscono una mappa di *lucidità* (gloss) invece che di rugosità: in quel caso
attiva **È una mappa di lucidità (gloss)**, che la inverte.

## Errori comuni

- **Tutto sembra di plastica:** succede con roughness alta (0,7 e oltre) su oggetti che
  dovrebbero brillare. Abbassala.
- **Il vetro sembra smerigliato:** il vetro trasparente vuole roughness vicina a 0; valori più
  alti danno il vetro satinato, che è voluto solo per quell'effetto.

Vedi anche [IOR](mat.ior).
