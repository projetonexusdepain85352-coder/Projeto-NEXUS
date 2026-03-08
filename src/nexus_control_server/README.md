# Nexus Control Server

[IMPLEMENTAÇÃO]
- Backend HTTP em Python para controle de serviços e terminal remoto.
- Arquivos principais:
  - `server.py`: autenticação Google, sessões, rate limit, API REST.
  - `frontend/`: UI web (`index.html`, `app.js`, `styles.css`).
  - `services.json`: catálogo de serviços gerenciados.
- Endpoints observados em `server.py`:
  - `GET /api/health`
  - `GET /api/auth/config`
  - `POST /api/auth/google`
  - `POST /api/auth/logout`
  - `GET /api/services`
  - `POST /api/services/<name>/start|stop|stdin`
  - `GET /api/terminal/<name>`
  - `GET /api/logs/<name>`

[OPERAÇÃO]
- Variáveis de ambiente relevantes:
  - `NEXUS_CONTROL_HOST`, `NEXUS_CONTROL_PORT`
  - `NEXUS_GOOGLE_CLIENT_ID`, `NEXUS_GOOGLE_ALLOWED_EMAILS`
  - `NEXUS_SESSION_BIND_CONTEXT`, `NEXUS_ENV`, `NEXUS_PROJECT_ROOT`
- Inicialização local:
  - `python src/nexus_control_server/server.py`
- Scripts de apoio:
  - ver `config/scripts/README.md`.
- Runbook detalhado legado:
  - `docs/runbooks/nexus_control_server_README.md`.
