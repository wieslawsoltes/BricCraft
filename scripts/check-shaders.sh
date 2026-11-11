#!/usr/bin/env bash
# Compile and render every production shader variant before exercising the browser.
set -euo pipefail

destination="${1:-target/shader-checks}"
python="${BRICCRAFT_SKIA_PYTHON:-python3}"
mkdir -p "$destination"

for quality in 0 1 2; do
    "$python" crates/briccraft-render/tools/smoke_shader.py \
        "$destination/shader-quality-$quality.png" \
        --quality "$quality" --width 320 --height 200 --frames 2
done
