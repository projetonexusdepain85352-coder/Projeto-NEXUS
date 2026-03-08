#!/usr/bin/env bash
# NEXUS Worker Setup - Roda UMA VEZ para criar o Worker fixo no Cloudflare
# Uso: bash config/scripts/setup_worker.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

CF_ACCOUNT_ID="8146df12c15dd29fd27507e418785c3d"
WORKER_NAME="nexus-control"

echo ""
echo "â•”â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•—"
echo "â•‘           NEXUS Worker Setup (execuÃ§Ã£o Ãºnica)               â•‘"
echo "â•šâ•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•"
echo ""
echo "VocÃª precisa de um Cloudflare API Token com permissÃ£o de Worker."
echo ""
echo "Crie em: https://dash.cloudflare.com/profile/api-tokens"
echo "  â†’ Criar token â†’ Editar Workers (template pronto)"
echo "  â†’ Escopo de conta: Workers Scripts - Editar"
echo "  â†’ Criar token"
echo ""
read -rp "Cole o API Token aqui: " CF_API_TOKEN
echo ""

# Salva o token para uso futuro
ENV_FILE="$ROOT_DIR/src/nexus_control_server/.env.worker"
echo "CF_API_TOKEN=$CF_API_TOKEN" > "$ENV_FILE"
echo "CF_ACCOUNT_ID=$CF_ACCOUNT_ID" >> "$ENV_FILE"
echo "WORKER_NAME=$WORKER_NAME" >> "$ENV_FILE"
echo "[OK] Token salvo em src/nexus_control_server/.env.worker"

# Descobre o subdomÃ­nio workers.dev da conta
echo ""
echo "Descobrindo subdomÃ­nio workers.dev..."
SUBDOMAIN_RESP=$(curl -s "https://api.cloudflare.com/client/v4/accounts/$CF_ACCOUNT_ID/workers/subdomain" \
  -H "Authorization: Bearer $CF_API_TOKEN")

SUBDOMAIN=$(echo "$SUBDOMAIN_RESP" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['result']['subdomain'])" 2>/dev/null || true)

if [[ -z "$SUBDOMAIN" ]]; then
  echo "SubdomÃ­nio workers.dev ainda nÃ£o configurado. Criando..."
  # Cloudflare cria automaticamente na primeira publicaÃ§Ã£o â€” continuamos
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

# Publica via multipart (mÃ³dulo ES)
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

# Habilita o subdomÃ­nio workers.dev para o Worker
curl -s -X POST \
  "https://api.cloudflare.com/client/v4/accounts/$CF_ACCOUNT_ID/workers/scripts/$WORKER_NAME/subdomain" \
  -H "Authorization: Bearer $CF_API_TOKEN" \
  -H "Content-Type: application/json" \
  --data '{"enabled":true}' > /dev/null

# Busca subdomÃ­nio novamente
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
echo "â•”â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•—"
echo "â•‘                    SETUP CONCLUÃDO âœ…                       â•‘"
echo "â• â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•£"
echo "â•‘                                                              â•‘"
echo "â•‘  URL FIXA do painel (use esta para sempre):                  â•‘"
echo "â•‘  $WORKER_URL"
echo "â•‘                                                              â•‘"
echo "â•‘  PRÃ“XIMO PASSO â€” configure o Google OAuth UMA VEZ:           â•‘"
echo "â•‘                                                              â•‘"
echo "â•‘  1. Abra: https://console.cloud.google.com/apis/credentials â•‘"
echo "â•‘     Projeto: NEXUS                                           â•‘"
echo "â•‘  2. Clique em 'NEXUS server'                                 â•‘"
echo "â•‘  3. Em Origens JS autorizadas: adicione a URL acima          â•‘"
echo "â•‘  4. Em URIs de redirecionamento: adicione a mesma URL        â•‘"
echo "â•‘  5. Salvar                                                   â•‘"
echo "â•‘                                                              â•‘"
echo "â•‘  Depois disso: use nexus_start.sh normalmente.              â•‘"
echo "â•‘  NUNCA mais precisarÃ¡ atualizar o Google OAuth.             â•‘"
echo "â•‘                                                              â•‘"
echo "â•šâ•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•"
echo ""
