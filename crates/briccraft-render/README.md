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
have analytic box/cylinder intersections. Fast and Balanced solve shaped parts
analytically as well; Full uses the original bounded signed-distance intersections.
The analytic traversal caches each coarse tile while visiting its fine cells. Studs extend above placement
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

The browser can render the world at a separate physical-pixel budget while the
HUD stays at display resolution. Its bitmap-cache parent contains a child with
the live custom effect, so descendant effect updates invalidate that parent.
The effect must not be placed on the cached parent itself: changes to an effect
on the cache root do not invalidate its own image in the pinned framework.
Resolution changes do not alter logical layout, input coordinates, or the world
camera. The adaptive controller measures actual presented-frame intervals and
caps effective shader quality at the user's requested setting.

## Materials

The shader includes directional shadows, local contact shading, individual stud
shadows, rounded-edge highlight normals, plastic specular and Fresnel lighting,
sky gradients/clouds, animated ocean, distance haze, and a placement preview.
Sky and water are procedural; there are no external art or texture downloads.

## Compiled quality variants

Quality selects a distinct Rust `CustomEffect` type and SkSL source. The framework
therefore caches three actual programs; a fast frame does not compile or execute
the Full shader's nested distance marcher and six-sample distance-field normals.
All three use exactly the same uniform/child layout and immutable atlas images.

| Quality | Source | Geometry and lighting |
| --- | --- | --- |
| 0, Fast | `FAST_WORLD_SKSL` | Analytic parts and studs; molded highlights, stud self-shadowing, animated water and atmospheric boundary; no secondary shadow/AO traversal or cloud-noise branch. |
| 1, Balanced | `BALANCED_WORLD_SKSL` | The same analytic silhouettes, four-sample contact shading, bounded occupancy shadows and clouds. Assembly shadows use their bounds. |
| 2, Full | `WORLD_SKSL` | Original signed-distance shapes and exact shaped-part shadow intersections. |

The analytic variants retain actual wheel/gear axial bores, liftarm pin holes,
window and door openings, arches, clipped slopes, cross axles, and rotating
propellers. Gear teeth use material modulation over an annular silhouette, rail
sleepers use a continuous distant-detail strip, and molded corner rounding is
represented by highlight normals. Full preserves the detailed distance-field silhouettes. Both paths normalize
the catalog's long/thin axes consistently and constrain spinning shafts/blades
to their declared occupied bounds, so promotion does not rotate pin holes or
propellers. These are explicit quality choices rather than changes to catalog
connectivity, collision geometry, construction data, or vehicle mechanics.

`Renderer::stats()` reports atlas payload/storage, active cells, coarse occupancy,
part/texture budgets, rebuilds, per-image publications, bytes published, and
uniform-only effect snapshots. `shader_quality` and `shader_variant` identify the
actual program selected for the most recent frame. These counters do not estimate presented FPS or
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
python3 crates/briccraft-render/tools/smoke_shader.py /tmp/briccraft-world.png /tmp/briccraft-scene.json --all-qualities --width 160 --height 100
```

The tool reads the exact `include_str!` sequence from each production Rust source
constant; it does not maintain a separate shader composition. `--all-qualities`
compiles and renders all three programs into separately named PNGs. The default
fixture also checks for visible warm-colored bricks/roof pixels, so an image of
sky and water alone cannot pass. It also
supports `--quality`, `--width`, `--height`, `--frames`, and
`--compare-source` for equal-resolution comparisons with an older concatenated
SkSL source. Timings explicitly identify the native CPU raster backend; browser
GPU throughput must be measured in the running application.

The historical CPU benchmark report predates these specialized shader programs.
It is not evidence of current browser GPU throughput. Browser performance gates
use FerroUI's actual presentation counters and elapsed wall time.
