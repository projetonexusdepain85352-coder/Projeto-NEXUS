# Control Server

Painel web para controlar servicos do NEXUS remotamente via navegador, com autenticacao Google e terminal interativo por servico.

## Base URL

- Local: `http://127.0.0.1:8787`

## Autenticacao

- Rotas protegidas exigem `Authorization: Bearer <token>`.
- Token obtido via `POST /api/auth/google`.
- Sessao expira em ~12h (SESSION_TTL_SECONDS) e pode ser vinculada a IP/UA.
- Limite por email: 5 sessoes.

### Fluxo OAuth (Google)

1. Cliente chama `GET /api/auth/config`.
2. Se `google_enabled=true`, o frontend obtem `id_token` no Google.
3. Cliente envia `POST /api/auth/google` com JSON `{ "id_token": "..." }`.
4. API retorna `{ token, email, expires_at }`.
5. Use o token como Bearer nas rotas protegidas.

Variaveis relevantes:
- `NEXUS_GOOGLE_CLIENT_ID`
- `NEXUS_GOOGLE_ALLOWED_EMAILS` (lista separada por virgula)
- `NEXUS_SESSION_BIND_CONTEXT` (default ativo)

## Endpoints

Publicos:
- `GET /api/health`
- `GET /api/auth/config`
- `POST /api/auth/google`
- `POST /api/auth/logout`
- `GET /metrics`
- `GET /health`
- `GET /health/ready`

Protegidos (Bearer + scopes):
- `GET /api/services`
- `POST /api/services/{name}/start`
- `POST /api/services/{name}/stop`
- `POST /api/services/{name}/stdin`
- `GET /api/terminal/{name}?chars=12000`
- `GET /api/logs/{name}?lines=200`

## Erros comuns

- `POST /api/auth/google`: 400 `id_token ausente`; 401 `token Google invalido`; 403 `acesso nao autorizado`; 429 `muitas tentativas de autenticacao`; 503 `autenticacao Google desabilitada`.
- `POST /api/services/{name}/start`: 400 `nome de servico invalido`; 404 `servico nao encontrado`; 429 `muitas requisicoes de controle`.
- `POST /api/services/{name}/stdin`: 400 `entrada excede limite` ou `campo input deve ser string`; 403 `servico nao aceita entrada interativa`.

### Exemplo /api/health

```
{"status":"ok","time":"2026-03-12T12:00:00Z"}
```

### Exemplo /api/services

```
{
  "services": [
    {
      "name":"nexus_rag",
      "running":true,
      "pid":1234,
      "command":["cargo","run","-p","nexus_rag"],
      "cwd":"/path",
      "interactive":false,
      "stdin_available":false,
      "log_file":"/logs/control/nexus_rag.log",
      "started_at":"2026-03-12T12:00:00Z"
    }
  ]
}
```

## Rate limit

- Auth geral: 240 req/60s por IP e scope.
- `/api/auth/google`: 20 req/5min por IP.
- `/api/logs` e `/api/terminal`: 120 req/60s por IP.
- Mutacoes de servico: 60 req/60s por IP.

## Headers de seguranca

Em todas as respostas:
- Content-Security-Policy (CSP)
- X-Frame-Options: DENY
- Cross-Origin-Opener-Policy: same-origin-allow-popups
- Strict-Transport-Security (apenas em `NEXUS_ENV=production`)

## Metrics e health

- `GET /metrics`: Prometheus text/plain com counters:
  - `nexus_documents_ingested_total`
  - `nexus_documents_validated_total{result="approved|rejected"}`
  - `nexus_rag_queries_total{result="found|denied|below_threshold"}`
  - `nexus_models_trained_total{result="approved|rejected"}`
  - `nexus_http_requests_total{method,path,status}`
- `GET /health`: ok simples
- `GET /health/ready`: inclui checks (postgres/qdrant)

## Estrutura de arquivos

- `src/nexus_control_server/server.py`: backend HTTP + auth + API.
- `src/nexus_control_server/frontend/`: UI web.
- `src/nexus_control_server/services.json`: catalogo de servicos.
- `config/scripts/nexus_start.sh` e `config/scripts/nexus_ctl.sh`: operacao.
- `config/scripts/resolve_kb_ingest_password.sh`: helper para senha do kb_ingest.
- `config/scripts/run_copy_env.sh`: prepara variaveis de ambiente da copia.
- `logs/control/*.log`: logs por servico.
- `src/nexus_control_server/.env.worker`: token do Worker (Cloudflare).
- `src/nexus_control_server/nexus.log`: log do launcher.

## Requisitos

- WSL2 ativo
- Python 3 no WSL
- `cloudflared` (opcional, para acesso remoto). Caminho esperado: `src/nexus_control_server/bin/cloudflared`
- Docker Desktop (se os servicos dependem de containers)
- `CF_API_TOKEN` valido em `src/nexus_control_server/.env.worker`

## Variaveis de ambiente (core)

- `NEXUS_CONTROL_HOST`, `NEXUS_CONTROL_PORT`
- `NEXUS_GOOGLE_CLIENT_ID`
- `NEXUS_GOOGLE_ALLOWED_EMAILS`
- `NEXUS_SESSION_BIND_CONTEXT`

## Operacao (nexus_ctl)

```
bash config/scripts/nexus_ctl.sh status
bash config/scripts/nexus_ctl.sh start
bash config/scripts/nexus_ctl.sh stop
bash config/scripts/nexus_ctl.sh restart
```

## services.json

Campos tipicos por servico:

- `command`: comando a executar.
- `cwd`: diretorio de execucao.
- `interactive`: `true|false`.
- `env`: variaveis adicionais.

Servicos interativos aceitam input remoto via endpoint `/stdin`.

## Troubleshooting

- Erro 1016/530 no site: worker apontando para tunnel expirado -> `nexus_ctl.sh restart`.
- Login Google nao abre popup: validar `NEXUS_GOOGLE_CLIENT_ID`, allowlist e CSP.
- Start falha: checar `logs/control/<servico>.log`.
- `stdin` indisponivel: parar e iniciar novamente via painel.

## Limites atuais

- Push Git automatico pode falhar no Windows por problema de credencial TLS (schannel).
- `/api/logs` e endpoint legado; UI atual usa `Servicos + Terminal`.
