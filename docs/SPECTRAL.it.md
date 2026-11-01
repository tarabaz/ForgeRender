# Rendering spettrale

🇬🇧 [Read in English](SPECTRAL.md)

ForgeRender ha due modi di trasportare la luce, da scegliere in **Render → Colore**:

| Modo | Cosa fa | Quando usarlo |
|---|---|---|
| **RGB** (predefinito) | Tre canali (rosso, verde, blu). Invariato, il più veloce. | Quasi sempre. |
| **Spettrale** | Quattro lunghezze d'onda per percorso (hero wavelength). Dispersione in vetri e gemme, colori più fedeli con luci sature. | Vetro, cristallo, gioielli, prismi; circa 2 volte più lento e con più rumore di colore, quindi servono più campioni. |

Lo spettrale è un'opzione: i progetti si aprono in RGB, a meno che non siano stati salvati in modalità spettrale. Funziona sia sul backend CPU sia su quello GPU, che danno lo stesso risultato.

## Dispersione

I materiali trasparenti hanno l'impostazione **Dispersione** (numero di Abbe V, 0 = nessuna). V più basso = colori più separati. Predefiniti: vetro crown BK7 (V 64,2, IOR 1,517), flint (36,4, 1,62), cristallo al piombo (30, 1,7), policarbonato (30, 1,586), diamante (55, 2,417). L'indice di rifrazione segue l'equazione di Cauchy n(λ) = A + B/λ², ricavata dall'IOR alla riga d (587,6 nm) e da V. La dispersione si vede solo in modalità spettrale.

## Come funziona

- **Lunghezze d'onda**: quattro per percorso tra 360 e 830 nm, con la distribuzione "visibile" di PBRT v4 (pdf ∝ 1/cosh²(0,0072 (λ − 538))), stratificate a passi di ¼ (Wilkie et al. 2014, *Hero Wavelength Spectral Sampling*).
- **Le direzioni non dipendono dalla lunghezza d'onda** (le quattro condividono pdf di campionamento e pesi MIS), tranne nei dielettrici con dispersione. Lì si usa l'IOR della hero e le altre tre si fermano, con la pdf della hero divisa per 4 (`TerminateSecondary` di PBRT v4). La stima resta non distorta.
- **RGB → spettro**: i colori della scena restano RGB fino al punto in cui servono, poi diventano spettri lisci s(c₀t² + c₁t + c₂) con s(x) = ½ + x/(2√(1+x²)) (Jakob e Hanika 2019, come in PBRT v4). La tabella dei coefficienti 3 × 64³ si calcola una volta per processo con un nostro fit di Gauss-Newton in CIELAB, in circa 2 s su un pool di thread dedicato; nessun file di dati esterno. Le riflettanze restano in [0, 1]; le emissioni usano la scala 2·max.
- **Colore**: funzioni colorimetriche CIE 1931 2° (fit analitico di Wyman, Sloan e Shirley 2013), da XYZ ai primari sRGB lineari con un adattamento di Bradford dal bianco a energia uguale. Uno spettro costante va esattamente in (1, 1, 1), quindi una luce bianca resta bianca come in modalità RGB. PBRT invece moltiplica le luci per il D65; qui i colori delle luci sono già espressi nello spazio RGB della scena.
- **Volumi**: Beer-Lambert per lunghezza d'onda, T(λ) = R(λ)^(d/d₀), con R lo spettro del colore del volume allo spessore d₀.
- **Accumulo**: ogni campione è convertito in RGB prima dell'accumulo. La conversione è lineare, quindi equivale ad accumulare XYZ, e accumulo, denoiser, tone mapping ed esportazione EXR non cambiano.
- **Codice**: `forge_render_core::spectral` (lunghezze d'onda, funzioni colorimetriche, tabella, Cauchy), `forge_render_cpu::spectral` (integratore; la BSDF è generica sui canali `Vec3`/`Vec4`, quindi la versione RGB è invariata), `forge_render_gpu/src/path_tracer_spectral.wgsl` (punto d'ingresso `main_spectral`, dati nel binding 4, caricati solo in modalità spettrale).

## Verifiche

Test automatici (`cargo test`):
- la pdf visibile integra a 1 e coincide con la costante di PBRT; il campionamento segue la CDF;
- giro RGB → spettro → RGB: errore massimo 0,0004, anche sui primari puri;
- fermare le lunghezze d'onda secondarie lascia in media bianco uno spettro bianco;
- white furnace (CPU e GPU): diffuso, vetro, vetro dispersivo e vetro satinato restano a 1;
- luce ad area sopra un piano diffuso: coincide con il fattore di forma analitico entro il 3% (MIS);
- lo spettrale coincide con l'RGB sulle scene colorate a un rimbalzo;
- la dispersione separa i colori (saturazione media da 0,03 a 0,16) e l'RGB ignora il numero di Abbe;
- spettrale GPU contro CPU sulla scena di prova: RMSE pari al rumore CPU−CPU (0,024 contro 0,025).

Misurato su una RTX 5070, 800 × 450, 2048 campioni: RGB 7,4 s, spettrale 14,0 s.

## Non ancora fatto

Interferenza dei film sottili (Airy), spettri misurati dei conduttori (η, k), spettri di corpo nero per le luci ed esportazione EXR spettrale. Sono i passi successivi possibili del piano.
