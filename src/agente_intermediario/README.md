# Agente Intermediário

[IMPLEMENTAÇÃO]
- Componente de coleta e ingestão técnica para a base de conhecimento.
- Fluxo: crawl -> limpeza -> deduplicação -> persistência em `documents` e `validation`.
- Implementação principal em `src/main.rs`.
- Conexão de banco é configurável por ambiente em `src/main.rs`:
  - `POSTGRES_HOST`, `POSTGRES_PORT`, `POSTGRES_DB`, `POSTGRES_USER`/`POSTGRES_INGEST_USER`
  - senha via `KB_INGEST_PASSWORD`
- Dependências principais: `reqwest` (blocking), `scraper`, `lopdf`, `postgres`, `sha2`, `uuid`.

[OPERAÇÃO]
- Variável obrigatória:
  - `KB_INGEST_PASSWORD`
- Build/execução:
  - `cargo build -p agente_intermediario --release`
  - `cargo run -p agente_intermediario --release`
- Integração com outros módulos:
  - alimenta o banco consumido por `validador`, `nexus_rag` e `nexus_mtp`.
- Observação operacional:
  - em WSL fora de container, use `POSTGRES_PORT=5433` (default do módulo quando não informado).
