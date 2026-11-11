# BricCraft renderer

This crate draws a real three-dimensional construction world with a FerroUI
`CustomEffect`, executed by the **Skia backend**. The UI remains FerroUI's
declarative control tree; no DOM, Canvas2D, Three.js, or separate WebGL renderer
draws the game.

## Geometry and uploads

Each active world window uses a 96 × 72 × 96 placement grid. X/Z coordinates are studs and Y
coordinates are plates, with one plate equal to 0.4 studs. A compact RGB24 cell
texture maps occupied cells to four-texel part records containing origin,
dimensions, color, shape, and quarter-turn rotation. Alpha is always opaque so
Skia's premultiplication cannot corrupt the packed data.

The backend-independent `briccraft-render-data` crate packs this bounded window.
It writes validated part cuboids directly, marks coarse occupied ranges once per
part, and keeps a compact sorted source-ID lookup. It does not scan the complete
occupancy array or perform hash lookups for every occupied cell. Atlas image
initialization remains proportional to the bounded texture size.

The SkSL clips rays to the static occupied bounds, then traverses cells with a
three-dimensional DDA and skips empty 4 × 6 × 4 coarse tiles. Bricks and their studs
have analytic box/cylinder intersections; shaped construction and mechanical
parts use bounded signed-distance intersections. Studs extend above placement
cells and are tested from the following cell, so their silhouettes are visible
without preventing another part from interlocking with them. The overhanging
stud path only intersects a cylinder; it does not invoke every complex shape's
distance function. Materials are fetched after accepting a hit. Seams belong
to whole parts rather than individual cells. A single shading call serves both
solid surfaces and shallow-water transmission, limiting driver inlining.

World revisions, active assembly membership, and body edits trigger packing.
The cell and descriptor images are compared independently: recoloring preserves
the cell image, and an identical restored snapshot publishes no new images.
FerroUI exposes immutable images, so this is reuse of unchanged complete images;
it does not claim subrectangle GPU uploads. Camera, lighting, animation,
selection, and vehicle motion change uniform values only.

An assembly has a separate local volume and packed ID namespace. The shader
transforms rays into this volume and compares its intersection with the static
world. `VehicleRender.parts: Option<Rc<[Instance]>>` can retain a body after its
original region leaves the active window. Keep its parts and pivot in a fixed
construction frame; rebase only its current pose and the camera. `instance_ids`
excludes the original parts from the **current** World and must be remapped from
stable world keys after a window swap. Detached body IDs may safely collide with
static IDs. The body budget is 256 parts, within the active window's dimensions.

Call `Renderer::invalidate()` whenever replacing a World, including replacement
with the same revision number. The browser tracks this with `scene_generation`.
For streamed worlds, `RenderFrame.window_fade = 16.0` hides the outer boundary
through atmospheric fade. Its default of zero preserves the original island;
moving vehicles remain visible at that boundary.

The viewport uses the live custom effect directly. It has no `BitmapCache`:
cache-root effect changes in the pinned FerroUI version do not invalidate the
cached root image, which would freeze the world while its HUD continues updating.

## Materials

The shader includes directional shadows, local contact shading, individual stud
shadows, rounded-edge highlight normals, plastic specular and Fresnel lighting,
sky gradients/clouds, animated ocean, distance haze, and a placement preview.
The three quality levels reduce shadow work. Sky and water are procedural;
there are no external art or texture downloads.

`Renderer::stats()` reports atlas payload/storage, active cells, coarse occupancy,
part/texture budgets, rebuilds, per-image publications, bytes published, and
uniform-only effect snapshots. These counters do not estimate presented FPS or
GPU transfer time. See [the measured CPU benchmarks](../briccraft-render-data/BENCHMARKS.md).

The geometry is a curated procedural representation of the catalog. Wheels,
gears, axles, liftarms and architecture have distinctive geometry, but it does
not reproduce every molded underside, printed decoration, or manufacturer's
mesh. Vello is not a supported rendering backend for this application.

## Native shader verification

`tools/smoke_shader.py` compiles and renders the **production** SkSL with
`skia-python` and a small mixed-part fixture. This is a shader regression tool,
not another implementation of the game. To inspect the real generated world:

```sh
cargo run --release -p briccraft-render-data --example export_scene -- /tmp/briccraft-scene.json
python3 -m pip install skia-python
python3 crates/briccraft-render/tools/smoke_shader.py /tmp/briccraft-world.png /tmp/briccraft-scene.json
```

The tool supports `--quality`, `--width`, `--height`, `--frames`, and
`--compare-source` for equal-resolution comparisons with an older concatenated
SkSL source. Timings explicitly identify the native CPU raster backend; browser
GPU throughput must be measured in the running application.
