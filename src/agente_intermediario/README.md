# Agente Intermediário

[IMPLEMENTAÇÃO]
- Componente de coleta e ingestão técnica para a base de conhecimento.
- Fluxo: crawl -> limpeza -> deduplicação -> persistência em `documents` e `validation`.
- Implementação principal em `src/main.rs`.
- Conexão de banco atual está hardcoded em `src/main.rs:1004`:
  - `host=localhost port=5432 dbname=knowledge_base user=kb_ingest`
  - senha vem de `KB_INGEST_PASSWORD`.
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
  - para operação em porta diferente (ex.: `5433`) ainda é necessário refatorar essa conexão.
