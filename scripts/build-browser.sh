#!/usr/bin/env bash
set -euo pipefail
BRICCRAFT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export FERROUI_BROWSER_WORKSPACE="$BRICCRAFT_ROOT"
exec bash "$BRICCRAFT_ROOT/FerroUI/scripts/build-browser.sh" briccraft-browser "$@"
