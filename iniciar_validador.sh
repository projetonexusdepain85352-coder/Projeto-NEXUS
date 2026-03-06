#!/usr/bin/env bash
set -euo pipefail

SOCKET="/tmp/nexus_sugestor.sock"
SERVIDOR_PID=""
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="${NEXUS_PROJECT_ROOT:-$SCRIPT_DIR}"

if [[ -f "$PROJECT_ROOT/scripts/run_copy_env.sh" ]]; then
    # shellcheck source=/dev/null
    source "$PROJECT_ROOT/scripts/run_copy_env.sh" >/dev/null
fi

if [[ -f "$PROJECT_ROOT/scripts/ensure_permissions.sh" ]]; then
    # shellcheck source=/dev/null
    source "$PROJECT_ROOT/scripts/ensure_permissions.sh"
fi

cleanup() {
    echo ""
    echo "[NEXUS] Encerrando sugestor..."
    if [[ -n "$SERVIDOR_PID" ]]; then
        kill "$SERVIDOR_PID" 2>/dev/null || true
    fi
    if [[ -f "$SOCKET" ]]; then
        rm -f "$SOCKET"
    fi
}
trap cleanup EXIT

if ! declare -F nexus_pg_check_reader >/dev/null || ! declare -F nexus_pg_reapply_reader_grants >/dev/null; then
    echo "[ERRO] Funcoes de permissao nao encontradas (scripts/ensure_permissions.sh)."
    exit 1
fi

echo "[NEXUS] Verificando acesso do kb_reader..."
if ! nexus_pg_check_reader; then
    echo "[AVISO] kb_reader sem acesso. Tentando reaplicar grants..."
    if ! nexus_pg_reapply_reader_grants; then
        echo "[ERRO] Nao foi possivel reaplicar grants para kb_reader."
        echo "[ERRO] Defina KB_INGEST_PASSWORD e, se necessario, KB_ADMIN_PASSWORD."
        exit 1
    fi
    if ! nexus_pg_check_reader; then
        echo "[ERRO] kb_reader continua sem acesso apos tentativa de correcao."
        exit 1
    fi
fi

echo "[NEXUS] Iniciando sugestor..."
python3 "$PROJECT_ROOT/nexus_sugestor/servidor.py" > "$PROJECT_ROOT/nexus_sugestor/servidor.log" 2>&1 &
SERVIDOR_PID=$!

sleep 2
if [[ -S "$SOCKET" ]]; then
    echo "[NEXUS] Sugestor pronto!"
else
    echo "[AVISO] Sugestor nao iniciou. Validador rodara sem sugestao de IA."
fi

echo "[NEXUS] Abrindo validador..."
cd "$PROJECT_ROOT/validador"
export KB_INGEST_PASSWORD="${KB_INGEST_PASSWORD:-KbIngest2026seCCKDS88448cure}"
if [[ -x "./target/release/nexus_validador" ]]; then
    ./target/release/nexus_validador
elif [[ -x "./target/release/validador" ]]; then
    ./target/release/validador
else
    cargo run --release
fi

