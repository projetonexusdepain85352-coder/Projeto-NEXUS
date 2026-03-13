# Crates

## agente_intermediario

- Propósito: coleta e ingestão de documentos técnicos no PostgreSQL.
- Binário: `agente_intermediario` (main padrão).
- Dependências principais: `reqwest`, `scraper`, `lopdf`, `postgres`, `sha2`, `uuid`.
- Status: implementado.

## nexus_rag

- Propósito: indexação RAG e consulta grounded.
- Binário: `nexus_rag`.
- Dependências principais: `qdrant-client`, `fastembed`, `sqlx`, `tokio`, `clap`.
- Status: implementado.

## nexus_mtp

- Propósito: pipeline de treino (extract → train → benchmark → approve → deploy).
- Binário: `nexus_mtp`.
- Dependências principais: `sqlx`, `tokio`, `serde`, `clap`, `candle-*`, `reqwest`.
- Status: implementado.

## nexus_validador

- Propósito: TUI de validação humana de documentos.
- Binário: `nexus_validador` (main padrão).
- Dependências principais: `postgres`, `ureq`, `tracing`, `chrono`.
- Status: implementado.

## nexus_rag_agent

- Propósito: servidor RAG com grounding e verificador holístico.
- Binário: `nexus_agent_server`.
- Dependências principais: `nexus_rag`, `axum`, `reqwest`, `fastembed`, `sqlx`.
- Status: implementado.

## nexus_control_server (Python)

- Propósito: painel HTTP para controle de serviços.
- Binário: `server.py` (Python).
- Dependências principais: Flask stack local, OAuth Google.
- Status: implementado.
