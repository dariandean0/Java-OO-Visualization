#!/usr/bin/env bash
# Build the backend to WASM for the frontend.
# Uses nix-shell to provide emscripten + rustup on NixOS.
# Mirrors the flags used by .github/workflows/build-wasm.yml.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "==> Building WASM from backend/"
nix-shell -p emscripten rustup --run '
set -e
cd backend

rustup target add wasm32-unknown-emscripten 2>&1 | tail -1

# NixOS emscripten: the default cache is in the read-only nix store.
# Point it to a writable user-local directory.
export EM_CACHE="$HOME/.cache/emscripten"
mkdir -p "$EM_CACHE"

# Tell the cc crate to invoke emcc/emar when cross-compiling to
# wasm32-unknown-emscripten (instead of the default system cc).
export CC_wasm32_unknown_emscripten=emcc
export AR_wasm32_unknown_emscripten=emar
export CFLAGS_wasm32_unknown_emscripten="-fPIC"

export RUSTFLAGS="-C link-args=-sMODULARIZE=1 \
-C link-args=-sSTANDALONE_WASM=0 \
-C link-args=-sEXPORTED_FUNCTIONS=[\"_wasm_execution_flow_gen\",\"_wasm_no_flow_gen\",\"_wasm_visualize_java_code\"] \
-C link-args=-sEXPORTED_RUNTIME_METHODS=[\"cwrap\",\"UTF8ToString\",\"stringToUTF8\",\"lengthBytesUTF8\"] \
-C link-args=-sWASM=1 \
-C link-args=-sEXPORT_ES6=1 \
-C link-args=-o \
-C link-args=backend.js \
-C opt-level=s"

cargo build --lib --target wasm32-unknown-emscripten --release
'

echo "==> Copying build output into wasm/"
mkdir -p wasm
cp backend/backend.js backend/backend.wasm wasm/
rm -f backend/backend.js backend/backend.wasm

echo "==> Done: $(ls -la wasm/)"
