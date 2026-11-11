#!/usr/bin/env bash
set -euo pipefail
BRICCRAFT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BRICCRAFT_SITE="${1:-${SITE:-$BRICCRAFT_ROOT/target/browser/briccraft-browser}}"
cd "$BRICCRAFT_ROOT"
rustup target add wasm32-unknown-unknown
cargo build --locked -p briccraft-import-worker --target wasm32-unknown-unknown --profile browser
mkdir -p "$BRICCRAFT_SITE/import-engine"
wasm-bindgen --target web --out-dir "$BRICCRAFT_SITE/import-engine" \
  "${CARGO_TARGET_DIR:-$BRICCRAFT_ROOT/target}/wasm32-unknown-unknown/browser/briccraft_import_worker.wasm"
