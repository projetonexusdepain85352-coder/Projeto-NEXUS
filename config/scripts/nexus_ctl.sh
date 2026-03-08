#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
CONTROL_DIR="$ROOT_DIR/src/nexus_control_server"
START_SCRIPT="$ROOT_DIR/config/scripts/nexus_start.sh"
LOG_FILE="$CONTROL_DIR/nexus.log"
PID_FILE="$CONTROL_DIR/.nexus_start.pid"
WORKER_URL="https://nexus-control.projeton-e-x-u-sdepain85352.workers.dev"

SERVER_PATTERN="python3 src/nexus_control_server/server.py"
TUNNEL_PATTERN="src/nexus_control_server/bin/cloudflared tunnel --url http://127.0.0.1:8787 --no-autoupdate"

find_server_pid() {
  pgrep -f "$SERVER_PATTERN" | head -n1 || true
}

find_tunnel_pid() {
  pgrep -f "$TUNNEL_PATTERN" | head -n1 || true
}

print_status() {
  local server_pid tunnel_pid
  server_pid="$(find_server_pid)"
  tunnel_pid="$(find_tunnel_pid)"

  echo "NEXUS Control Server status"
  echo "Worker URL : $WORKER_URL"
  if [[ -n "$server_pid" ]]; then
    echo "Server PID : $server_pid (running)"
  else
    echo "Server PID : not running"
  fi

  if [[ -n "$tunnel_pid" ]]; then
    echo "Tunnel PID : $tunnel_pid (running)"
  else
    echo "Tunnel PID : not running"
  fi

  if [[ -f "$PID_FILE" ]]; then
    echo "Launcher PID file: $PID_FILE ($(cat "$PID_FILE" 2>/dev/null || echo "n/a"))"
  else
    echo "Launcher PID file: not found"
  fi

  if [[ -f "$LOG_FILE" ]]; then
    local tunnel_url
    tunnel_url="$(grep -Eo 'https://[-a-z0-9]+\.trycloudflare\.com' "$LOG_FILE" | tail -n1 || true)"
    if [[ -n "$tunnel_url" ]]; then
      echo "Last quick tunnel URL: $tunnel_url"
    fi
    echo
    echo "Last log lines:"
    tail -n 15 "$LOG_FILE" || true
  else
    echo "Log file not found: $LOG_FILE"
  fi
}

stop_server() {
  local launcher_pid server_pid tunnel_pid

  if [[ -f "$PID_FILE" ]]; then
    launcher_pid="$(cat "$PID_FILE" 2>/dev/null || true)"
    if [[ -n "${launcher_pid:-}" ]] && kill -0 "$launcher_pid" >/dev/null 2>&1; then
      kill "$launcher_pid" >/dev/null 2>&1 || true
      sleep 2
    fi
    rm -f "$PID_FILE"
  fi

  server_pid="$(find_server_pid)"
  if [[ -n "$server_pid" ]]; then
    kill "$server_pid" >/dev/null 2>&1 || true
  fi

  tunnel_pid="$(find_tunnel_pid)"
  if [[ -n "$tunnel_pid" ]]; then
    kill "$tunnel_pid" >/dev/null 2>&1 || true
  fi

  pkill -f "$SERVER_PATTERN" >/dev/null 2>&1 || true
  pkill -f "$TUNNEL_PATTERN" >/dev/null 2>&1 || true

  sleep 1
  echo "NEXUS Control Server stopped."
}

start_server() {
  local server_pid tunnel_pid
  server_pid="$(find_server_pid)"
  tunnel_pid="$(find_tunnel_pid)"

  if [[ -n "$server_pid" ]] && [[ -n "$tunnel_pid" ]]; then
    echo "NEXUS Control Server is already running."
    print_status
    return 0
  fi

  if [[ ! -x "$START_SCRIPT" ]]; then
    chmod +x "$START_SCRIPT"
  fi

  mkdir -p "$CONTROL_DIR"
  nohup bash "$START_SCRIPT" > "$LOG_FILE" 2>&1 &
  echo $! > "$PID_FILE"

  sleep 8
  echo "NEXUS Control Server started."
  print_status
}

restart_server() {
  stop_server
  start_server
}

usage() {
  cat <<EOF
Usage: $(basename "$0") <status|start|stop|restart>
EOF
}

main() {
  if [[ $# -ne 1 ]]; then
    usage
    exit 1
  fi

  case "$1" in
    status) print_status ;;
    start) start_server ;;
    stop) stop_server ;;
    restart) restart_server ;;
    *)
      usage
      exit 1
      ;;
  esac
}

main "$@"