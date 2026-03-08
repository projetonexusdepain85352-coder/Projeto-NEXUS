# Módulo: nexus_control_server

- Responsabilidade única: fornecer painel/API para operação dos serviços NEXUS.
- Entradas: requisições HTTP autenticadas e comandos de operador.
- Saídas: status de serviços, logs, terminal remoto, ações start/stop.
- Dependências internas: execução de `validador`, `agente_intermediario` e `sugestor` via `services.json`.
- Dependências externas: Google OAuth (tokeninfo), opcional Cloudflare Tunnel/Worker.
- Variáveis obrigatórias: `NEXUS_CONTROL_HOST`, `NEXUS_CONTROL_PORT` (defaults existem); para login Google: `NEXUS_GOOGLE_CLIENT_ID`.
- Pontos de atenção para IA:
  - não quebrar validação de sessão/rate limit;
  - manter sanitização de nome de serviço;
  - revisar caminhos do `services.json` ao mover estrutura.
