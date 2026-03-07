#!/usr/bin/env bash
# NEXUS Worker Setup - Roda UMA VEZ para criar o Worker fixo no Cloudflare
# Uso: bash nexus_control_server/scripts/setup_worker.sh

set -euo pipefail

CF_ACCOUNT_ID="8146df12c15dd29fd27507e418785c3d"
WORKER_NAME="nexus-control"

echo ""
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║           NEXUS Worker Setup (execução única)               ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""
echo "Você precisa de um Cloudflare API Token com permissão de Worker."
echo ""
echo "Crie em: https://dash.cloudflare.com/profile/api-tokens"
echo "  → Criar token → Editar Workers (template pronto)"
echo "  → Escopo de conta: Workers Scripts - Editar"
echo "  → Criar token"
echo ""
read -rp "Cole o API Token aqui: " CF_API_TOKEN
echo ""

# Salva o token para uso futuro
ENV_FILE="$(dirname "$0")/../.env.worker"
echo "CF_API_TOKEN=$CF_API_TOKEN" > "$ENV_FILE"
echo "CF_ACCOUNT_ID=$CF_ACCOUNT_ID" >> "$ENV_FILE"
echo "WORKER_NAME=$WORKER_NAME" >> "$ENV_FILE"
echo "[OK] Token salvo em nexus_control_server/.env.worker"

# Descobre o subdomínio workers.dev da conta
echo ""
echo "Descobrindo subdomínio workers.dev..."
SUBDOMAIN_RESP=$(curl -s "https://api.cloudflare.com/client/v4/accounts/$CF_ACCOUNT_ID/workers/subdomain" \
  -H "Authorization: Bearer $CF_API_TOKEN")

SUBDOMAIN=$(echo "$SUBDOMAIN_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['result']['subdomain'])" 2>/dev/null || true)

if [[ -z "$SUBDOMAIN" ]]; then
  echo "Subdomínio workers.dev ainda não configurado. Criando..."
  # Cloudflare cria automaticamente na primeira publicação — continuamos
  WORKER_URL="https://$WORKER_NAME.$CF_ACCOUNT_ID.workers.dev"
else
  WORKER_URL="https://$WORKER_NAME.$SUBDOMAIN.workers.dev"
  echo "CF_WORKER_URL=$WORKER_URL" >> "$ENV_FILE"
fi

# Cria o Worker com URL placeholder
WORKER_SCRIPT=$(cat <<'JSEOF'
const TUNNEL_URL = "http://127.0.0.1:8787";
export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    const target = (env.TUNNEL_URL || TUNNEL_URL) + url.pathname + url.search;
    try {
      const resp = await fetch(target, {
        method: request.method,
        headers: request.headers,
        body: ["GET","HEAD"].includes(request.method) ? null : request.body,
        redirect: "follow"
      });
      return resp;
    } catch(e) {
      return new Response("NEXUS Control offline: " + e.message, { status: 503 });
    }
  }
}
JSEOF
)

echo ""
echo "Publicando Worker '$WORKER_NAME'..."

# Publica via multipart (módulo ES)
RESPONSE=$(curl -s -X PUT \
  "https://api.cloudflare.com/client/v4/accounts/$CF_ACCOUNT_ID/workers/scripts/$WORKER_NAME" \
  -H "Authorization: Bearer $CF_API_TOKEN" \
  -F "metadata={\"main_module\":\"worker.js\",\"compatibility_date\":\"2024-01-01\"};type=application/json" \
  -F "worker.js=$WORKER_SCRIPT;type=application/javascript+module")

SUCCESS=$(echo "$RESPONSE" | python3 -c "import sys,json; print(json.load(sys.stdin)['success'])" 2>/dev/null || echo "false")

if [[ "$SUCCESS" != "True" ]]; then
  echo "ERRO ao publicar Worker:"
  echo "$RESPONSE" | python3 -m json.tool 2>/dev/null || echo "$RESPONSE"
  exit 1
fi

echo "[OK] Worker publicado!"

# Habilita o subdomínio workers.dev para o Worker
curl -s -X POST \
  "https://api.cloudflare.com/client/v4/accounts/$CF_ACCOUNT_ID/workers/scripts/$WORKER_NAME/subdomain" \
  -H "Authorization: Bearer $CF_API_TOKEN" \
  -H "Content-Type: application/json" \
  --data '{"enabled":true}' > /dev/null

# Busca subdomínio novamente
SUBDOMAIN_RESP=$(curl -s "https://api.cloudflare.com/client/v4/accounts/$CF_ACCOUNT_ID/workers/subdomain" \
  -H "Authorization: Bearer $CF_API_TOKEN")
SUBDOMAIN=$(echo "$SUBDOMAIN_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['result']['subdomain'])" 2>/dev/null || true)

if [[ -n "$SUBDOMAIN" ]]; then
  WORKER_URL="https://$WORKER_NAME.$SUBDOMAIN.workers.dev"
  # Atualiza .env.worker com URL final
  sed -i "s|^CF_WORKER_URL=.*|CF_WORKER_URL=$WORKER_URL|" "$ENV_FILE" 2>/dev/null || \
    echo "CF_WORKER_URL=$WORKER_URL" >> "$ENV_FILE"
fi

echo ""
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║                    SETUP CONCLUÍDO ✅                       ║"
echo "╠══════════════════════════════════════════════════════════════╣"
echo "║                                                              ║"
echo "║  URL FIXA do painel (use esta para sempre):                  ║"
echo "║  $WORKER_URL"
echo "║                                                              ║"
echo "║  PRÓXIMO PASSO — configure o Google OAuth UMA VEZ:           ║"
echo "║                                                              ║"
echo "║  1. Abra: https://console.cloud.google.com/apis/credentials ║"
echo "║     Projeto: NEXUS                                           ║"
echo "║  2. Clique em 'NEXUS server'                                 ║"
echo "║  3. Em Origens JS autorizadas: adicione a URL acima          ║"
echo "║  4. Em URIs de redirecionamento: adicione a mesma URL        ║"
echo "║  5. Salvar                                                   ║"
echo "║                                                              ║"
echo "║  Depois disso: use nexus_start.sh normalmente.              ║"
echo "║  NUNCA mais precisará atualizar o Google OAuth.             ║"
echo "║                                                              ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""
