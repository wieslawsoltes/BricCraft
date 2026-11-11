#!/usr/bin/env bash
# Compile and render every production shader variant before exercising the browser.
set -euo pipefail

destination="${1:-target/shader-checks}"
python="${BRICCRAFT_SKIA_PYTHON:-python3}"
mkdir -p "$destination"

# Tiny deterministic rays catch a numerical DDA stall which a colorful scene
# screenshot alone cannot detect. This gate uses CPU raster Skia on every OS.
"$python" crates/briccraft-render/tools/test_shader_rays.py

for quality in 0 1 2; do
    "$python" crates/briccraft-render/tools/smoke_shader.py \
        "$destination/shader-quality-$quality.png" \
        --quality "$quality" --width 320 --height 200 --frames 2
done
