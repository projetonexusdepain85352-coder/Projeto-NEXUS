# NEXUS Control Server REST API

## Base URL
http://127.0.0.1:8787

## Autenticacao
- Todas as rotas protegidas exigem `Authorization: Bearer <token>`.
- O token e obtido via `POST /api/auth/google`.
- Sessao expira em ~12h (SESSION_TTL_SECONDS) e pode ser vinculada a IP/UA.

## Fluxo OAuth (Google)
1. Cliente chama `GET /api/auth/config`.
2. Se `google_enabled=true`, o frontend obtem `id_token` no Google.
3. Cliente envia `POST /api/auth/google` com JSON `{ "id_token": "..." }`.
4. API retorna `{ token, email, expires_at }`.
5. Use o token como Bearer nas rotas protegidas.

## Endpoints

### GET /api/health
Resposta: 200
```json
{"status":"ok","time":"2026-03-12T12:00:00Z"}
```

### GET /api/auth/config
Resposta: 200
```json
{"google_enabled":true,"google_client_id":"..."}
```

### POST /api/auth/google
Auth: nao requer token.
Request (JSON):
```json
{"id_token":"..."}
```
Resposta 200:
```json
{"token":"...","email":"usuario@exemplo.com","expires_at":1700000000}
```
Erros:
- 400 `id_token ausente`
- 401 `token Google invalido`
- 403 `acesso nao autorizado`
- 429 `muitas tentativas de autenticacao`
- 503 `autenticacao Google desabilitada`

### POST /api/auth/logout
Auth: Bearer (opcional).
Resposta 200:
```json
{"ok":true}
```

### GET /api/services
Auth: Bearer (scope services:list).
Resposta 200:
```json
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

### POST /api/services/{name}/start
Auth: Bearer (scope services:mutate).
Resposta 200:
```json
{"name":"nexus_rag","running":true,"pid":1234,"already_running":false,"interactive":false}
```
Erros:
- 400 `nome de servico invalido`
- 404 `servico nao encontrado`
- 429 `muitas requisicoes de controle`

### POST /api/services/{name}/stop
Auth: Bearer (scope services:mutate).
Resposta 200:
```json
{"name":"nexus_rag","running":false,"stopped":true}
```

### POST /api/services/{name}/stdin
Auth: Bearer (scope services:mutate).
Request (JSON):
```json
{"input":"comando","append_newline":true}
```
Resposta 200:
```json
{"name":"nexus_validador","running":true,"accepted":true,"sent_bytes":8}
```
Erros comuns:
- 400 `entrada excede limite` ou `campo 'input' deve ser string`
- 403 `servico nao aceita entrada interativa`

### GET /api/logs/{name}?lines=200
Auth: Bearer (scope logs:read).
Resposta 200:
```json
{"name":"nexus_rag","logs":"..."}
```

### GET /api/terminal/{name}?chars=12000
Auth: Bearer (scope terminal:read).
Resposta 200:
```json
{"name":"nexus_validador","output":"..."}
```

## Rate limit
- Auth geral: 240 req/60s por IP e scope.
- /api/auth/google: 20 req/5min por IP.
- /api/logs e /api/terminal: 120 req/60s por IP.
- Mutacoes de servico: 60 req/60s por IP.

## Headers de seguranca
Em todas as respostas:
- Content-Security-Policy (CSP)
- X-Frame-Options: DENY
- Cross-Origin-Opener-Policy: same-origin-allow-popups
- Strict-Transport-Security (apenas em NEXUS_ENV=production)

## Exemplos curl
```bash
curl -s http://127.0.0.1:8787/api/health

curl -s http://127.0.0.1:8787/api/auth/config

curl -s -X POST http://127.0.0.1:8787/api/auth/google \
  -H "Content-Type: application/json" \
  -d '{"id_token":"..."}'

curl -s http://127.0.0.1:8787/api/services \
  -H "Authorization: Bearer $TOKEN"
```
