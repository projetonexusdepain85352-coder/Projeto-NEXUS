# Scripts Operacionais

[IMPLEMENTAÇÃO]
Scripts centralizados em `config/scripts` para bootstrap, operação, túnel e manutenção.

Scripts presentes:
- `iniciar_validador.sh`: prepara ambiente, valida permissões e inicia sugestor + validador.
- `run_copy_env.sh`: resolve host/ports e exporta variáveis padrão do ambiente de cópia.
- `ensure_permissions.sh`: checa/reaplica grants no PostgreSQL para `kb_reader`.
- `resolve_kb_ingest_password.sh`: resolve senha do usuário de ingestão.
- `nexus_ctl.sh`: controle do servidor (`status/start/stop/restart`).
- `nexus_start.sh`: sobe backend e tunnel/worker.
- `install_cloudflared_wsl.sh`: instala `cloudflared` local.
- `setup_worker.sh`: setup inicial do Worker Cloudflare.
- `setup_named_tunnel_wsl.sh`: setup de named tunnel.
- `start_named_tunnel_wsl.sh`: sobe backend + named tunnel.
- `start_quick_tunnel_wsl.sh`: sobe backend + quick tunnel.
- `backup_snapshot.ps1`: snapshot de código.
- `sync_container_to_github.ps1`: sincronização de dump de container.

Nota de segurança (hardcode histórico):
- Foi identificado hardcode de credencial em `iniciar_validador.sh` (linha 63 no layout original).
- Correção aplicada: o valor hardcoded foi removido e substituído por leitura de `NEXUS_DB_PASSWORD` com falha explícita quando a variável não está definida.

[OPERAÇÃO]
Ordem recomendada (fluxo local):
1. `source config/scripts/run_copy_env.sh`
2. `source config/scripts/ensure_permissions.sh`
3. `bash config/scripts/iniciar_validador.sh`
4. (Opcional) `bash config/scripts/nexus_ctl.sh start`

Variáveis de ambiente frequentes:
- `POSTGRES_HOST`, `POSTGRES_PORT`, `POSTGRES_DB`, `POSTGRES_USER`
- `KB_READER_PASSWORD`, `KB_INGEST_PASSWORD`
- `NEXUS_CONTROL_HOST`, `NEXUS_CONTROL_PORT`
- `NEXUS_GOOGLE_CLIENT_ID`, `NEXUS_GOOGLE_ALLOWED_EMAILS`
- `NEXUS_PROJECT_ROOT`, `NEXUS_ENV`

