#!/bin/bash

SOCKET="/tmp/nexus_sugestor.sock"
SERVIDOR_PID=""

cleanup() {
    echo ""
    echo "[NEXUS] Encerrando sugestor..."
    [ -n "$SERVIDOR_PID" ] && kill "$SERVIDOR_PID" 2>/dev/null
    [ -f "$SOCKET" ] && rm -f "$SOCKET"
}
trap cleanup EXIT

echo "[NEXUS] Iniciando sugestor..."
python3 ~/projeto/nexus_sugestor/servidor.py > ~/projeto/nexus_sugestor/servidor.log 2>&1 &
SERVIDOR_PID=$!

sleep 2
if [ -S "$SOCKET" ]; then
    echo "[NEXUS] Sugestor pronto!"
else
    echo "[AVISO] Sugestor nao iniciou. Validador rodara sem sugestao de IA."
fi

echo "[NEXUS] Abrindo validador..."
cd ~/projeto/validador
export KB_INGEST_PASSWORD='KbIngest2026seCCKDS88448cure'
./target/release/nexus_validador
