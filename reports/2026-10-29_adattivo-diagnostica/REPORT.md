# Campionamento adattivo: diagnostica (passo 0)

- **Data**: 29/10/2026
- **Commit**: `420a510` (diagnostica); l'algoritmo è quello di `7eccf8e`, invariato
- **Macchina**: container di sviluppo, CPU 3 thread, GPU software lavapipe
  (vedi [indice](../README.md#macchine))
- **Richiesta**: passo 0 approvato da ChatGPT in `reports/chatgpt/CLAUDE_HANDOFF.md`, cioè
  misurare prima di cambiare il criterio

## Domande

1. **Metà B**: la quota di campioni che finisce nella metà "B" (n_B/n) è davvero circa ½ su CPU
   e su GPU?
2. **Distribuzione dei campioni**: dove finiscono i campioni sulla scena del cocktail? È vero
   che le zone scure tengono attivi i riquadri? Era l'ipotesi del
   [report precedente](../2026-10-29_campionamento-adattivo/REPORT.md).
3. **Metodo**: la misura dell'errore per zona su PNG è affidabile?

## Strumenti aggiunti (commit `420a510`, nessun cambiamento al rendering)

- `Accumulator::b_fraction()`: quota n_B/n per pixel, con media (non pesata), minimo e massimo
  sui pixel con campioni. I pixel fermi conservano il valore del momento in cui si sono fermati.
- **Log**: alla prima valutazione della maschera, una riga con n_B/n e la frazione di riquadri
  attivi.
- **CLI**:
  - `--adaptive-map F.png` scrive un PNG a 16 bit con i campioni esatti di ogni pixel;
  - in fondo al render stampa una riga con n_B/n.
- **Script**: `../strumenti/analisi_adattivo.py` calcola, per zone scure (< 0.1), medie e
  chiare (≥ 0.5) della luminanza del riferimento PNG:
  - quota di pixel e quota di campioni;
  - MSE corretto per il rumore del riferimento;
  - differenza media con il suo errore standard;
  - un CSV per riquadro 16×16.
- **Test**: tre test unitari in `forge_render_core::accum`:
  - passate da 1 campione → ½;
  - passate 1, 2, 4, 8, 16 → 21/31;
  - pixel fermo → valore congelato.

**Controllo di non regressione** (eseguito): i render adattivi del cocktail rifatti con la
diagnostica (seed 7, soglie 0.04 e 0.08) sono **identici pixel per pixel** a `ad04.png` e
`ad08.png` del report precedente.

## Comandi

```
python3 reports/strumenti/crea_progetti_bench.py
forgerender-cli render --project samples/scene/cocktail-ghiaccio-secco/bench_ad.frproject \
    --backend cpu --width 320 --height 400 --threads 3 --quiet --spp 1024 --seed 7 \
    --noise-threshold 0.04 --adaptive-map mappa_ad04.png --out ad04.png      # e 0.08
python3 reports/strumenti/analisi_adattivo.py \
    --rif ../2026-10-29_campionamento-adattivo/ref.png ../2026-10-29_campionamento-adattivo/ref2.png \
    --mappa mappa_ad04.png --img ad04.png --img uni128.png --img uni32.png --csv riquadri_ad04.csv
forgerender-cli render --test-scene --backend cpu|gpu --width W --height H --spp 1024 \
    --noise-threshold 0.04 --seed S --quiet --out x.png
```

## Risultati

### 1. Quota della metà B (misurato)

**Cocktail su CPU, 320×400**:
- alla prima valutazione n_B/n = 0.500;
- alla fine, media 0.502–0.503, massimo 0.529 (pixel fermi a un numero dispari di campioni).

**Scena di test con la stessa soglia 0.04** (`log_cpu_piccola.txt`, `log_gpu_lavapipe.txt`):

| Risoluzione | Backend | Campioni per passata | n_B/n alla 1ª valutazione | n_B/n finale | Campioni per pixel all'arresto (seed 7 / 8 / 9) |
|---|---|---:|---:|---:|---|
| 192×108 | GPU lavapipe | fino a 5 | 0.500 | 0.503 | 246.5 (solo seed 7) |
| 64×36 | CPU | 1 | 0.500 | 0.500–0.502 | 279.8 / 300.0 / 294.2 |
| 64×36 | GPU lavapipe | fino a 15–16 | 0.667–0.677 | 0.512–0.527 | 306.0 / 230.0 / 278.2 |
| 32×18 | CPU | 1 | 0.500 | 0.502 | 235 / 209 / 217 |
| 32×18 | GPU lavapipe | fino a 64 | 0.677 | 0.661–0.669 | 124 / 127 / 125 |

- Quando la GPU raddoppia i campioni per passata, B riceve circa 2/3 dei campioni: 21/31 alla
  prima valutazione, come previsto dal codice.
- A 32×18 lo sbilanciamento resta fino alla fine e l'arresto arriva con **circa il 43 % di
  campioni in meno** della CPU (125 contro 220 di media su 3 seed), alla stessa soglia.
- A 64×36 l'effetto è più piccolo e si confonde con la variabilità tra seed.
- Sulla GPU software le passate sono piccole già a risoluzioni modeste. Sulla RTX 5070 le
  passate saranno probabilmente da 64 campioni anche a piena risoluzione, ma **è da misurare**:
  serve la riga n_B/n del CLI sul PC di Valerio.

### 2. Dove finiscono i campioni (cocktail, CPU)

Fasce di luminanza del riferimento (`analisi.txt`):

| Zona | Pixel | Soglia 0.04: campioni | spp medi | al massimo (1024) | Soglia 0.08: campioni | spp medi | al massimo |
|---|---:|---:|---:|---:|---:|---:|---:|
| scura (< 0.1) | 60.9 % | 43.6 % | 445 | 25.2 % | 32.4 % | 180 | 7.6 % |
| media | 23.2 % | 31.1 % | 833 | 58.1 % | 34.3 % | 499 | 21.2 % |
| chiara (≥ 0.5) | 15.8 % | 25.2 % | 989 | 94.5 % | 33.2 % | 709 | 22.9 % |

Con soglia 0.04 il 43.8 % dei pixel arriva a 1024 campioni e spende il 72.2 % di tutti i campioni.

**Immagine** `riferimento_mappa04_mappa08.png`: a sinistra il riferimento, poi le mappe a
soglia 0.04 e 0.08 (scala logaritmica da 16, viola, a 1024, giallo). Arrivano al massimo:
- bicchiere e nebbia;
- bancone illuminato con il tappeto di nebbia;
- portacandele a destra.

Lo sfondo scuro con bottiglie e lucine sfocate riceve pochi campioni.

### 3. Errore per zona e limite del metodo su PNG

MSE corretto per il rumore del riferimento:

| | scura | media | chiara | totale |
|---|---:|---:|---:|---:|
| Uniforme 32 | 0.00149 | 0.01956 | 0.01444 | 0.00774 |
| Uniforme 128 | 0.00087 | 0.00658 | 0.00295 | 0.00253 |
| Adattivo 0.08 | 0.00095 | 0.00345 | 0.00154 | 0.00163 |
| Adattivo 0.04 | 0.00042 | 0.00139 | 0.00039 | 0.00064 |

**Differenze medie nelle zone media e chiara.**
- Gli uniformi a pochi campioni sono più scuri del riferimento: uniforme 32 −0.028 (z ≈ −33);
  uniforme 128 −0.007/−0.010 (z ≈ −17/−20).
- Gli adattivi lo sono meno: −0.001/−0.006.

**Non è distorsione del rendering.** Il confronto è fatto **dopo il tone mapping**, che è una
curva concava. Per la disuguaglianza di Jensen un'immagine più rumorosa ha una media mostrata più
bassa anche se la radianza lineare è corretta. La differenza cresce con il rumore, come si vede.

**Zona scura.**
- Il 61 % dei pixel scuri del riferimento vale meno di 3/255: lì l'arrotondamento a 8 bit pesa
  quanto le differenze misurate.
- L'adattivo 0.04 ha +0.00077 (z ≈ 9.7), cioè circa 1/5 di livello del PNG.
- Con questi dati **non si può dire** se sia un effetto della regola di arresto o della
  quantizzazione.

## Conclusioni

1. **Ipotesi "le zone scure tengono attivi i riquadri": smentita** su questa scena. I pixel
   scuri ricevono meno campioni (445 contro 833 e 989 per pixel). I campioni vanno nelle zone
   chiare e medie: bicchiere con la nebbia, bancone, riflessi. Quasi tutte arrivano al massimo
   con soglia 0.04 (94.5 % dei pixel chiari).
2. **Ipotesi "B sbilanciata sulla GPU": confermata** sulla GPU software (n_B/n fino a 0.68) con
   arresto anticipato a parità di soglia. Sulla RTX 5070 è da misurare.
3. **Metodo**: le metriche su PNG vanno bene per l'MSE complessivo, ma **non** per cercare
   distorsioni locali piccole. Il tone mapping (Jensen) e gli 8 bit (zone scure) producono
   differenze sistematiche che non vengono dal renderer. Serve l'uscita lineare in virgola
   mobile.

## Non eseguito

- Misure sulla RTX 5070.
- Repliche con più seed sul cocktail: un solo seed per soglia, come nel report precedente.
- Curve MSE–tempo a parità di qualità.
- Confronto in radianza lineare: il CLI oggi salva solo PNG.

## Prossimi passi proposti (da approvare)

- **0b. Strumento, non algoritmo**: uscita lineare del CLI (`--out-exr F.exr`) e lettura EXR
  in `analisi_adattivo.py`. Così la differenza media per zona si misura in radianza, dove lo
  stimatore deve essere non distorto, senza Jensen né 8 bit.
- **0c.** Valerio lancia sulla RTX 5070 il render adattivo del cocktail con `--adaptive-map` e
  riporta la riga n_B/n e la riga del log "prima valutazione".
- **1. Primo cambiamento dell'algoritmo**, se 0c conferma lo sbilanciamento: B bilanciata per
  pixel.
  - Una passata va in B se `count_b·2 < count`, invece che per parità.
  - Solo `accum.rs`; sulla CPU il comportamento è identico per costruzione.
  - Corregge un difetto misurato e rende confrontabili CPU e GPU prima di toccare il criterio.
- **Dopo**: la scelta del criterio (spazio mostrato, vicinato) va riformulata alla luce del
  punto 1 delle conclusioni.
  - Il problema del cocktail non sono le zone scure, ma zone chiare con rumore vero (nebbia
    illuminata, riflessi) che con la stima attuale non scendono mai sotto 0.04.
  - Un criterio nello spazio mostrato le comprimerebbe (tone mapping sulle luci). Va valutato
    come compromesso di qualità, non solo come velocità.
