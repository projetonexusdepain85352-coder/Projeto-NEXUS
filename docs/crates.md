# Crates e Modulos

## Rust (workspace Cargo)

### agente_intermediario

- Proposito: coleta e ingestao de documentos tecnicos no PostgreSQL.
- Binario: `agente_intermediario`.
- Dependencias principais: `reqwest`, `scraper`, `lopdf`, `postgres`, `sha2`, `uuid`.
- Status: implementado.
- Variaveis obrigatorias: `KB_INGEST_PASSWORD`.
- Atencao: conexao DB hardcoded em `src/main.rs:1004` (rever antes de refatorar).

### nexus_rag

- Proposito: indexacao RAG e consulta grounded (STRICT_DB_ONLY).
- Binario: `nexus_rag`.
- Dependencias principais: `qdrant-client`, `fastembed`, `sqlx`, `tokio`, `clap`.
- Status: implementado.
- Variaveis obrigatorias: `KB_READER_PASSWORD`, `QDRANT_URL`.
- Atencao: nao relaxar `STRICT_MIN_SCORE = 0.35` sem justificativa.

### nexus_mtp

- Proposito: pipeline de treino (extract -> train -> benchmark -> approve -> deploy).
- Binario: `nexus_mtp`.
- Dependencias principais: `sqlx`, `tokio`, `serde`, `clap`, `reqwest`.
- Status: implementado.
- Variaveis obrigatorias: `KB_INGEST_PASSWORD`.
- Atencao:
  - conexao DB hardcoded em `src/main.rs:490`.
  - garantir caminho do modelo base e adapter antes de treinar.
  - fluxo exige aprovacao humana.

### nexus_validador

- Proposito: TUI de validacao humana de documentos.
- Binario: `nexus_validador`.
- Dependencias principais: `postgres`, `ureq`, `tracing`, `chrono`.
- Status: implementado.
- Variaveis obrigatorias: `KB_INGEST_PASSWORD` (e `POSTGRES_*`).
- Atencao: defaults de DB em `src/main.rs:3052`.

### nexus_rag_agent

- Proposito: servidor RAG com grounding e verificador holistico.
- Binario: `nexus_agent_server`.
- Dependencias principais: `nexus_rag`, `axum`, `reqwest`, `fastembed`, `sqlx`.
- Status: implementado.
- Variaveis obrigatorias: `NEXUS_OLLAMA_URL`, `NEXUS_BASE_MODEL`, `QDRANT_URL`, `POSTGRES_*`, `KB_READER_PASSWORD`.
- Atencao: `QDRANT_URL` deve usar porta gRPC 6336; `POSTGRES_HOST` deve ser gateway WSL.

## Modulos Python (fora do workspace)

### nexus_control_server

- Proposito: painel HTTP para controle de servicos.
- Binario: `src/nexus_control_server/server.py`.
- Dependencias principais: Flask, Google OAuth, Cloudflare (opcional).
- Atencao: manter sanitizacao de nome de servico e rate limit.
- Como executar: `python src/nexus_control_server/server.py`.

### nexus_sugestor

- Proposito: sugestao automatica para o validador via socket UNIX.
- Binario: `src/nexus_sugestor/servidor.py`.
- Dependencias principais: Python + Ollama local.
- Interface: socket UNIX em `/tmp/nexus_sugestor.sock` (env `NEXUS_SUGESTOR_SOCKET`).
- Como executar: `python3 src/nexus_sugestor/servidor.py`.
