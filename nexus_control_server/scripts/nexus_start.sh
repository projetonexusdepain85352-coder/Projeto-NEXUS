#!/bin/bash
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
ENV_FILE="$ROOT_DIR/nexus_control_server/.env.worker"
CLOUDFLARED="$ROOT_DIR/nexus_control_server/bin/cloudflared"
CF_ACCOUNT_ID="8146df12c15dd29fd27507e418785c3d"
CF_WORKER_NAME="nexus-control"
CF_WORKER_URL="https://nexus-control.projeton-e-x-u-sdepain85352.workers.dev"

source "$ENV_FILE"
export NEXUS_GOOGLE_CLIENT_ID="126514381182-9qk4j14vnmhe3c4btmkkp8boml7t05b2.apps.googleusercontent.com"
export NEXUS_GOOGLE_ALLOWED_EMAILS="dulandin44@gmail.com,evagpt0@gmail.com,nerdalienx@gmail.com,superpurorits@gmail.com,eduardo.landin@eaportal.org,henriquethedodoro777@gmail.com,hareklevit@gmail.com,igparera777@gmail.com,intraotsu@gmail.com,projeton.e.x.u.sdepain85352@gmail.com"
export NEXUS_CONTROL_TOKEN=123

echo "[1/3] Iniciando servidor..."
pkill -f "nexus_control_server/server.py" 2>/dev/null; sleep 1
cd "$ROOT_DIR"
python3 nexus_control_server/server.py &
SERVER_PID=$!
for i in $(seq 1 10); do curl -s http://127.0.0.1:8787 > /dev/null 2>&1 && break; sleep 1; done
echo "[1/3] ✅ Servidor OK"

echo "[2/3] Iniciando tunnel..."
pkill -f cloudflared 2>/dev/null; sleep 1
TUNNEL_LOG=$(mktemp)
"$CLOUDFLARED" tunnel --url http://127.0.0.1:8787 --no-autoupdate > "$TUNNEL_LOG" 2>&1 &
TUNNEL_PID=$!
TUNNEL_URL=""
for i in $(seq 1 30); do
  TUNNEL_URL=$(grep -o 'https://[a-z0-9-]*\.trycloudflare\.com' "$TUNNEL_LOG" 2>/dev/null | head -1)
  [ -n "$TUNNEL_URL" ] && break; sleep 1
done
echo "[2/3] ✅ Tunnel: $TUNNEL_URL"

echo "[3/3] Atualizando Worker..."
WORKER_SCRIPT="addEventListener(\"fetch\", e => { const u = new URL(e.request.url); e.respondWith(fetch(\"${TUNNEL_URL}\" + u.pathname + u.search, {method: e.request.method, headers: e.request.headers, body: [\"GET\",\"HEAD\"].includes(e.request.method)?null:e.request.body})); });"
curl -s "https://api.cloudflare.com/client/v4/accounts/${CF_ACCOUNT_ID}/workers/scripts/${CF_WORKER_NAME}" \
  -X PUT -H "Authorization: Bearer ${CF_API_TOKEN}" -H "Content-Type: application/javascript" \
  --data-binary "$WORKER_SCRIPT" | python3 -c "import sys,json; r=json.load(sys.stdin); print('[3/3] ✅ Worker atualizado!' if r.get('success') else f'❌ Erro: {r}')"

echo ""
echo "══════════════════════════════════════"
echo "  NEXUS ONLINE ✅"
echo "  URL FIXA: $CF_WORKER_URL"
echo "  (Google OAuth configurado uma vez só)"
echo "══════════════════════════════════════"

cleanup() { kill $SERVER_PID $TUNNEL_PID 2>/dev/null; rm -f "$TUNNEL_LOG"; exit 0; }
trap cleanup SIGINT SIGTERM
wait $SERVER_PID
