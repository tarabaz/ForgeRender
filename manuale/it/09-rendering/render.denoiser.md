---
id: render.denoiser
titolo: Denoiser (riduzione del rumore)
parole: rumore, grana, sgranato, pulizia, oidn, cuda, anteprima
ui: panel.render.denoiser
correlate: render.campioni, intro.benvenuto
---
# Denoiser (riduzione del rumore)

**Dove si trova:** modalità Avanzata → scheda **Render** → riquadro **Riduzione del rumore
(denoiser)**. In modalità Semplice è sempre attivo con i valori migliori.

**A cosa serve:** un path tracer parte da un'immagine sgranata (rumore) che si pulisce solo con
molti campioni. Il denoiser (Intel Open Image Denoise) ripulisce l'immagine riconoscendo bordi,
colori e superfici: bastano poche centinaia di campioni per una foto pulita.

![Denoiser spento, 256 campioni: grana nel vetro e sul pavimento](img/comune/render_denoiser_off.png) ![Denoiser acceso, 256 campioni: pulita](img/comune/render_denoiser_on.png)

*Gli stessi 256 campioni senza e con il denoiser (qualità Alta, dettagli preservati).*

## Controlli

| Controllo | Valori | Predefinito | Cosa fa |
|---|---|---|---|
| Modalità | Disattivata · Solo immagine finale · Anteprima veloce + finale | Solo immagine finale (Semplice: Anteprima veloce + finale) | Quando ripulire: mai, solo alla fine, o anche durante il render |
| Usa CUDA (GPU NVIDIA) | sì / no | sì | Usa la scheda video NVIDIA, molto più veloce; senza, il denoiser passa da solo al processore |
| Qualità finale | Veloce · Bilanciata · Alta | Alta | Qualità della pulizia dell'immagine finale |
| Qualità anteprima | Veloce · Bilanciata · Alta | Veloce | Solo con "Anteprima veloce + finale" |
| Prima anteprima a | da 1 a 1024 campioni | 8 | Prima pulizia durante il render, poi a ogni raddoppio (8, 16, 32…) |
| Intensità | da 0 a 100 % | 100 % (Semplice: 90 %) | Miscela tra immagine grezza (0 %) e ripulita (100 %) |
| Preserva i dettagli (albedo e normali) | sì / no | sì | Usa colori e orientamento delle superfici per non impastare texture e spigoli |
| Vista | Finale · Grezza · Albedo · Normali | Finale | Mostra il risultato o i dati usati dal denoiser, per controllo |

Nessuna di queste impostazioni riavvia il render: l'immagine attuale si ripulisce di nuovo.

## Consigli

- **Per la foto finale:** modalità *Solo immagine finale* o *Anteprima veloce + finale*, qualità
  Alta, dettagli preservati.
- **Intensità sotto il 100 %:** lascia un filo di grana naturale; il 90 % della modalità Semplice
  evita l'effetto "plastica" sulle superfici opache.
- **Texture impastate:** controlla che **Preserva i dettagli** sia attivo.
- **Troppo pochi campioni:** con pochissimi campioni il denoiser deve inventare i dettagli e
  l'immagine sembra di cera. Dagli almeno 64 – 128 campioni, di più per vetri e nebbia.

## Errori comuni

- **"Open Image Denoise non trovato":** mancano le librerie del denoiser accanto al programma.
  Il render funziona lo stesso, ma resta sgranato.
- **"CUDA non disponibile su questa GPU":** la scheda non è NVIDIA o il driver è vecchio; il
  denoiser usa il processore, più lento ma con lo stesso risultato.
