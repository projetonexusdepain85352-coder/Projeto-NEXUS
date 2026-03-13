# Setup e Ambiente

## Requisitos

- WSL2 Ubuntu 24.04
- Docker Desktop
- Rust toolchain
- Python 3
- Ollama (Mistral local)

## Onboarding (IA/colaborador)

Leitura recomendada antes de operar o ambiente:
1. `docs/overview.md`
2. `docs/architecture.md`
3. `docs/setup.md`
4. `docs/crates.md`
5. `docs/operations.md`

## Containers necessarios

- PostgreSQL: `pg_copia` na porta `5433`
- Qdrant: `qdrant_copia` com gRPC na porta `6336`

## Arquivo `.env` (exemplo)

```
RUST_LOG=info
NEXUS_OLLAMA_URL=http://localhost:11434
NEXUS_BASE_MODEL=mistral
QDRANT_URL=http://localhost:6336
POSTGRES_HOST=<ip_gateway_wsl>
POSTGRES_PORT=5433
POSTGRES_DB=knowledge_base
POSTGRES_USER=kb_reader
KB_READER_PASSWORD=<password>
KB_INGEST_PASSWORD=<password>
KB_ADMIN_PASSWORD=<password>
NEXUS_AGENT_HOST=0.0.0.0
NEXUS_AGENT_PORT=8765
VERIFIER_THRESHOLD=0.45
FASTEMBED_CACHE_PATH=
```

## Descobrir POSTGRES_HOST no WSL

```
ip route | grep default | awk '{print $3}'
```

## Observacoes criticas

- `QDRANT_URL` deve usar gRPC na porta `6336` (nao `6335`).
- `KB_READER_PASSWORD` e obrigatorio para leitura.

## Setup do banco

1. Criar o banco `knowledge_base`.
2. Aplicar migracoes em `database/migrations` (ordem lexicografica).
3. Rodar ajuste de permissoes:

```
bash config/scripts/ensure_permissions.sh
```

## Build do workspace

```
/home/dulan/.cargo/bin/cargo build --release --workspace
```

## Ordem recomendada para subir servicos

1. PostgreSQL e Qdrant online.
2. `agente_intermediario` para ingestao inicial.
3. `nexus_validador` para aprovacoes.
4. `nexus_rag index` para indexacao.
5. `nexus_mtp` para treino/benchmark.
6. `nexus_control_server` para operacao remota.

## Subir o nexus_agent_server

```
nohup env QDRANT_URL=http://localhost:6336 \
  ./target/release/nexus_agent_server \
  > /tmp/agent.log 2>&1 &
disown $!
```

## Bootstrap model (dev only)

Se houver um modelo bootstrap em `models/bootstrap/`, ele pode ser usado para testes rapidos:

```
export NEXUS_BASE_MODEL=/mnt/c/Users/dulan/OneDrive/Documentos/GitHub/Projeto-NEXUS/models/bootstrap/qwen2.5-0.5b-instruct
nexus_mtp train --domain infra --dataset ./datasets/infra_YYYYMMDD_HHMMSS.jsonl --epochs 1 --lora-r 8
```

Observacao: modelo de bootstrap e apenas para desenvolvimento. O fluxo oficial passa por `train -> benchmark -> approve -> deploy`.
