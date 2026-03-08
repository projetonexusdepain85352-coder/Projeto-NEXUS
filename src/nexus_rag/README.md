# Nexus RAG

[IMPLEMENTAÇÃO]
- Responsável por indexar documentos `approved` do PostgreSQL em coleções Qdrant e responder consultas por similaridade.
- Estrutura principal em `src/`:
  - `main.rs`: CLI (`index`, `query`, `status`).
  - `db.rs`: conexão e leitura do PostgreSQL.
  - `indexer.rs`: chunking/embedding/upsert no Qdrant.
  - `query.rs`: busca e retorno de evidências.
  - `approval.rs`: gate de aprovação humana ativo em `NEXUS_ENV=production`.
- Dependências centrais: `sqlx`, `qdrant-client`, `fastembed`, `tokio`, `clap`, `tracing`.

[OPERAÇÃO]
- Variáveis obrigatórias:
  - `KB_READER_PASSWORD`
  - `QDRANT_URL`
- Variáveis opcionais:
  - `POSTGRES_HOST`, `POSTGRES_PORT`, `POSTGRES_DB`, `POSTGRES_USER`
  - `QDRANT_API_KEY`, `NEXUS_ENV`, `FASTEMBED_CACHE_PATH`
- Comandos principais:
  - `cargo run -p nexus_rag -- status`
  - `cargo run -p nexus_rag -- index`
  - `cargo run -p nexus_rag -- query "pergunta" --domain infra --top 5`
- Integração com outros módulos:
  - Consome documentos validados pelo `validador`.
  - Alimenta consultas grounded da camada de aplicação.
