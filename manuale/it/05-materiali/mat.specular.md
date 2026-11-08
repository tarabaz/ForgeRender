---
id: mat.specular
titolo: Specular (forza del riflesso)
parole: speculare, forza del riflesso, riflesso, lucentezza, fresnel, brillantezza
ui: panel.materials.specular
correlate: mat.ior, mat.roughness, mat.metallic
---
# Specular (forza del riflesso)

**Dove si trova:** modalità Avanzata → scheda **Materiali** → cursore **Specular**.

**A cosa serve:** regola quanto è forte il riflesso di una superficie **non metallica**. A 0,5 il
riflesso è quello fisico che deriva dall'[IOR](mat.ior); valori più bassi lo indeboliscono,
valori più alti lo rinforzano. Sui metalli (Metallic 1) non ha effetto.

![Specular 0: nessun riflesso, un nero spento](img/comune/mat_specular_000.png) ![Specular 0,5: il riflesso fisico di una plastica](img/comune/mat_specular_050.png) ![Specular 1: riflesso più forte, effetto laccato](img/comune/mat_specular_100.png)

*Plastica nera, roughness 0,15, Specular 0, 0,5 e 1.*

- **Intervallo:** da 0 a 1.
- **Predefinito:** 0,5 (riflesso fisico derivato dall'IOR).

## Quando cambiarlo

- **Lascialo a 0,5** per quasi tutti i materiali: l'IOR dà già il riflesso giusto.
- **Abbassalo** per superfici che riflettono meno di una plastica: gesso, tessuto, terracotta non
  smaltata.
- **Alzalo** per un effetto laccato profondo o nero pianoforte.

Sui materiali scuri la differenza si vede di più, perché lì il riflesso è quasi tutto quello che
vedi.
