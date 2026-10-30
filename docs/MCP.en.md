# Control ForgeRender with Claude (MCP)

> **ForgeRender 0.4.0 or later · Claude Code · Windows**
>
> Ask an AI assistant to import a model, prepare a studio, place lights, assign materials, set the camera and render. Changes appear in the ForgeRender window, and the assistant receives render previews so it can iterate.

ForgeRender includes a local **Model Context Protocol (MCP)** server. No API key or network port is needed: Claude Code starts ForgeRender on your computer and communicates over standard input/output (stdio).

🇮🇹 [Versione italiana](MCP.md) · [Back to README](../README.md)

## 1. Requirements

- ForgeRender **0.4.0 or later**.
- Claude Code installed and available from PowerShell as the `claude` command.
- A Windows build of ForgeRender (portable ZIP, installer or local build).

This guide follows the currently distributed Windows packages. Do not copy example paths literally: each computer may have a different installation folder.

## 2. Locate the executable

Find `forgerender.exe` in the extracted portable folder, in `target\release` for a local build, or in the installed application directory. Use the full path to the executable, not just its containing folder. Keep the DLLs shipped with ForgeRender, including the denoiser libraries, beside the executable.

## 3. Register the MCP server in Claude Code

A dedicated working folder is recommended so ForgeRender is not launched whenever you open Claude Code in an unrelated project.

Create the folder and move into it:

~~~powershell
New-Item -ItemType Directory -Force "$HOME\ForgeRender-IA" | Out-Null
Set-Location "$HOME\ForgeRender-IA"
~~~

Set the path to your own executable. Replace the example path with the real one:

~~~powershell
$exe = (Resolve-Path "C:\REAL\PATH\TO\forgerender.exe").Path
~~~

Register the server:

~~~powershell
claude mcp add forgerender -- "$exe" --mcp
~~~

Without `--scope`, the registration is limited to the current folder. Check it with:

~~~powershell
claude mcp list
~~~

The `forgerender` server should appear in the list. Inside Claude Code, use `/mcp` to inspect its tools.

To remove the registration, run this from the same folder:

~~~powershell
claude mcp remove forgerender
~~~

### Claude Desktop

Open **Settings → Developer → Edit configuration** and edit `claude_desktop_config.json`. Replace the placeholder with the absolute path to your executable:

~~~json
{
  "mcpServers": {
    "forgerender": {
      "command": "C:\\\\REAL\\\\PATH\\\\TO\\\\forgerender.exe",
      "args": ["--mcp"]
    }
  }
}
~~~

Windows backslashes must be doubled in JSON. Save the file and restart Claude Desktop.

## 4. Use ForgeRender with Claude

Open Claude Code from the dedicated folder:

~~~powershell
claude
~~~

Claude Code also starts ForgeRender. The application window displays **AI connected (MCP)**. On first use, Claude may ask permission to call individual tools.

Example prompts:

- “Import my STL model; its dimensions are in millimetres, then prepare a photo studio.”
- “Set an anthracite background, a warm key light on the right and a cool rim light.”
- “Apply satin bronze metal to the body and matte black plastic to the base.”
- “Set a three-quarter view with an 85 mm lens and a 4:5 aspect ratio.”
- “Make a quick preview render and tell me what you would improve.”
- “The right light is too strong: halve its intensity and render again at 512 samples.”
- “Save the project and final image to the folder I specify.”

The assistant can inspect the scene before changing it. You can also edit manually in ForgeRender; later assistant operations use the updated scene. When Claude Code closes, the ForgeRender window may remain open and show **AI disconnected**. Your work is not lost.

## 5. Available tools

Measurements use **metres**, with Y pointing up. Directions such as right, left and front are relative to the render camera.

| Tool | Function |
|---|---|
| `scene_info` | Reads scene, product, components, materials, lights, camera, environment, settings and render state |
| `new_scene` | Creates an empty or demonstration scene |
| `open_project` / `save_project` | Opens or saves a `.frproject` |
| `import_model` | Imports a supported model and sets units/axis when needed |
| `prepare_studio` | Prepares the photo studio or another available look |
| `fit_scene_to_product` | Fits backdrop and lights to product dimensions |
| `undo_preset` | Reverts the last applied preset |
| `add_object` | Adds a floor, backdrop, wall or primitive |
| `transform_objects` | Moves, rotates, scales or grounds objects |
| `set_visibility` | Shows or hides components |
| `add_light` | Adds area, point, spot or directional lights |
| `edit_light` / `remove_light` | Edits or removes a light |
| `set_all_lights_color` | Sets the colour temperature of all lights |
| `list_materials` | Searches built-in and library materials |
| `apply_material` | Applies a material to components or the whole product |
| `create_material` | Creates and applies a new project material |
| `set_camera` | Sets framing, focal length, projection and depth of field |
| `set_environment` | Sets a uniform, gradient, HDRI or black environment |
| `set_render_settings` | Sets aspect ratio, resolution, samples, CPU/GPU and denoiser |
| `render` | Renders and returns the preview to the assistant |
| `stop_render` / `save_image` | Stops rendering or saves the latest image |

## 6. Output and saved renders

If no output path is specified, ForgeRender uses the output folder associated with the project or installation. You can ask the assistant to save the result to a specific absolute path.

## 7. Security

- MCP exposes the tools listed above; it is not a console for running arbitrary code inside ForgeRender.
- Arguments are validated and bounded; file paths must be absolute.
- An existing file is not overwritten without the explicit `overwrite: true` parameter.
- Library materials are not modified: `create_material` creates a new material in the project.
- Transformations can be undone with Ctrl+Z; presets have a dedicated undo command.
- The stdio transport does not open a TCP port listening on the network.

Grant MCP permissions carefully, based on the operations you want the assistant to perform.

## 8. Troubleshooting

**`claude mcp list` reports `failed`**
- Check the absolute path stored in `$exe`.
- Verify that the file exists with `Test-Path $exe`.
- Test ForgeRender on its own, started the way Claude starts it (one line, with `$exe` set as above):

  ```
  $p = New-Object System.Diagnostics.Process; $p.StartInfo.FileName = $exe; $p.StartInfo.Arguments = "--mcp"; $p.StartInfo.UseShellExecute = $false; $p.StartInfo.RedirectStandardInput = $true; $p.StartInfo.RedirectStandardOutput = $true; $null = $p.Start(); $p.StandardInput.WriteLine('{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'); $t = $p.StandardOutput.ReadLineAsync(); if ($t.Wait(20000)) { $t.Result } else { "NO ANSWER within 20 seconds" }; $p.StandardInput.Close()
  ```

  It works if a line `{"id":1,"jsonrpc":"2.0","result":{...,"serverInfo":{"name":"forgerender",...}}}`
  appears (accented letters may look garbled in PowerShell; Claude reads them correctly). The
  window opens and closes by itself. Tested on Windows with the 0.4.0 portable on an external drive.
- Launching `forgerender.exe --mcp` by hand, or piping text into it with `|`, is **not** a valid
  test: PowerShell does not connect the input of a windowed program, so ForgeRender receives
  nothing and closes immediately, as after a health check.

**ForgeRender does not respond**
- Keep the application window open and do not minimise it.
- Make sure no modal dialog is blocking the application.
- If a command times out, retry once the window is responsive again.

**Rendering is slow**
- For previews, request 32–64 samples and a smaller resolution.
- For final renders, select the GPU if available and enable the denoiser when appropriate.

Received MCP commands are recorded in the ForgeRender log (lines beginning with `MCP: …`).

## 9. Developer notes

The server is implemented inside the GUI executable: `forgerender --mcp`. Its code is in `crates/forge_app/src/mcp/`.

- `rpc.rs`: JSON-RPC 2.0 over stdin/stdout, one line per message.
- `mod.rs`: reads the protocol on a dedicated thread and forwards calls to the application window.
- `tools.rs`: tool schemas and implementations, reusing application functions.
- `ops.rs`: window-independent operations and their tests.

Logs must go to **stderr**: stdout is reserved for the MCP protocol. For implementation details and verification, see [the MCP report](../reports/2026-10-30_mcp/REPORT.md).