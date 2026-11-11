# BricCraft renderer

This crate draws a real three-dimensional construction world with a FerroUI
`CustomEffect`, executed by the **Skia backend**. The UI remains FerroUI's
declarative control tree; no DOM, Canvas2D, Three.js, or separate WebGL renderer
draws the game.

## Geometry and uploads

The world uses a 96 × 72 × 96 placement grid. X/Z coordinates are studs and Y
coordinates are plates, with one plate equal to 0.4 studs. A compact RGB24 cell
texture maps occupied cells to four-texel part records containing origin,
dimensions, color, shape, and quarter-turn rotation. Alpha is always opaque so
Skia's premultiplication cannot corrupt the packed data.

The SkSL traverses cells with a three-dimensional DDA. Bricks and their studs
have analytic box/cylinder intersections; shaped construction and mechanical
parts use bounded signed-distance intersections. Studs extend above placement
cells and are tested from the following cell, so their silhouettes are visible
without preventing another part from interlocking with them. Seams belong to
whole parts rather than individual cells.

Textures are recreated only when the world's revision or active assembly's
membership changes. Camera, lighting, animation, selection, and vehicle motion
change uniform values. Activating an assembly moves its members into a local
volume in the same texture; the shader transforms rays into this volume and
compares its intersection with the static world. Vehicle motion does not rebuild
or upload geometry.

## Materials

The shader includes directional shadows, local contact shading, individual stud
shadows, rounded-edge highlight normals, plastic specular and Fresnel lighting,
sky gradients/clouds, animated ocean, distance haze, and a placement preview.
The three quality levels reduce shadow work. Sky and water are procedural;
there are no external art or texture downloads.

The geometry is a curated procedural representation of the catalog. Wheels,
gears, axles, liftarms and architecture have distinctive geometry, but it does
not reproduce every molded underside, printed decoration, or manufacturer's
mesh. Vello is not a supported rendering backend for this application.

## Native shader verification

`tools/smoke_shader.py` compiles and renders the **production** SkSL with
`skia-python` and a small mixed-part fixture. This is a shader regression tool,
not another implementation of the game. To inspect the real generated world:

```sh
cargo run -p briccraft-render --example export_scene -- /tmp/briccraft-scene.json
python3 -m pip install skia-python
python3 crates/briccraft-render/tools/smoke_shader.py /tmp/briccraft-world.png /tmp/briccraft-scene.json
```
