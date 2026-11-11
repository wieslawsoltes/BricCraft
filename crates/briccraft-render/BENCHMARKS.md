# Renderer traversal measurements

Measured on 2026-11-11 against commit `03e25226ec80f837fda226026b9c7b38a13f113f`.
[Raw measurements](benchmarks/2026-11-11-hierarchy.json) contain every draw,
source hashes, image differences, traversal counts and driver identification.
Both sides use the same current scene data, camera, 320 × 200 pixel dimensions,
quality, texture format and time-uniform updates. No resolution reduction or
geometry/material simplification is part of this comparison.

## Native Ganesh OpenGL

The driver is **Mesa llvmpipe (LLVM 19.1.7, 256 bits), OpenGL 4.5**, via EGL and
`skia-python` 144.1 on the shared Linux runner. This is a software OpenGL driver.
It is not a hardware-GPU, SwiftShader, WASM, browser-UI or presented-FPS result.
Each sample calls `flushAndSubmit(kYes)` and `glFinish`; these are completed-draw
times, not command enqueue times. Image readback is outside the timing interval.

The table is the median of the last 11 of 12 frames. Each frame receives a new
shader snapshot with a changing time uniform. The first draw is recorded but
excluded because it can compile a driver program and upload images. Driver disk
caches affect cold compilation, so first-draw times are not compared as an
optimization claim. Shared scheduling and processor frequency also affect the
small steady-state timings.

### Generated island: 7,444 parts, 66,462 occupied cells

| Quality | Baseline | Current | Change |
| --- | ---: | ---: | ---: |
| Fast | 4.100 ms | 2.818 ms | -31.3% |
| Balanced | 4.367 ms | 3.945 ms | -9.7% |
| Full | 5.692 ms | 4.498 ms | -21.0% |

The corresponding process CPU work includes llvmpipe's worker threads; it is not
the same quantity as elapsed draw latency:

| Quality | Baseline | Current | Change |
| --- | ---: | ---: | ---: |
| Fast | 25.577 ms | 16.636 ms | -35.0% |
| Balanced | 31.063 ms | 24.189 ms | -22.1% |
| Full | 38.247 ms | 27.008 ms | -29.4% |

### Mixed inspection fixture: 250 parts

| Quality | Baseline | Current | Change |
| --- | ---: | ---: | ---: |
| Fast | 2.109 ms | 2.167 ms | +2.8% |
| Balanced | 2.892 ms | 3.366 ms | +16.4% |
| Full | 3.468 ms | 3.574 ms | +3.1% |

Every pixel in this shorter scene is identical. It does not benefit uniformly:
the additional shared traversal state costs a small amount on this fixture,
especially Balanced. The column lookup is disabled when the occupied vertical
span is at most 12 studs, because clipping to a shallow global box already
removes most empty air. These measurements support a gain on the tall island;
they do not establish a universal speedup for every scene or driver.

## Native CPU raster control

The same island was rendered through CPU Skia, independently of the OpenGL
measurements. The table reports median process CPU work after excluding the first
of five frames. Both sides use the same raster backend and settings:

| Quality | Baseline | Current | Change |
| --- | ---: | ---: | ---: |
| Fast | 344.759 ms | 274.972 ms | -20.2% |
| Balanced | 438.946 ms | 411.968 ms | -6.1% |
| Full | 634.405 ms | 590.997 ms | -6.8% |

## Traversal correctness and work

In the baseline island view, 426 of 64,000 rays exhaust the 384-iteration limit.
The affected pixels form narrow missing-surface stripes. Starting a ray at an
integer camera coordinate and adding a tiny negative displacement can round
back onto the old coarse-cell boundary. The old empty-cell jump then returns the
same distance repeatedly.

Separating the integer and fractional origin prevents that roundoff. A new
conservative 4/16/64-stud column hierarchy skips only empty air above the highest
static part bound, including its stud protrusion. All qualities retain the same
3D part intersections beneath those bounds; moving assemblies use their own
local volume. The Full program now shares the primary DDA with Fast/Balanced,
while retaining its existing detailed shape and shadow intersections.

For Fast and Balanced, mean primary iterations fall from **28.239 to 10.698**
(62.1% fewer), the 95th percentile falls from **42 to 24**, and the maximum falls
from **384 to 62**. No measured ray exhausts the limit. Full has equivalent
counts (28.235 to 10.695 mean). The image differences are concentrated in the
corrected missing-surface stripes: approximately 0.5% of pixels differ by more
than eight channel values. Geometry and materials were not replaced.

`test_shader_rays.py` traces 54 deterministic probes through the actual three
production variants, with the hierarchy both enabled and disabled. It covers
tiny negative movement, axis-aligned rays, the underside of a floating platform,
misses and world bounds. All pass. The baseline fails the three tiny-negative-X
probes. `scripts/check-shaders.sh` runs this CPU-only gate on Linux and macOS
before rendering all three ordinary inspection images.

## Bounded atlas cost

The hierarchy contains **576 RGB texels**, packed into the existing opaque cell
image after coarse occupancy. For the island and 100,000-unit-plate fixtures,
these texels fit into previous row padding: combined image payload remains
**2,809,856 B** and **4,288,512 B**, respectively. There is no extra texture or
per-frame hierarchy rebuild. Global mapped-area length does not increase this
working set; it is derived from the active 96 × 72 × 96 window.

Current release atlas rebuild medians in this run are **0.284 ms** for the island
and **2.582 ms** for the 100,000-part fixture (20 samples, two warmups). These are
current measurements, not a comparison against historical runs on a different
runner. Twelve pure-data tests cover independent maximum-height reconstruction,
full-height stud bounds, moving-body exclusion, namespaces, and existing atlas
correctness/budget cases.

## Reproduce

```sh
cargo run --release -p briccraft-render-data --example export_scene -- /tmp/briccraft-scene.json
python3 -m pip install skia-python==144.1 PyOpenGL
python3 crates/briccraft-render/tools/benchmark_shader.py \
  --baseline-ref 03e25226 --scene /tmp/briccraft-scene.json \
  --backend egl --frames 12 --count-traversal --output /tmp/briccraft-gl.json
python3 crates/briccraft-render/tools/benchmark_shader.py \
  --baseline-ref 03e25226 --scene /tmp/briccraft-scene.json \
  --backend raster --frames 5 --output /tmp/briccraft-raster.json
python3 crates/briccraft-render/tools/benchmark_shader.py \
  --baseline-ref 03e25226 --backend egl --frames 12 --output /tmp/briccraft-mixed.json
cargo test -p briccraft-render-data
cargo run --release -p briccraft-render-data --example benchmark
```

`--backend egl` requires a usable EGL OpenGL driver; it never silently substitutes
CPU raster. The normal shader correctness gate remains portable CPU raster and
does not require PyOpenGL. Browser rendering must still be measured separately
using actual presented-frame counters and elapsed wall time.
