#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT_DIR"

PORT="${NEXUS_CONTROL_PORT:-8787}"
HOST="${NEXUS_CONTROL_HOST:-127.0.0.1}"
TUNNEL_NAME="${NEXUS_TUNNEL_NAME:-nexus-control}"

if [[ -z "${NEXUS_TUNNEL_HOSTNAME:-}" ]]; then
  echo "Set NEXUS_TUNNEL_HOSTNAME (ex: nexus-control.seudominio.com)"
  exit 1
fi

if command -v cloudflared >/dev/null 2>&1; then
  CF_BIN="$(command -v cloudflared)"
else
  CF_BIN="$ROOT_DIR/src/nexus_control_server/bin/cloudflared"
fi

if [[ ! -x "$CF_BIN" ]]; then
  echo "cloudflared not found. Run install_cloudflared_wsl.sh first."
  exit 1
fi

mkdir -p "$ROOT_DIR/logs/control"
SERVER_LOG="$ROOT_DIR/logs/control/server.log"

cleanup() {
  if [[ -n "${TUNNEL_PID:-}" ]] && kill -0 "$TUNNEL_PID" >/dev/null 2>&1; then
    kill "$TUNNEL_PID" >/dev/null 2>&1 || true
  fi
  if [[ -n "${SERVER_PID:-}" ]] && kill -0 "$SERVER_PID" >/dev/null 2>&1; then
    kill "$SERVER_PID" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT INT TERM

python3 src/nexus_control_server/server.py >"$SERVER_LOG" 2>&1 &
SERVER_PID=$!

for _ in $(seq 1 30); do
  if curl -fsS "http://$HOST:$PORT/api/health" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

if ! curl -fsS "http://$HOST:$PORT/api/health" >/dev/null 2>&1; then
  echo "Server failed health check. See $SERVER_LOG"
  exit 1
fi

echo "Public URL: https://${NEXUS_TUNNEL_HOSTNAME}"
echo "Starting named tunnel: $TUNNEL_NAME"

"$CF_BIN" tunnel --config "$HOME/.cloudflared/config.yml" run "$TUNNEL_NAME" &
TUNNEL_PID=$!

wait "$TUNNEL_PID"
