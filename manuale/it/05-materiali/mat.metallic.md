---
id: mat.metallic
titolo: Metallic (metallicità)
parole: metallo, metallico, metallicità, conduttore, acciaio, oro, rame, alluminio, cromo
ui: panel.materials.metallic
correlate: mat.roughness, mat.specular, mat.pannello
---
# Metallic (metallicità)

**Dove si trova:** modalità Avanzata → scheda **Selezione** (materiale del pezzo scelto) o **Scena → Materiali** → cursore **Metallicità (metallic)**.

**A cosa serve:** dice se la superficie è un metallo o no. Un non metallo (plastica, vernice,
legno) mostra il suo colore nella luce diffusa e ha riflessi bianchi e piuttosto deboli; un
metallo non ha colore diffuso: tutto quello che mostra è riflesso, **tinto dal colore base**.

![Metallic 0: plastica, corpo colorato e riflessi bianchi](img/comune/mat_metallic_000.png) ![Metallic 0,5: a metà, fisicamente insolito](img/comune/mat_metallic_050.png) ![Metallic 1: rame, riflessi colorati](img/comune/mat_metallic_100.png)

*Stesso colore base e roughness 0,2, Metallic 0, 0,5 e 1.*

- **Intervallo:** da 0 a 1.
- **Predefinito:** 0.
- **Effetto sul render:** a 1 il colore base diventa il colore dei riflessi (oro, rame,
  ottone); a 0 è il colore del corpo.

## Che valore usare

I materiali veri o sono metalli o non lo sono: usa **0 oppure 1**. I valori intermedi servono
solo con una **texture di metallicità** (vernice scrostata su un metallo, ruggine sull'acciaio),
dove ogni punto è l'uno o l'altro.

| Metallo | Colore base (indicativo) |
|---|---|
| Acciaio, cromo | grigio chiaro 0,56 – 0,58 |
| Alluminio | quasi bianco 0,91 |
| Oro | 1,0, 0,71, 0,29 |
| Rame | 0,95, 0,64, 0,54 |

## Errori comuni

- **Metallo nero:** un metallo con colore base molto scuro non riflette quasi niente. I metalli
  sono chiari: usa colori luminosi.
- **Metallo che sembra plastica:** Metallic lasciato a 0. Il cromo vuole Metallic 1 e
  [Roughness](mat.roughness) bassa.
- **Un metallo ha bisogno di qualcosa da riflettere:** davanti a uno sfondo uniforme sembra
  piatto. Un ambiente HDRI o qualche luce attorno gli danno vita.
