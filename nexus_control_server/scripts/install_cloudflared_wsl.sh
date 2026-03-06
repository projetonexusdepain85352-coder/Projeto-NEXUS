#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
BIN_DIR="$ROOT_DIR/nexus_control_server/bin"
CF_BIN="$BIN_DIR/cloudflared"

mkdir -p "$BIN_DIR"

if command -v cloudflared >/dev/null 2>&1; then
  echo "cloudflared already available at $(command -v cloudflared)"
  exit 0
fi

if [[ ! -x "$CF_BIN" ]]; then
  echo "Downloading cloudflared to $CF_BIN"
  curl -fsSL "https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64" -o "$CF_BIN"
  chmod +x "$CF_BIN"
fi

echo "cloudflared installed at: $CF_BIN"
echo "Add to PATH when needed: export PATH=\"$BIN_DIR:\$PATH\""
