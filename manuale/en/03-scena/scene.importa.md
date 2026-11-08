---
id: scene.importa
titolo: Importing a model
parole: import, open model, gltf, glb, obj, stl, step, stp, blender, blend, solidworks, cad, units, tessellation
ui: file.import
correlate: scene.oggetto, scene.progetti, intro.primo-render
---
# Importing a model

**Where:** **File → Import model…** (Ctrl+I), or **Import model…** in Simple mode.

| Format | Notes |
|---|---|
| glTF / GLB | Recommended (also the best export from SolidWorks): materials, textures, lights and cameras come with it |
| OBJ (with .mtl) | Materials of the .mtl file are read |
| STL | Geometry only; units must be chosen |
| Blender `.blend` | Through an installed copy of Blender |
| STEP (.step, .stp) | Converted once into triangles, with its own window (see below) |
| SolidWorks SLDPRT / SLDASM | Experimental |

## Import options

| Option | Meaning |
|---|---|
| Units (STL / OBJ) | Millimetres, centimetres, metres or inches: these files do not say which unit they use |
| Vertical axis | Automatic: STL with Z up; OBJ, STEP and SolidWorks with Y up |
| Triangulation quality (STEP) | Low, Medium, High |
| Position: Place on the floor after import | The model lands exactly on the floor |
| Normal crease angle | Above this angle between two faces the edge stays sharp, below it the surface is smoothed |

Every model keeps the structure of its parts: the tree of components appears in the Scene panel
on the left, and every part can get its own material.

## STEP files

A single STEP file opens its own window: the file on the left, the tessellation settings in the
middle and, on the right, the model **as it will be imported**: the same parts, colors, vertical
axis and position on the floor, drawn exactly as in the viewport. Choose **Shaded** or
**Wireframe** (the edges of the triangles, like the Mesh view of the viewport); drag to orbit,
Shift or the right button to pan, wheel to zoom, double click or **Frame** to frame it again.
Below are triangles, vertices, size and parts. Changing the vertical axis or **Place on the floor
after import** updates the preview at once, without converting again.

| Tessellation | Linear / angular deviation | When |
|---|---|---|
| Draft | 1 % and 28.6° | To try quickly |
| Balanced | 0.4 % and 17.2° | The usual value: clean curves at a reasonable cost |
| High quality | 0.15 % and 8.6° | Close-ups and glossy surfaces |
| Custom | the two values below | **Linear deviation**: how far the mesh may move from the CAD surface. **Angular deviation**: maximum angle between two neighbouring triangles on a curve |

You can **Convert** as often as you like; the model enters the scene only with **Import**, and it
is exactly the model of the preview (it is not converted again). The result is kept: reopening
the project does not convert the STEP again.

## Common mistakes

- **Huge or tiny model:** wrong units for STL/OBJ. Reimport with the right unit, or use the quick
  scale buttons in the [Object](scene.oggetto) tab (×25.4 converts inches to millimetres).
- **Faceted curves on a STEP:** convert again with a higher tessellation quality.
