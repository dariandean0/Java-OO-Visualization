#!/usr/bin/env bash
# Serve the frontend locally.
# The frontend imports '../../wasm/backend.js' from frontend/javascript/visualizer.js,
# so the server MUST be rooted at the repo root, not at frontend/.
# Then open http://localhost:$PORT/frontend/ in a browser.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

PORT="${1:-8000}"

if [ ! -f wasm/backend.js ] || [ ! -f wasm/backend.wasm ]; then
    echo "ERROR: wasm/backend.{js,wasm} not found."
    echo "Run: ./scripts/build-wasm.sh"
    exit 1
fi

# WASM MIME type must be application/wasm or browsers refuse to stream-compile.
# Python 3.11+ http.server sets it correctly; older versions do not.
python3 -c '
import http.server, socketserver, sys
port = int(sys.argv[1])

class H(http.server.SimpleHTTPRequestHandler):
    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        ".wasm": "application/wasm",
        ".js":   "application/javascript",
        ".mjs":  "application/javascript",
    }
    def end_headers(self):
        # Required so cross-origin isolation works if the app ever needs it.
        # Also disables aggressive browser caching during dev.
        self.send_header("Cache-Control", "no-store")
        super().end_headers()

with socketserver.TCPServer(("", port), H) as httpd:
    print(f"Serving {sys.argv[2]} on http://localhost:{port}/frontend/")
    print("Press Ctrl+C to stop.")
    httpd.serve_forever()
' "$PORT" "$REPO_ROOT"
