---
id: ai.mcp
titolo: Controllo con l'IA (MCP)
parole: ia, intelligenza artificiale, ai, claude, codex, assistente, mcp, linguaggio naturale, automazione, ponte
ui: tools.ai_control, tools.ai_setup
correlate: intro.primo-render, ui.finestra
---
# Controllo con l'IA (MCP)

**Dove si trova:** l'interruttore **AI MCP** nella barra in alto (l'ingranaggio accanto apre le
impostazioni); **Strumenti → Integrazione AI…**.

**A cosa serve:** un assistente IA (Claude Desktop, Claude Code o Codex) può preparare la scena e
fare il render per te: "aggiungi un fondale da studio, una luce calda a destra, fai il pezzo in
metallo satinato e fai un render quadrato". Vedi ogni modifica dal vivo nella finestra di
ForgeRender, e l'assistente riceve l'anteprima del render per correggerla. Il collegamento usa il
**Model Context Protocol** (MCP): non servono internet né chiavi API, resta tutto sul tuo
computer.

## Decidi tu

- **AI MCP spento** (sempre spento all'avvio): nessun assistente può toccare la scena.
- **AI MCP acceso:** un assistente può collegarsi.
- **Controllato da AI** (o da Claude Code, Claude Desktop, Codex) compare mentre un assistente è
  collegato. Quando si scollega il tuo lavoro resta: continui a mano o lo ricolleghi dopo.

## Collegare con un clic

**Strumenti → Integrazione AI…** mostra lo stato del server MCP di ForgeRender e del ponte
`forgerender-mcp.exe`, poi una riga per assistente: *Non installato*, *Non configurato*, *Da
aggiornare*, *Configurato*, **Collegato**. **Collega**, **Aggiorna** e **Scollega** fanno tutto: il
programma scrive solo la sua voce nella configurazione dell'assistente e prima ne fa una copia.

| Assistente | Dopo "Collega" |
|---|---|
| Claude Desktop | Riavvialo una volta (lo fa il pulsante); poi ogni chat vede ForgeRender |
| Claude Code | Apri una sessione nuova |
| Codex | Apri una sessione nuova |
| ChatGPT | Non supportato: collega solo server remoti su internet |

## Cosa può fare l'assistente

Leggere la scena, importare modelli, preparare lo studio, aggiungere e modificare luci, creare e
applicare materiali, impostare ambiente e camera, cambiare le impostazioni di render, fare il
render, guardare il risultato e salvare immagine e progetto. Non sovrascrive file esistenti se non
glielo chiedi.

La guida completa, con esempi e l'elenco degli strumenti, è in `docs/MCP.md` nel repository del
programma.
