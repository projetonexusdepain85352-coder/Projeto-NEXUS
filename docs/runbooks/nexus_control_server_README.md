# NEXUS Control Server

Painel web para controlar servicos do Projeto NEXUS remotamente via navegador, com autenticacao Google e terminal interativo por servico.

## 1. O que este servidor faz

- Exibe servicos configurados no painel.
- Inicia e encerra servicos (`start/stop`).
- Mantem sessao interativa de terminal por servico (TTY/PTy), para voce enviar teclas e comandos de interacao.
- Mostra a saida recente da sessao de terminal do servico selecionado.
- Publica a URL fixa via Cloudflare Worker apontando para o tunnel ativo.

## 2. Arquitetura operacional

Fluxo de acesso:

1. Browser -> URL fixa `workers.dev`
2. Cloudflare Worker -> URL atual do Quick Tunnel
3. Cloudflared Tunnel -> `127.0.0.1:8787`
4. `nexus_control_server/server.py`
5. ServiceManager -> processos locais (`validador`, `agente_intermediario`, etc.)

## 3. Estrutura de arquivos

- `nexus_control_server/server.py`: backend HTTP + autenticacao + API + gerenciamento de processos.
- `nexus_control_server/frontend/index.html`: UI do painel.
- `nexus_control_server/frontend/app.js`: logica do frontend (auth, servicos, terminal).
- `nexus_control_server/frontend/styles.css`: estilo visual.
- `nexus_control_server/services.json`: definicao dos servicos gerenciados.
- `nexus_control_server/scripts/nexus_start.sh`: sobe server + tunnel + atualiza worker.
- `nexus_control_server/scripts/nexus_ctl.sh`: comandos operacionais (`status/start/stop/restart`).
- `nexus_control_server/scripts/resolve_kb_ingest_password.sh`: tentativa automatica de resolver senha do `kb_ingest`.
- `nexus_control_server/.env.worker`: token de API da Cloudflare para publicar o Worker.
- `nexus_control_server/nexus.log`: log do launcher e do processo de publicacao.
- `logs/control/*.log`: logs por servico gerenciado.

## 4. Requisitos

- WSL2 ativo.
- Python 3 no WSL.
- `cloudflared` em `nexus_control_server/bin/cloudflared`.
- `curl` e `psql` no WSL.
- Docker Desktop (se os servicos dependem de containers).
- `CF_API_TOKEN` valido em `nexus_control_server/.env.worker`.

## 5. Autenticacao

Autenticacao ativa: Google OAuth + sessao de backend.

Variaveis relevantes:

- `NEXUS_GOOGLE_CLIENT_ID`
- `NEXUS_GOOGLE_ALLOWED_EMAILS` (lista separada por virgula)
- `NEXUS_SESSION_BIND_CONTEXT` (default ativo)

Sessao:

- TTL: 12 horas
- limite por email: 5 sessoes
- validacao opcional por IP + User-Agent hash

## 6. Comandos de gerenciamento do servidor

Use sempre a partir da raiz do repo no WSL:

```bash
cd /mnt/c/Users/dulan/OneDrive/Documentos/GitHub/Projeto-NEXUS
```

### 6.1 Status

```bash
bash nexus_control_server/scripts/nexus_ctl.sh status
```

Mostra:

- PID do backend
- PID do tunnel
- URL fixa do worker
- ultima URL do quick tunnel
- ultimas linhas de log

### 6.2 Start

```bash
bash nexus_control_server/scripts/nexus_ctl.sh start
```

### 6.3 Stop

```bash
bash nexus_control_server/scripts/nexus_ctl.sh stop
```

### 6.4 Restart

```bash
bash nexus_control_server/scripts/nexus_ctl.sh restart
```

### 6.5 Health-check publico

```bash
curl -fsS https://nexus-control.projeton-e-x-u-sdepain85352.workers.dev/api/health
```

### 6.6 Ver log do launcher em tempo real

```bash
tail -f nexus_control_server/nexus.log
```

## 7. Como subir manualmente (sem nexus_ctl)

```bash
bash nexus_control_server/scripts/nexus_start.sh
```

Esse script faz:

1. sobe backend local em `127.0.0.1:8787`
2. abre quick tunnel com `cloudflared`
3. captura URL publica do tunnel
4. publica script do Worker via API Cloudflare
5. mantem processo em execucao

## 8. Servicos gerenciados (`services.json`)

Cada servico aceita:

- `command`: array com comando
- `cwd`: diretorio de execucao
- `interactive`: `true/false`
- `env`: variaveis extras para processo

Servico interativo:

- usa sessao TTY/PTy (quando disponivel)
- aceita entrada remota pelo painel
- saida vai para buffer de terminal + log de arquivo

Servicos atuais:

- `sugestor`
- `agente_intermediario` (interativo)
- `validador` (interativo)

## 9. API do backend

### Publicas

- `GET /api/health`
- `GET /api/auth/config`
- `POST /api/auth/google`
- `POST /api/auth/logout`

### Protegidas (Bearer session token)

- `GET /api/services`
- `POST /api/services/<name>/start`
- `POST /api/services/<name>/stop`
- `POST /api/services/<name>/stdin`
- `GET /api/terminal/<name>?chars=12000`
- `GET /api/logs/<name>?lines=200` (endpoint legado, nao usado na UI atual)

## 10. Como usar o painel (operacao diaria)

1. Acesse a URL fixa e faca login Google.
2. Em `Servicos`, clique `Start` no servico desejado.
3. Em `Terminal`, selecione o servico interativo.
4. Envie comandos/teclas (ex.: `a`, `r`, `p`, `q`, `help`).
5. Acompanhe a saida no painel de terminal.
6. Use `Stop` para encerrar o processo.

## 11. Integracao com banco (caso validador/agente)

- `scripts/run_copy_env.sh` define host/porta do PostgreSQL de copia.
- `resolve_kb_ingest_password.sh` tenta resolver automaticamente senha do `kb_ingest`.
- `validador` agora le conexao por ambiente:
  - `POSTGRES_HOST`
  - `POSTGRES_PORT`
  - `POSTGRES_DB`
  - `POSTGRES_USER`
  - `KB_INGEST_PASSWORD`

Se o servico cair apos start com erro de banco, valide credenciais com:

```bash
PGPASSWORD='<senha>' psql -h 127.0.0.1 -p 5433 -U kb_ingest -d knowledge_base -c 'SELECT 1;'
```

## 12. Troubleshooting

### 12.1 Erro 1016/530 no site

Causa: Worker apontando para tunnel expirado.

Correcao:

```bash
bash nexus_control_server/scripts/nexus_ctl.sh restart
```

### 12.2 Login Google nao abre popup

- confirmar `NEXUS_GOOGLE_CLIENT_ID`
- confirmar allowlist
- conferir bloqueio de extensao/antivirus
- checar CSP/COOP no response

### 12.3 Start de servico nao sobe

- abrir terminal e rodar `status`
- checar `logs/control/<servico>.log`
- verificar banco e senha para servicos que dependem de PostgreSQL

### 12.4 `stdin` indisponivel

- servico pode ter sido iniciado fora deste painel
- dar `Stop` e `Start` pelo painel para reconectar TTY

## 13. Seguranca implementada

- validacao estrita de token
- sessao com expiracao
- rate limit por escopo
- headers de seguranca HTTP
- CSP e HSTS em producao
- validacao de payload JSON
- sanitizacao de nome de servico
- leitura de logs limitada ao diretorio permitido

## 14. Limites atuais

- push Git automatico pode falhar no host Windows por problema de credencial TLS (`schannel`).
- se credenciais do banco mudarem sem atualizar variaveis, `validador/agente` podem cair no start.
- endpoint `/api/logs` ainda existe para retrocompatibilidade, mas a UI atual usa `Servicos + Terminal`.

## 15. Operacao recomendada (resumo)

1. `bash nexus_control_server/scripts/nexus_ctl.sh status`
2. se necessario: `bash nexus_control_server/scripts/nexus_ctl.sh restart`
3. login no painel
4. start de servico
5. interacao pelo terminal remoto
6. monitorar `nexus.log` e `logs/control/*.log`
