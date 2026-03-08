#!/usr/bin/env bash
set -euo pipefail

nexus_resolve_kb_ingest_password() {
  local host="${POSTGRES_HOST:-127.0.0.1}"
  local port="${POSTGRES_PORT:-5432}"
  local db="${POSTGRES_DB:-knowledge_base}"
  local user="${POSTGRES_INGEST_USER:-kb_ingest}"

  if ! command -v psql >/dev/null 2>&1; then
    return 1
  fi

  local -a candidates=()
  [[ -n "${KB_INGEST_PASSWORD:-}" ]] && candidates+=("$KB_INGEST_PASSWORD")
  [[ -n "${NEXUS_KB_INGEST_PASSWORD:-}" ]] && candidates+=("$NEXUS_KB_INGEST_PASSWORD")
  candidates+=(
    "kb_ingest_copy_local"
    "KbIngest2026seCCKDS88448cure"
    "123"
  )

  local seen='|'
  local cand
  for cand in "${candidates[@]}"; do
    [[ -z "$cand" ]] && continue
    if [[ "$seen" == *"|$cand|"* ]]; then
      continue
    fi
    seen+="$cand|"

    if PGPASSWORD="$cand" psql -h "$host" -p "$port" -U "$user" -d "$db" -tAc "SELECT 1" >/dev/null 2>&1; then
      export KB_INGEST_PASSWORD="$cand"
      return 0
    fi
  done

  return 1
}
