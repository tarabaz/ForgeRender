# Report delle misure e delle simulazioni

Questa cartella conserva **i dati grezzi e le conclusioni** di ogni misura fatta su ForgeRender,
in modo che chiunque (anche un altro agente) possa ricontrollarli, rifare i conti o proporre
migliorie. Solo misure realmente eseguite: niente numeri stimati.

## Come è fatto un report

Una sottocartella per esperimento, `AAAA-MM-GG_argomento/`, con:

- `REPORT.md`: domanda, macchina, commit, comandi esatti, risultati in tabella, controlli di
  qualità, conclusioni e limiti, prossimi passi proposti;
- `log_*.txt`: l'output originale dei comandi (tempi, campioni);
- le immagini prodotte (PNG), così le metriche si possono ricalcolare.

Gli strumenti per riprodurre le misure stanno in [`strumenti/`](strumenti/):

- `crea_progetti_bench.py`: crea i progetti di benchmark derivati dalla demo del cocktail;
- `confronta_immagini.py`: errore (MSE/RMSE) contro un riferimento, con il rumore del
  riferimento sottratto quando se ne danno due.
- `analisi_adattivo.py`: per zone scure/medie/chiare del riferimento, quota di campioni (dalla
  mappa `--adaptive-map`), errore e differenza media con errore standard; CSV per riquadro.

Servono Python 3 con `numpy` e `pillow` (`py -m pip install numpy pillow` su Windows).

## Indice

| Data | Report | Esito in breve |
|---|---|---|
| 2026-10-28 | [Esperimenti precedenti (riepilogo)](2026-10-28_esperimenti-precedenti/REPORT.md) | Portali 6.5× meno rumore; BVH parallela 3.3× su 4 core; bug della griglia della nebbia; benchmark 9900X + RTX 5070 |
| 2026-10-29 | [Majorant 3D della nebbia](2026-10-29_majorant-3d-nebbia/REPORT.md) | 1.70–1.98× più veloce sulla CPU, immagine invariata |
| 2026-10-29 | [Campionamento adattivo](2026-10-29_campionamento-adattivo/REPORT.md) | 1.8× su scena con cielo, **peggiore** sulla scena del cocktail: spento di default |
| 2026-10-29 | [Adattivo: diagnostica (passo 0)](2026-10-29_adattivo-diagnostica/REPORT.md) | Campioni nelle zone chiare (non scure); metà B sbilanciata sulla GPU (fino a 0.68, arresto anticipato); PNG non adatto a misurare distorsioni locali |

## Macchine

- **Container di sviluppo**: Intel Xeon @ 2.10–2.80 GHz, 4 vCPU (misure con 3 thread), 15.7 GB,
  Ubuntu 24.04; GPU solo software (lavapipe, limite 134 MB per buffer: le scene grandi non
  entrano). Utile per confronti prima/dopo sulla CPU, non per i tempi assoluti.
- **PC di riferimento**: AMD Ryzen 9 9900X (12 core, 24 thread), 93.6 GB, NVIDIA RTX 5070,
  Windows 11. I tempi che contano.
