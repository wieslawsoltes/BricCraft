# Renderer measurements

Measured on 2026-11-11 in the shared Linux x86_64 development runner, using
Rust 1.99.0 and native `skia-python` 144.1. Baseline code comes from commit
`2c14d12`. Both packers use the same current domain code and fixtures.
[The complete measurements](benchmarks/2026-11-11.json) retain elapsed times,
process CPU times, image differences, counts, and image payload sizes.

These are CPU packing and native CPU shader measurements. They do not measure
browser WebGL frame rate, WASM startup, Skia GPU compilation, or driver upload
latency. The browser runs the shader through FerroUI's Skia backend; its actual
frame rate must be measured separately in the running application.

## Atlas packing

Release optimization, thin LTO, two warmups and twenty measured rebuilds per
fixture. Scene generation and test-data construction are outside the timed
section. Linux process CPU time removes time spent descheduled on the shared
runner; wall latency is retained in the JSON.

| Fixture | Parts | Occupied cells | Baseline median CPU work | Current median CPU work | Reduction | Current median elapsed latency |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Generated island | 7,444 | 66,462 | 2.085 ms | 0.540 ms | 74.1% | 0.541 ms |
| Dense unit plates | 100,000 | 100,000 | 7.972 ms | 4.842 ms | 39.3% | 4.844 ms |

The current elapsed p95 values were 0.649 ms and 6.662 ms respectively. These
numbers describe this runner and these scenes, not a latency guarantee on every
device. Packing happens on edits/window publication, not on camera-only or
vehicle-pose-only frames.

The improvements come from writing validated cuboids directly, eliminating the
full occupancy scan and per-voxel hash lookups, marking each part's coarse range
once, and using a sorted compact ID lookup instead of constructing a hash map.
Initialization still writes a bounded image buffer; the benchmark includes it.

| Fixture | Active placement cells | Combined RGBA8 atlas payload | ID lookup storage | Owned staging buffers plus lookup |
| --- | ---: | ---: | ---: | ---: |
| Generated island | 663,552 | 2,809,856 B | 59,552 B | 2,869,408 B |
| Dense unit plates | 663,552 | 4,288,512 B | 800,000 B | 5,088,512 B |

Image payloads are identical in size to the baseline. These sizes exclude World
storage, temporary build inputs, FerroUI bitmap copies, and GPU-driver resources.
`AtlasStats` explicitly distinguishes payload, staging, and image-publication
counters. Global world length does not enlarge this working volume: only the
active 96 × 72 × 96 window and the bounded moving body are packed.

Reproduce:

```sh
cargo test -p briccraft-render-data
cargo run --release -p briccraft-render-data --example benchmark
python3 crates/briccraft-render-data/tools/compare_atlas.py --baseline-ref 2c14d12
```

If release domain libraries already exist and another Cargo task holds the build
directory lock, add `--reuse-libraries` to the comparison command. It compiles
only the small packers and benchmark binaries, finishes both compilers, and then
runs the measurements. It uses the same optimization and thin-LTO settings.

## Native shader comparison before adaptive shader variants

These measurements describe the shader in `e8e5c6e`, before the separate analytic Fast/Balanced programs were introduced. They remain a historical comparison of traversal changes, not measurements of the later adaptive renderer. The atlas measurements above are unaffected.

Both shader versions rendered the same fixed 400 × 250 view in Balanced quality.
Each rendered three frames; the table is the median CPU work of the last two.
The native raster surface executes CPU Skia, not browser GPU code.

| Fixture | Baseline draw CPU work | Current draw CPU work | Pixel comparison |
| --- | ---: | ---: | --- |
| Mixed parts, 250 pieces | 742.840 ms | 474.385 ms | Every pixel identical |
| Generated island, 7,444 pieces | 1,027.824 ms | 1,073.590 ms | Every pixel identical |

The mixed fixture benefits substantially from clipping traversal to occupied
bounds and removing duplicated complex-part work. The generated island fills
most of the window and does not show a CPU draw-time improvement in this small
sample: the current result is 4.5% higher. No general browser rendering speedup
is inferred from these CPU results. GPU timing remains a separate validation.

The measured shader retained the same geometry and materials for existing worlds. It
used an analytic stud-only branch, fetches material after accepting a hit, has
one solid-surface shading call site, and stops tracing beyond opaque water.
Its traversal limits were fixed at 384 primary DDA steps, 96 shadow steps and
28 complex-shape march steps; they do not grow with global region count.

Generate the actual scene without building FerroUI:

```sh
cargo run --release -p briccraft-render-data --example export_scene -- /tmp/briccraft-scene.json
python3 -m pip install skia-python numpy
python3 crates/briccraft-render/tools/smoke_shader.py /tmp/world.png /tmp/briccraft-scene.json --width 400 --height 250 --frames 3
```

To reproduce the historical shader comparison, extract both measured sources:

```sh
python3 - <<'PY'
from pathlib import Path
import subprocess
names = ('uniforms', 'geometry', 'trace', 'lighting', 'main')
for ref, output in [('2c14d12', '/tmp/briccraft-baseline.sksl'), ('e8e5c6e', '/tmp/briccraft-measured.sksl')]:
    source = '\n'.join(subprocess.check_output(
        ['git', 'show', f'{ref}:crates/briccraft-render/src/shaders/{name}.sksl'],
        text=True) for name in names)
    Path(output).write_text(source)
PY
python3 crates/briccraft-render/tools/smoke_shader.py /tmp/world.png /tmp/briccraft-scene.json --width 400 --height 250 --frames 3 --source /tmp/briccraft-measured.sksl --compare-source /tmp/briccraft-baseline.sksl
```

Omit the scene argument to use the mixed-part fixture. The tool reports both
wall and process CPU draw times and saves the baseline image beside the current
one. Shader compilation time is reported separately.

## Streaming correctness checks

Ten focused data tests cover opaque encoding, RGB24 boundaries, stud-overhang
coarse tiles, exact agreement with the generated World's occupancy, 100,000
parts, legacy activation, detached/static ID collisions, independent descriptor
changes, unchanged body-local geometry after rebasing, and explicit body limits.

A native shader check also rendered a detached body with construction origin
`[-100, 0, 200]` while the static World was empty. It changed 446 pixels in the
200 × 125 view. Enabling a 16-stud window fade changed zero vehicle pixels,
confirming that boundary fog does not hide moving bodies. The browser integration
preserves the body's Rc/pivot frame, remaps current-world exclusion IDs, and
invalidates renderer packing on each published snapshot generation.
