#!/usr/bin/env bash
# Native compilation and raster shader checks need libraries, not an X server.
# Check capabilities first: a complete runner must not depend on an apt mirror.
set -euo pipefail

has_library() {
    python3 - "$1" <<'PY'
import ctypes
import sys

try:
    ctypes.CDLL(sys.argv[1])
except OSError:
    sys.exit(1)
PY
}

missing=()
if ! command -v pkg-config >/dev/null 2>&1; then
    missing+=(pkg-config libfontconfig-dev libfreetype-dev)
else
    pkg-config --exists fontconfig || missing+=(libfontconfig-dev)
    pkg-config --exists freetype2 || missing+=(libfreetype-dev)
fi
has_library libEGL.so.1 || missing+=(libegl1)
has_library libGL.so.1 || missing+=(libgl1)
python3 -c 'import ensurepip, venv' >/dev/null 2>&1 || missing+=(python3-venv)

if (( ${#missing[@]} )); then
    printf 'Missing native capabilities; installing: %s\n' "${missing[*]}"
    # Fail clearly on a mirror/DNS outage instead of spending ten minutes on
    # every archive. Package signatures and the runner's sources stay intact.
    sudo timeout --kill-after=10s 120s apt-get \
        -o Acquire::Retries=1 \
        -o Acquire::http::Timeout=15 \
        -o Acquire::https::Timeout=15 \
        -o APT::Update::Error-Mode=any update
    sudo timeout --kill-after=10s 180s apt-get \
        -o Acquire::Retries=1 \
        -o Acquire::http::Timeout=15 \
        -o Acquire::https::Timeout=15 \
        install -y --no-install-recommends "${missing[@]}"
fi

pkg-config --modversion fontconfig freetype2
has_library libEGL.so.1
has_library libGL.so.1
python3 -c 'import ensurepip, venv'
