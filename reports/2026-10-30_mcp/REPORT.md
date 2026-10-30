# Controllo di ForgeRender da Claude tramite MCP (versione 0.4.0)

- **Data**: 30/10/2026.
- **Branch**: `claude/festive-heisenberg-d2ow6z`, poi `main` (commit di questo report).
- **Macchina**: container di sviluppo (Xvfb, GPU software lavapipe, render CPU 3 thread, senza
  OIDN).
- **Da verificare su Windows** con Claude Code vero: avvio da `claude mcp add`, permessi degli
  strumenti, finestra ridotta a icona.

## Cosa è stato fatto

`forgerender --mcp` apre la finestra normale e fa da server MCP su stdin/stdout (JSON-RPC 2.0,
protocollo 2025-06-18). 24 strumenti: scena e progetto, import, preset dello studio, oggetti,
luci, materiali, camera, ambiente, impostazioni e render con anteprima restituita. Guida per
l'utente e per gli sviluppatori: `docs/MCP.md`.

## Verifiche

| Prova | Come | Esito |
|---|---|---|
| Protocollo | test `mcp::rpc` (inizializzazione, elenco, chiamata, notifiche, errori −32700/−32601/−32602, base64, arrotondamenti) | verdi |
| Strumenti | test `mcp::tools` (ogni strumento ha schema valido e un gestore; sovrascrittura rifiutata) e `mcp::ops` (lati delle luci rispetto alla camera, colori, ricerca di luci, componenti, materiali, stato della scena) | verdi |
| Giro completo "studio" | `client_mcp.py` con `comandi_studio.json`: 13 comandi sul programma vero, render 640×360 a 32 campioni | tutti eseguiti; immagini 01 e 02; due errori voluti (luce 99, sovrascrittura) restituiti come errori |
| Giro completo "progetto" | `comandi_progetto.json`: import di un GLB, studio scuro, rotazione, ambiente, luce, salvataggio, scena vuota, riapertura del progetto, file inesistente, render | tutti eseguiti; immagine 03 (progetto riaperto e renderizzato) |
| Verifica di `claude mcp list` | inizializzazione + elenco, poi stdin chiuso | la finestra si chiude da sola (0,9 s); dopo dei comandi invece resta aperta |
| stdout pulito | i log vanno su stderr | le risposte sono solo JSON (nota: `xvfb-run` unisce stderr a stdout, lo script reindirizza stderr) |

`cargo fmt`, `cargo clippy --workspace --all-targets -D warnings` e `cargo test --workspace`
verdi.

## Limiti noti

- I comandi si eseguono a ogni aggiornamento della finestra: se è ridotta a icona può non
  rispondere; dopo 20 s senza presa in carico l'assistente riceve un errore chiaro.
- `set_camera` con focale lunga inquadra la sfera che contiene il prodotto: il prodotto può
  risultare piccolo; si corregge con `zoom`.
- Una sola chiamata alla volta (le successive aspettano in coda).

## Come rifarla

```
xvfb-run -a sh -c "..."   # solo su Linux senza schermo
python3 client_mcp.py <forgerender> <cartella_uscita> comandi_studio.json [opzioni di forgerender]
```

Su Windows basta: `python client_mcp.py ...\forgerender.exe C:\prove comandi_studio.json`
(dopo aver tolto `xvfb-run` dalla riga `Popen` dello script).
