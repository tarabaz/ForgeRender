---
id: render.campioni
titolo: Campioni per pixel e soglia di rumore
parole: campioni, spp, passate, qualità, rumoroso, campionamento adattivo, soglia di rumore, illimitato, tempo di render, convergenza
ui: panel.render.samples
correlate: render.denoiser, render.rimbalzi, render.pannello
---
# Campioni per pixel e soglia di rumore

**Dove si trova:** scheda Render → **Campioni per pixel**.

**A cosa serve:** ogni campione è un percorso di luce tracciato per ogni pixel. L'immagine è la
media dei campioni: con pochi è rumorosa (sgranata), con tanti diventa pulita. Il rumore si
dimezza ogni volta che i campioni si moltiplicano per quattro, quindi gli ultimi passi costano di
più.

![16 campioni: molto sgranata](img/comune/render_samples_016.png) ![64 campioni: grana ancora visibile](img/comune/render_samples_064.png) ![256 campioni: grana fine su vetro e ombre](img/comune/render_samples_256.png) ![1024 campioni: pulita](img/comune/render_samples_1024.png)

*Denoiser spento, per mostrare il rumore vero: 16, 64, 256 e 1024 campioni. Il vetro e le ombre
morbide sono gli ultimi a pulirsi.*

| Scelta | Valori |
|---|---|
| Preset | 16, 32, 64, 128, 256, 512, 1024 |
| Personalizzato | da 1 a 1.000.000 |
| Illimitato | il render continua finché non lo fermi |

- **Predefinito:** 128.
- **Si può cambiare durante il render:** l'accumulo prosegue senza ripartire.
- **Tempo:** cresce in proporzione ai campioni: 1024 campioni impiegano circa quattro volte 256.

## Quanti?

| Scena | Con il denoiser |
|---|---|
| Prodotto opaco in studio | 128 – 256 |
| Vetri, liquidi, metalli lucidi | 512 – 1024 |
| Fumo, nebbia, caustiche, interni scuri | 1024 e oltre |

Con il [denoiser](render.denoiser) acceso di solito bastano poche centinaia di campioni: toglie
lui la grana che resta.

## Arresto a soglia di rumore (campionamento adattivo)

**Arresto a soglia di rumore** fa fermare ogni zona dell'immagine quando il suo rumore stimato
scende sotto la soglia: il tempo va alle zone difficili (vetro, ombre, nebbia) invece che allo
sfondo. Il numero di campioni sopra resta il **massimo**.

![Soglia 0,08: veloce, ancora rumorosa](img/comune/render_noisethreshold_0080.png) ![Soglia 0,04: circa la qualità di 256 campioni](img/comune/render_noisethreshold_0040.png) ![Soglia 0,01: pulita, più lenta](img/comune/render_noisethreshold_0010.png)

*Massimo 1024 campioni, denoiser spento, soglie 0,08, 0,04 e 0,01.*

| Controllo | Intervallo | Predefinito | Cosa fa |
|---|---|---|---|
| Soglia di rumore | 0,002 – 0,1 | 0,03 quando la accendi | Più bassa = più pulita e più lenta. Sulla scena di prova 0,08 ≈ 64 campioni, 0,04 ≈ 256, 0,02 ≈ 1000 |
| Campioni minimi | 4 – 4096 | 16 | Campioni per pixel prima di misurare il rumore (evita di fermare troppo presto le zone con luce rara) |

Conviene con sfondi e zone facili; nelle scene scure con nebbia e scintille può costare più del
campionamento uniforme.
