# Controllo IA separato: ponte `forgerender-mcp` e interruttore nella finestra (0.5.0)

- **Data**: 30/10/2026.
- **Branch**: `claude/festive-heisenberg-d2ow6z`, poi `main`.
- **Perché**: nella 0.4.0 Claude avviava ForgeRender e lo chiudeva a fine sessione (prova su
  Windows: "ha chiuso di nuovo il software"). L'utente vuole aprirlo lui, tenerlo aperto e
  decidere quando l'IA lo comanda.
- **Macchina**: container di sviluppo (Xvfb, render CPU), **Claude Code 2.1.314 vero** come
  client MCP (`claude -p`).

## Cosa cambia

| Prima (0.4.0) | Ora (0.5.0) |
|---|---|
| Claude avvia `forgerender.exe --mcp` | L'utente apre ForgeRender; Claude avvia `forgerender-mcp.exe` |
| La finestra muore con la sessione di Claude | La finestra resta aperta; ci si ricollega |
| Sempre comandabile mentre è aperta | Solo con "Controllo IA" acceso (spento a ogni avvio) |
| Render lunghi: il client smette di aspettare | `render` risponde "in corso" dopo ~45 s, `render_status` dà il risultato |

## Verifiche

| Prova | Esito |
|---|---|
| Test automatici: protocollo, chiave (giusta/sbagliata), giro ponte ↔ finestra, spegnimento che toglie il file di contatto | verdi |
| `claude mcp list` con il ponte | "√ Connected" (anche a finestra chiusa: il ponte risponde da solo) |
| Sessione 1 di Claude: studio, luce calda a destra, Acciaio satinato, render 480×270 | eseguita; dopo lo scollegamento la finestra è ancora aperta |
| Sessione 2 di Claude (nuova) | trova "Luce calda destra" e l'Acciaio satinato lasciati dalla sessione 1 |
| Render 960×540 a 256 campioni (77 s su CPU) | `render` risponde "in corso", Claude chiama `render_status`, la finestra salva il PNG e Claude vede l'immagine (04) |
| Finestra chiusa di colpo (file di contatto rimasto) | il ponte risponde a `initialize` e `tools/list` (25 strumenti) e ai comandi con "apri ForgeRender e accendi Controllo IA" |
| Indicatore | "Controllato da IA" visibile durante il comando (01, 03), sparito dopo (02) |

## Da verificare su Windows

- Pulsante "Controllo IA" e indicatore sul PC reale.
- File di contatto in `%LOCALAPPDATA%\ForgeRender\controllo-ia.json`.
- Configurazione `.mcp.json` con il percorso di `forgerender-mcp.exe` (guida `docs/MCP.md`).
