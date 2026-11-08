---
id: ai.mcp
titolo: AI control (MCP)
parole: ai, artificial intelligence, claude, codex, assistant, mcp, natural language, automation, bridge
ui: tools.ai_control, tools.ai_setup
correlate: intro.primo-render, ui.finestra
---
# AI control (MCP)

**Where:** the **AI MCP** switch in the top bar (the gear next to it opens the settings);
**Tools → AI integration…**.

**What it does:** an AI assistant (Claude Desktop, Claude Code or Codex) can prepare the scene
and render for you: "add a studio backdrop, a warm light on the right, make the part satin
metal and render a square image". You see every change live in the ForgeRender window, and the
assistant receives the preview of the render so it can correct it. The connection uses the
**Model Context Protocol** (MCP): no internet connection or API key is needed, everything stays
on your computer.

## You decide

- **AI MCP off** (always off at start-up): no assistant can touch the scene.
- **AI MCP on:** an assistant can connect.
- **Controlled by AI** (or by Claude Code, Claude Desktop, Codex) appears while an assistant is
  connected. When it disconnects your work stays: continue by hand or reconnect it later.

## Connecting with one click

**Tools → AI integration…** shows the state of the ForgeRender MCP server and of the bridge
`forgerender-mcp.exe`, then one row per assistant: *Not installed*, *Not configured*, *Needs
update*, *Configured*, **Connected**. **Connect**, **Update** and **Disconnect** do everything:
the program writes only its own entry in the assistant's configuration and makes a backup copy
first.

| Assistant | After "Connect" |
|---|---|
| Claude Desktop | Restart it once (the button does it); then every chat sees ForgeRender |
| Claude Code | Open a new session |
| Codex | Open a new session |
| ChatGPT | Not supported: it connects only remote servers on the internet |

## What the assistant can do

Read the scene, import models, prepare the studio, add and edit lights, create and apply
materials, set the environment and the camera, change the render settings, render, look at the
result and save image and project. It does not overwrite existing files unless you ask it to.

The full guide, with examples and the list of tools, is in `docs/MCP.en.md` in the program
repository.
