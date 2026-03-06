#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT_DIR"

PORT="${NEXUS_CONTROL_PORT:-8787}"
HOST="${NEXUS_CONTROL_HOST:-127.0.0.1}"

if [[ -z "${NEXUS_CONTROL_TOKEN:-}" && -z "${NEXUS_GOOGLE_CLIENT_ID:-}" ]]; then
  echo "Set NEXUS_CONTROL_TOKEN and/or NEXUS_GOOGLE_CLIENT_ID before starting."
  exit 1
fi

if command -v cloudflared >/dev/null 2>&1; then
  CF_BIN="$(command -v cloudflared)"
else
  "$ROOT_DIR/nexus_control_server/scripts/install_cloudflared_wsl.sh"
  CF_BIN="$ROOT_DIR/nexus_control_server/bin/cloudflared"
fi

mkdir -p "$ROOT_DIR/logs/control"
SERVER_LOG="$ROOT_DIR/logs/control/server.log"
TUNNEL_LOG="$ROOT_DIR/logs/control/tunnel_quick.log"

cleanup() {
  if [[ -n "${TUNNEL_PID:-}" ]] && kill -0 "$TUNNEL_PID" >/dev/null 2>&1; then
    kill "$TUNNEL_PID" >/dev/null 2>&1 || true
  fi
  if [[ -n "${SERVER_PID:-}" ]] && kill -0 "$SERVER_PID" >/dev/null 2>&1; then
    kill "$SERVER_PID" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT INT TERM

python3 nexus_control_server/server.py >"$SERVER_LOG" 2>&1 &
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

: >"$TUNNEL_LOG"
"$CF_BIN" tunnel --url "http://$HOST:$PORT" --no-autoupdate >"$TUNNEL_LOG" 2>&1 &
TUNNEL_PID=$!

PUBLIC_URL=""
for _ in $(seq 1 60); do
  PUBLIC_URL="$(grep -Eo 'https://[-a-z0-9]+\.trycloudflare\.com' "$TUNNEL_LOG" | head -n1 || true)"
  if [[ -n "$PUBLIC_URL" ]]; then
    break
  fi
  sleep 1
done

echo ""
echo "NEXUS Control online"
echo "Local : http://$HOST:$PORT"
if [[ -n "$PUBLIC_URL" ]]; then
  echo "Public: $PUBLIC_URL"
else
  echo "Public URL not detected yet. Check log: $TUNNEL_LOG"
fi

echo ""
echo "Keep this terminal running. Ctrl+C to stop server+tunnel."

wait "$TUNNEL_PID"
