#!/usr/bin/env bash
# Dev loop: rebuild WASM, serve locally.
# Use this for iterative backend work.
#
#   ./scripts/dev.sh         # rebuild, serve on 8000
#   ./scripts/dev.sh 9000    # rebuild, serve on 9000
#   ./scripts/dev.sh --skip-build   # just serve (no rebuild)

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

PORT=8000
SKIP_BUILD=0
for arg in "$@"; do
    case "$arg" in
        --skip-build) SKIP_BUILD=1 ;;
        [0-9]*) PORT="$arg" ;;
    esac
done

if [ "$SKIP_BUILD" -eq 0 ]; then
    ./scripts/build-wasm.sh
    echo ""
fi

echo "==> Open http://localhost:${PORT}/frontend/ in your browser"
./scripts/serve.sh "$PORT"
