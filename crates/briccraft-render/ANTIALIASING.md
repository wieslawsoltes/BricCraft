# Spatial antialiasing verification

Measured on 2026-11-11 with native Skia CPU raster, skia-python 144.1,
Python 3.12.14 and NumPy 2.3.5 on an AMD EPYC 9V45 host running Linux.
[The machine-readable report](benchmarks/2026-11-11-antialias.json) preserves
the individual draws, source hashes, errors and scene settings.

These measurements concern native shader correctness and arithmetic cost.
They do not estimate browser frame rate, GPU throughput, or the performance of
a full running game. The host is shared; the draw samples show scheduling and
frequency variance. The older performance reports remain historical records
for their own shader revisions.

## Actual geometry coverage

`test_antialias.py` renders a 15-part fixture: 12 static shapes and three parts
in an independently rotated assembly. It includes real studs, slope edges,
thin members, quarter-turned tires, pin holes, window openings and an arch.
The production traversal, geometry, sample offsets and resolve are unchanged.
Only lighting/background colors and the display curve are replaced to isolate
silhouette coverage.

For each quality, an eight-times denser image (576 × 384) provides a box-averaged
reference for the 72 × 48 test image. This covers both geometry variants; the
Fast and Balanced silhouettes are intentionally identical.

| Geometry variant | 1 sample mean absolute coverage error | 2 samples | 4 samples | Error reduction, 4 versus 1 |
| --- | ---: | ---: | ---: | ---: |
| Fast analytic | 1.6249% | 1.0922% | 0.6183% | 61.9% |
| Balanced analytic | 1.6249% | 1.0922% | 0.6183% | 61.9% |
| Full SDF | 1.6253% | 1.0844% | 0.6493% | 60.1% |

The fixture contains 277 partially covered pixels for analytic geometry and
279 for Full geometry. On those edge pixels, four samples reduced mean absolute
error from 20.27% to 7.71% and from 20.13% to 8.04%, respectively.
This is a reproducible result for this fixture, not a guarantee for every view.
The CI gate requires improvement over center sampling, rather than exact image
identity across Skia versions.

The same run passed 16 coverage/filter probes against independent 65,536-tap
numerical integration and 24 footprint probes against finite-difference
ray/plane intersections. The footprint probes include all sample counts and
a rotated assembly. Maximum errors were 0.001961 and 0.001936, respectively,
within the approximately half-channel quantization of RGBA8 output.

All 54 existing production traversal probes passed, including tiny negative
directions at integer cell boundaries. All three complete production material
shaders also compiled and rendered the 250-part inspection fixture at four
samples, including the visible warm-brick/roof guard. A 640 × 400 Balanced,
four-sample render was visually inspected.

## Cost at equal physical resolution

The cost comparison uses the 250-part mixed fixture at 160 × 100, with
**one sample** in both the old and new sources. It compares baseline commit
`91728add6e07bddb9478cba87cfef291e03d2f8a` with the sampled/filtered source.
Each source draws eight frames with changing time uniforms; the first draw is
excluded from the steady median. Order alternates across quality levels.

| Quality | Baseline median | Filtered median | Observed change |
| --- | ---: | ---: | ---: |
| Fast | 41.00 ms | 51.16 ms | +24.8% |
| Balanced | 60.45 ms | 64.98 ms | +7.5% |
| Full | 79.58 ms | 81.44 ms | +2.3% |

The material filters add arithmetic. This change does **not** claim improved
ray throughput. Two and four samples add bounded scene evaluations and can
approach two and four times the ray/shading work. The application's rendering
policy should secure adequate physical resolution before adding samples.

Primary traversal work was identical before and after: mean 6.0721 iterations
for analytic geometry, 6.0733 for Full, 95th percentile 25 and maximum 47, with
zero pixels exhausting the 384-iteration bound. No occupancy or atlas contract
changed. The stationary-world optimization is separate: exact immutable
effect reuse lets a paused world keep its cached image while native menus
continue to update.

## Reproduce

Install `skia-python==144.1` and NumPy in an isolated Python environment, then:

```sh
python3 crates/briccraft-render/tools/test_shader_rays.py
python3 crates/briccraft-render/tools/test_antialias.py --output /tmp/briccraft-aa
python3 crates/briccraft-render/tools/smoke_shader.py /tmp/briccraft-aa-smoke.png --all-qualities --samples 4 --width 160 --height 100 --frames 2
python3 crates/briccraft-render/tools/benchmark_shader.py --baseline-ref 91728add6e07bddb9478cba87cfef291e03d2f8a --output /tmp/briccraft-aa-cost.json --width 160 --height 100 --samples 1 --frames 8 --count-traversal
```

The AA tool writes numerical reports and geometry-coverage images. The smoke
tool uses the complete production lighting and shader composition, without the
coverage adapter. Benchmark reports record both the requested sample count and
the count actually supported by an older baseline.
