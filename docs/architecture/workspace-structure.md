# Estrutura do Workspace

## Membros Rust do workspace
O workspace Cargo inclui apenas crates Rust com `Cargo.toml` proprio:
- `src/agente_intermediario`
- `src/validador`
- `src/nexus_rag`
- `src/nexus_mtp`

Build pelo root do repositorio:
- `cargo build --workspace`
- `cargo test --workspace`

## Modulos fora do workspace Cargo
Alguns modulos sao Python e nao possuem `Cargo.toml`. Eles ficam fora do workspace e sao executados separadamente.

### `src/nexus_control_server`
- **Motivo da exclusao**: servidor HTTP em Python + frontend estatico (sem crate Rust).
- **Como executar**:
  - `python src/nexus_control_server/server.py`
- **Notas**: depende de configuracao OAuth e usa `services.json` para gerenciar subprocessos. Veja `src/nexus_control_server/README.md` e `docs/runbooks/nexus_control_server_README.md`.

### `src/nexus_sugestor`
- **Motivo da exclusao**: daemon Python com socket UNIX + Ollama (sem crate Rust).
- **Como executar**:
  - `python3 src/nexus_sugestor/servidor.py`
- **Notas**: requer Ollama rodando localmente e um modelo compativel. Veja `src/nexus_sugestor/README.md`.
