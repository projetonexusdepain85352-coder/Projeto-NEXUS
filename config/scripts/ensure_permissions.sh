#!/usr/bin/env bash
set -euo pipefail

nexus_pg_check_reader() {
  local host="${POSTGRES_HOST:-localhost}"
  local port="${POSTGRES_PORT:-5433}"
  local db="${POSTGRES_DB:-knowledge_base}"

  if [[ -z "${KB_READER_PASSWORD:-}" ]]; then
    echo "[ERRO] KB_READER_PASSWORD nao definida para teste de permissao." >&2
    return 1
  fi

  PGPASSWORD="$KB_READER_PASSWORD" psql \
    -h "$host" -p "$port" -U kb_reader -d "$db" \
    -tAc "SELECT 1" >/dev/null 2>&1
}

nexus_pg_reapply_reader_grants() {
  local host="${POSTGRES_HOST:-localhost}"
  local port="${POSTGRES_PORT:-5433}"
  local db="${POSTGRES_DB:-knowledge_base}"
  local sql="GRANT USAGE ON SCHEMA public TO kb_reader; GRANT SELECT ON ALL TABLES IN SCHEMA public TO kb_reader; ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO kb_reader;"

  if [[ -n "${KB_INGEST_PASSWORD:-}" ]]; then
    if PGPASSWORD="$KB_INGEST_PASSWORD" psql -v ON_ERROR_STOP=1 \
      -h "$host" -p "$port" -U kb_ingest -d "$db" -c "$sql" >/dev/null 2>&1; then
      echo "[NEXUS] Grants reaplicados via kb_ingest."
      return 0
    fi
    echo "[AVISO] kb_ingest nao conseguiu reaplicar grants (permissao insuficiente ou conexao)." >&2
  fi

  if [[ -n "${KB_ADMIN_PASSWORD:-}" ]]; then
    if PGPASSWORD="$KB_ADMIN_PASSWORD" psql -v ON_ERROR_STOP=1 \
      -h "$host" -p "$port" -U kb_admin -d "$db" -c "$sql" >/dev/null 2>&1; then
      echo "[NEXUS] Grants reaplicados via kb_admin."
      return 0
    fi
    echo "[AVISO] kb_admin tambem falhou ao reaplicar grants." >&2
  fi

  return 1
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  if nexus_pg_check_reader; then
    echo "[NEXUS] kb_reader OK"
    exit 0
  fi

  echo "[NEXUS] kb_reader sem acesso. Tentando corrigir grants..."
  nexus_pg_reapply_reader_grants
  nexus_pg_check_reader
  echo "[NEXUS] kb_reader restaurado"
fi
