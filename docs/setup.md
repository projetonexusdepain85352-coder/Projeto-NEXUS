# Setup e Ambiente

## Requisitos

- WSL2 Ubuntu 24.04
- Docker Desktop
- Rust toolchain
- Ollama (Mistral local)

## Containers necessários

- PostgreSQL: `pg_copia` em `5433`
- Qdrant: `qdrant_copia` com gRPC em `6336`

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

## Observações críticas

- `QDRANT_URL` deve usar gRPC na porta `6336` (não `6335`).
- `KB_READER_PASSWORD` é obrigatório para leitura.

## Build do workspace

```
/home/dulan/.cargo/bin/cargo build --release --workspace
```

## Subir o nexus_agent_server

```
nohup env QDRANT_URL=http://localhost:6336 \
  ./target/release/nexus_agent_server \
  > /tmp/agent.log 2>&1 &
disown $!
```
