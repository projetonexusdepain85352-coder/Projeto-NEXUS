# Testes

Este repositorio possui testes por modulo dentro de `tests/`.

## Comandos principais
```bash
cargo test --workspace
```

## Integracao (ignorados por padrao)
```bash
NEXUS_INTEGRATION_TESTS=1 cargo test --workspace -- --include-ignored
```

## Execucao por modulo
```bash
cargo test -p agente_intermediario
cargo test -p nexus_validador
cargo test -p nexus_rag
cargo test -p nexus_mtp
```

## Observacoes
- Testes de integracao podem exigir PostgreSQL e Qdrant (normalmente via Docker).
- `nexus_control_server` e `nexus_sugestor` sao Python e nao entram nos testes Rust.