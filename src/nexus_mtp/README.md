# Nexus MTP

[IMPLEMENTAÇÃO]
- Pipeline de treino especializado do NEXUS: `extract`, `train`, `benchmark`, `approve`, `deploy`, `status`, `stage-a-gate`.
- Módulos internos:
  - `dataset.rs`: extração e validação de dataset.
  - `trainer.rs`: execução do treino.
  - `benchmark.rs`: avaliação de desempenho.
  - `approval.rs`: aprovação humana em TUI.
  - `db.rs`: persistência de ciclos de treino.
- Conexão de banco atual está em `src/main.rs:488-491` com URL montada para `localhost:5432` usando `KB_INGEST_PASSWORD`.
  - Motivo atual: operação local simplificada durante a fase inicial.
  - Impacto: acoplamento operacional ao host/porta.

[OPERAÇÃO]
- Variável obrigatória:
  - `KB_INGEST_PASSWORD`
- Variáveis úteis:
  - `NEXUS_BASE_MODEL` (override do modelo base)
- Banco esperado:
  - PostgreSQL com schema `knowledge_base`.
  - Migração versionada em `database/migrations/001_mtp_schema.sql`.
- Comandos:
  - `cargo run -p nexus_mtp -- extract --domain infra --max-samples 1000`
  - `cargo run -p nexus_mtp -- train --domain infra --dataset <arquivo.jsonl>`
  - `cargo run -p nexus_mtp -- benchmark --model-id <uuid>`
  - `cargo run -p nexus_mtp -- approve`
  - `cargo run -p nexus_mtp -- deploy --model-id <uuid>`
