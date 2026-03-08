# Validador

[IMPLEMENTAÇÃO]
- TUI em Rust para decisão humana sobre documentos (`approved`, `rejected`, `pending`).
- Arquivo principal: `src/main.rs` (fluxo interativo, heurística e integração opcional de IA).
- Conexão de banco em `src/main.rs:3052-3055`:
  - defaults para `POSTGRES_HOST=172.23.160.1`, `POSTGRES_PORT=5433`, `POSTGRES_DB=knowledge_base`, `POSTGRES_USER=kb_ingest`.
  - senha via `KB_INGEST_PASSWORD`.
- Regras de validação:
  - decisão manual como fonte principal;
  - heurística local e IA como suporte;
  - persistência de sessão para retomada.

[OPERAÇÃO]
- Variáveis obrigatórias:
  - `KB_INGEST_PASSWORD`
- Variáveis opcionais:
  - `POSTGRES_HOST`, `POSTGRES_PORT`, `POSTGRES_DB`, `POSTGRES_USER`
- Execução:
  - `cargo run -p nexus_validador --release`
  - ou via script de inicialização em `config/scripts/iniciar_validador.sh`.
- Integração:
  - consome documentos coletados pelo agente;
  - determina o conjunto aprovado usado por RAG e MTP.
