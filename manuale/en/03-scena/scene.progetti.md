---
id: scene.progetti
titolo: Projects, packages and demo scenes
parole: project, save, open, frproject, frpack, package, zip, folder, recent, demo, samples
ui: file.new_empty, file.open, file.save, file.save_as, file.save_pack, file.save_bundle, file.save_zip, file.demo, file.recent
correlate: scene.importa, export.immagine
---
# Projects, packages and demo scenes

**Where:** **File** menu.

A project (`.frproject`) saves the scene: models (as paths to the imported files), materials and
their assignment to the parts, lights, environment, effects, cameras, animation and render
settings. It is a text file saved safely (a crash while saving never leaves half a file); the
paths are relative, so a folder with project and models can be moved.

| Command | Keys | What it does |
|---|---|---|
| New empty scene | Ctrl+N | A scene without objects or lights |
| Demo scenes | | Ready scenes (in the `samples` folder next to the program) |
| Open project… | Ctrl+O | Opens a `.frproject` or `.frpack` |
| Recent projects | | The last projects opened |
| Save project | Ctrl+S | The first time it asks where |
| Save project as… | Ctrl+Shift+S | A copy with another name |
| Save complete package (.frpack)… | | One file with everything inside: project, imported models, material textures and HDRI. It opens on any computer |
| Save to a folder with all files… | | The project and, next to it, the models, textures and HDRI it uses: just copy the folder |
| Save as ZIP with all files… | | The same folder, compressed into a ZIP to send or archive |

## Missing files

If a model or a texture is no longer where the project says, the project opens anyway and the
console lists what is missing. To move a project to another computer, use the **package** or the
**folder with all files**.

## Closing

**File → Exit** asks before losing unsaved changes. Closing during an export asks first too.
