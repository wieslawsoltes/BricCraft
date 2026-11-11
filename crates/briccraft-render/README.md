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
three-dimensional DDA and skips empty 4 × 6 × 4 coarse tiles. A conservative
column hierarchy also skips air above all geometry at 4-, 16- and 64-stud scales.
It is enabled when the occupied vertical span exceeds 12 studs; shallower scenes
already clip tightly enough to avoid another column fetch.
One RGB texel carries all three maxima, including a stud-height margin in the
shader. This remains a full 3D renderer: below those bounds it resolves the
original bricks, openings, tunnels, overhangs and moving bodies. The hierarchy
does not depend on how terrain or an imported model was created.

All three qualities share this traversal. It keeps integer and fractional ray
origins separate when locating a cell, so a tiny negative movement from an
integer camera coordinate cannot round back onto the old tile and stall.
Primary volume bounds are checked once, leaving only the two special Y-row
checks in the hot cell fetches. Bricks and their studs
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

The renderer also retains the latest complete `BrickWorldEffect` and its
`Rc<dyn IEffect>`. After checking revisions, assembly membership and every
uniform, an identical request returns that same Rc. The comparison includes
atlas image identity, camera, animation time, daylight, selection, preview,
quality, sample count and vehicle pose/motion. A frozen modal or paused world
can therefore keep its existing world image while the native HUD changes.
This does not pause animation by itself: the host must hold visual time fixed.
`invalidate_effect()` drops only that snapshot; `invalidate()` also requests an
atlas revision check after a World replacement. Neither discards unchanged
atlas images or resets upload counters.

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
camera. The host chooses physical resolution, shader detail and spatial samples
independently. Resolution policy belongs to the application, not this renderer;
there is no hidden low-resolution cap in the effect.

## Spatial antialiasing and materials

`RenderFrame.samples` selects **1, 2 or 4 samples per physical world pixel**,
independently of geometry/shadow quality. The default is 1. Unsupported counts
round down to the nearest supported value, with 0 treated as 1. A center ray,
a symmetric diagonal pair, or a rotated four-sample grid evaluates the same
production scene function. The shader averages their lighting before applying
the display shoulder and vignette. It uses no temporal history, random jitter,
motion reprojection, or extra scene textures.

Sampling resolves actual silhouette coverage, including stud cylinders, bores,
slopes, rails and moving constructions. Geometry is never inflated to make a
part appear antialiased. Work is explicitly bounded at four scene evaluations;
it can approach four times the ray/shading cost. Prefer enough physical world
pixels before spending more rays on each pixel. Spatial sampling cannot recover
detail lost by displaying a very small world image at a much larger size.
Skia's antialiasing of the viewport rectangle does not cover the silhouettes
computed inside that rectangle's runtime shader.

Fine surface details use an independent analytic filter at every sample count:

- Pinhole ray differentials are projected onto the actual hit tangent plane,
  then transformed into assembly coordinates when needed. This gives a local
  footprint without GPU derivatives or neighboring volume fetches. The grazing
  denominator and maximum footprint are bounded.
- Part seams, stud-cap rings, construction grids and preview lines integrate
  coverage over that footprint. Periodic grids approach their average area at a
  distance instead of oscillating between solid and missing lines.
- Tire treads, analytic gear stripes and ocean waves use a box-filtered sinusoid.
  Quarter-turned wheels use the same canonical axis convention as their geometry.
- Molded-edge normal detail fades when unresolved. A bounded footprint variance
  broadens narrow specular highlights on curved surfaces. Flat face interiors
  retain their normal and roughness.
- Plastic, smooth tiles and rubber have separate roughness values. Direct
  highlights use a GGX distribution, height-correlated Smith visibility and
  Schlick Fresnel. Rough environment reflections use a smooth sky lobe, so
  high-frequency cloud noise does not sparkle in distant plastic.
- A stud's projected shadow is a filtered capsule on its supporting top face.
  The actual stud geometry and the existing scene shadow queries are preserved.

`filtering.sksl`, `atmosphere.sksl`, `materials.sksl`,
`lighting.sksl`, and `water.sksl` are shared by all three geometry qualities.
The existing palette, artistic ambient lighting and display curve remain;
the renderer does not claim a fully calibrated photometric color pipeline.
Sky, water and surface details are procedural, with no external art downloads.

References: [Skia runtime-effect composition and coordinates](https://docs.skia.org/docs/user/sksl/),
[Filament's microfacet BRDF derivation](https://google.github.io/filament/main/filament.html),
and [specular antialiasing and roughness filtering](https://google.github.io/filament/main/materials.html).
The footprint here is an analytic ray differential, not Filament's
screen-derivative implementation.

Native verification, measured coverage error and equal-resolution cost are
recorded in [the antialiasing report](ANTIALIASING.md).

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
uniform-only effect snapshots. Reused requests do not increment those snapshot
counts. `Renderer::reused_effects()` reports such requests; `Renderer::samples()`
reports the normalized sample count. `shader_quality` and `shader_variant`
identify the actual program selected for the most recent frame. `height_columns` reports the
bounded column working set. These counters do not estimate presented FPS or
GPU transfer time. See [the shader measurements](BENCHMARKS.md) and
[the atlas measurements](../briccraft-render-data/BENCHMARKS.md).

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
python3 -m pip install skia-python==144.1
python3 crates/briccraft-render/tools/smoke_shader.py /tmp/briccraft-world.png /tmp/briccraft-scene.json --all-qualities --width 160 --height 100
python3 crates/briccraft-render/tools/test_shader_rays.py
python3 crates/briccraft-render/tools/test_antialias.py --output /tmp/briccraft-aa
```

The tool reads the exact `include_str!` sequence from each production Rust source
constant; it does not maintain a separate shader composition. `--all-qualities`
compiles and renders all three programs into separately named PNGs. The default
fixture also checks for visible warm-colored bricks/roof pixels, so an image of
sky and water alone cannot pass. It also
supports `--quality`, `--samples 1|2|4`, `--width`, `--height`, `--frames`, and
`--compare-source` for equal-resolution comparisons with an older concatenated
SkSL source. CPU raster is the default. `--backend egl` selects synchronized
native Ganesh OpenGL, requires PyOpenGL/EGL, and reports the actual driver.
`--animate-uniforms` verifies repeated draws with changing effect snapshots.
`tools/benchmark_shader.py` compares each production source against its matching
variant in a Git revision at identical settings. Use `--samples 1` when comparing
with an older source that has no sampling uniform; reports distinguish the
requested count from the count actually supported by that source. Browser GPU
throughput must still be measured in the running application.

`tools/test_antialias.py` keeps the actual sample pattern, resolve, primary
traversal, geometry and moving assembly transforms. Its silhouette adapter
replaces only surface/background colors and the display curve, then compares
1/2/4 samples against an 8× denser reference. It also compares periodic/interval
filters against 65,536-tap numerical integration and analytic surface
differentials against independent finite-difference ray/plane intersections,
including transformed assemblies. It writes a JSON error report and PNG
fixtures. These are correctness measurements, not a browser FPS estimate.
Existing tiny-negative-direction, boundary and hierarchy ray probes remain
unchanged. `effect_cache.rs` has native unit tests for paused snapshot identity,
visible uniform changes and explicit invalidation.


The historical CPU benchmark report predates these specialized shader programs.
It is not evidence of current browser GPU throughput. Browser performance gates
use FerroUI's actual presentation counters and elapsed wall time.
