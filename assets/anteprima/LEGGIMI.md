# Scena delle anteprime dei materiali

`anteprima.frproject` è la scena su cui ForgeRender renderizza le miniature e le anteprime dei
materiali: il modello, le due luci ad area, la camera e l'ambiente li ha scelti Valerio. Il
materiale da mostrare va sul pezzo `material_surface`; base e nucleo restano in marmo nero.
Se questa cartella manca, le anteprime usano una sfera generata dal programma.

## Modello: USD Shader Ball for glTF

File `USDShaderBallForGltf.gltf` / `.bin` e immagini, dai
[glTF Sample Assets](https://github.com/KhronosGroup/glTF-Sample-Assets/tree/main/Models/USDShaderBallForGltf)
di Khronos.

- Licenza: **Creative Commons Attribution 4.0 International (CC-BY-4.0)**, vedi
  `LICENSE-USDShaderBall.md` e `metadata-USDShaderBall.json`.
- Attribuzione: geometria e texture Chris Rydalch; specifica e validazione originali André
  Mazzone; scena originale Thomas Anagnostou; conversione in glTF Eric Chadwick. Adattamento
  dello [USD Standard Shader Ball](https://github.com/usd-wg/assets/tree/main/full_assets/StandardShaderBall).
- La cartella `USDShaderBallForGltf_texture` contiene le texture estratte dal glTF (così il
  progetto le trova anche dove la cartella del programma non è scrivibile).
