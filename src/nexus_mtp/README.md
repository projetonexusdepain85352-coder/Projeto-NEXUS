# Nexus MTP

[IMPLEMENTAÇÃO]
- Pipeline de treino especializado do NEXUS: `extract`, `train`, `benchmark`, `approve`, `deploy`, `status`, `stage-a-gate`.
- Módulos internos:
  - `dataset.rs`: extração e validação de dataset.
  - `trainer.rs`: execução do treino.
  - `benchmark.rs`: avaliação de desempenho.
  - `approval.rs`: aprovação humana em TUI.
  - `db.rs`: persistência de ciclos de treino.
- Conexão de banco é montada via variáveis de ambiente em `src/main.rs` (`POSTGRES_HOST`, `POSTGRES_PORT`, `POSTGRES_DB`, `POSTGRES_USER`/`POSTGRES_INGEST_USER`, `KB_INGEST_PASSWORD`).
  - Defaults para execução WSL fora de container: `localhost:5433`, DB `knowledge_base`, usuário `kb_ingest`.

[OPERAÇÃO]
- Variável obrigatória:
  - `KB_INGEST_PASSWORD`
- Variáveis úteis:
  - `NEXUS_BASE_MODEL` (override do modelo base)
  - `NEXUS_MODELS_DIR` (diretório de saída de treinamento/deploy; default `/opt/nexus/models`)
- Banco esperado:
  - PostgreSQL com schema `knowledge_base`.
  - Migração versionada em `database/migrations/001_mtp_schema.sql`.
- Comandos:
  - `cargo run -p nexus_mtp -- extract --domain infra --max-samples 1000`
  - `cargo run -p nexus_mtp -- train --domain infra --dataset <arquivo.jsonl>`
  - `cargo run -p nexus_mtp -- benchmark --model-id <uuid>`
  - `cargo run -p nexus_mtp -- approve`
  - `cargo run -p nexus_mtp -- deploy --model-id <uuid>`

[BENCHMARK]
- O benchmark roda via subprocess Python (HF + PEFT + BnB). Nao usar Candle para modelos BnB 4-bit.
- Prompt Alpaca obrigatorio:
```
### Instruction:
{q}

### Response:
```
- Stop tokens obrigatorios: `eos_token_id = [2, 1542]` (EOS + token `###`).
- `adapter_path` no banco deve ser relativo ao `NEXUS_MODELS_DIR`, sem prefixo `models/` (ex.: `adapters/<model>`).
