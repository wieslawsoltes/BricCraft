#!/usr/bin/env bash
set -euo pipefail
BRICCRAFT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export FERROUI_BROWSER_WORKSPACE="$BRICCRAFT_ROOT"
BRICCRAFT_SITE="$BRICCRAFT_ROOT/target/browser/briccraft-browser"
BRICCRAFT_ARGS=("$@")
for ((BRICCRAFT_I=0; BRICCRAFT_I<${#BRICCRAFT_ARGS[@]}; BRICCRAFT_I++)); do
  if [[ "${BRICCRAFT_ARGS[$BRICCRAFT_I]}" == "--out" ]]; then
    BRICCRAFT_SITE="${BRICCRAFT_ARGS[$((BRICCRAFT_I+1))]}"
  fi
done
bash "$BRICCRAFT_ROOT/FerroUI/scripts/build-browser.sh" briccraft-browser --out "$BRICCRAFT_SITE" "$@"
bash "$BRICCRAFT_ROOT/scripts/build-import-worker.sh" "$BRICCRAFT_SITE"
