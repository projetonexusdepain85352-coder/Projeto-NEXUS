# Replicação Step-by-Step

## Pré-requisitos
- Git
- Rust + Cargo
- Python 3
- PostgreSQL
- Qdrant
- (Opcional) Docker/WSL

## Passo 1: Clonar
- `git clone <repo>`
- `cd Projeto-NEXUS`

## Passo 2: Configurar ambiente
- Criar arquivo local de ambiente (ver `config/env/`).
- Variáveis mínimas por fluxo:
  - `KB_READER_PASSWORD`
  - `KB_INGEST_PASSWORD`
  - `QDRANT_URL`
  - `POSTGRES_HOST`, `POSTGRES_PORT`, `POSTGRES_DB`, `POSTGRES_USER`
  - para painel com Google: `NEXUS_GOOGLE_CLIENT_ID`, `NEXUS_GOOGLE_ALLOWED_EMAILS`

## Passo 3: Setup do banco
- Criar banco `knowledge_base`.
- Aplicar migrações de `database/migrations` (ordem lexicográfica).
- Rodar ajuste de permissões com `config/scripts/ensure_permissions.sh`.

## Passo 4: Compilar módulos
- `cargo build --workspace`
- Se necessário, compilar módulo específico:
  - `cargo build -p agente_intermediario`
  - `cargo build -p nexus_rag`
  - `cargo build -p nexus_mtp`
  - `cargo build -p nexus_validador`

## Passo 5: Subir serviços (ordem)
1. Banco e Qdrant online.
2. `agente_intermediario` para ingestão inicial.
3. `validador` para aprovações.
4. `nexus_rag index` para indexação.
5. `nexus_mtp` para treino/benchmark.
6. `nexus_control_server` para operação remota.

## Erros comuns
- Falha de conexão DB: revisar host/porta/usuário/senha.
- RAG sem resultados: conferir se há documentos aprovados e indexados.
- Painel sem autenticação: validar `NEXUS_GOOGLE_CLIENT_ID` e allowlist.
- Script falhando por caminho: validar se está usando a nova árvore (`src/`, `config/`).

## Checklist de validação final
- `cargo build --workspace` finaliza sem erro.
- `nexus_rag status` mostra coleções e deltas esperados.
- `validador` conecta ao banco e lista pendentes.
- `nexus_control_server` responde em `/api/health`.
- Fluxo completo `coleta -> validação -> index -> consulta` funciona ponta a ponta.
