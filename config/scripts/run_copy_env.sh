#!/usr/bin/env bash
set -euo pipefail

can_connect() {
  local host="$1"
  local port="$2"
  timeout 2 bash -c "cat < /dev/null > /dev/tcp/${host}/${port}" >/dev/null 2>&1
}

# Prefer Docker's stable alias only if it is reachable from this WSL session.
if [[ -n "${NEXUS_COPY_HOST:-}" ]]; then
  COPY_HOST="$NEXUS_COPY_HOST"
elif can_connect 127.0.0.1 5433; then
  COPY_HOST="127.0.0.1"
elif getent hosts host.docker.internal >/dev/null 2>&1 && can_connect host.docker.internal 5433; then
  COPY_HOST="host.docker.internal"
else
  COPY_HOST="$(ip route show default | cut -d' ' -f3)"
fi

export POSTGRES_HOST="${POSTGRES_HOST:-$COPY_HOST}"
export POSTGRES_PORT="${POSTGRES_PORT:-5433}"
export POSTGRES_DB="${POSTGRES_DB:-knowledge_base}"
export POSTGRES_USER="${POSTGRES_USER:-kb_reader}"
export KB_READER_PASSWORD="${KB_READER_PASSWORD:-kb_reader_copy_local}"
export KB_INGEST_PASSWORD="${KB_INGEST_PASSWORD:-kb_ingest_copy_local}"
export QDRANT_URL="${QDRANT_URL:-http://$COPY_HOST:6335}"
export NEXUS_ENV="${NEXUS_ENV:-development}"

echo "Copy env loaded:"
echo "  POSTGRES_HOST=$POSTGRES_HOST"
echo "  POSTGRES_PORT=$POSTGRES_PORT"
echo "  POSTGRES_DB=$POSTGRES_DB"
echo "  POSTGRES_USER=$POSTGRES_USER"
echo "  QDRANT_URL=$QDRANT_URL"
echo "  NEXUS_ENV=$NEXUS_ENV"

