# Replicacao (Step-by-Step)

## Pre-requisitos

- Git
- Rust + Cargo
- Python 3
- PostgreSQL
- Qdrant
- (Opcional) Docker/WSL

## Passo 1: Clonar

```
git clone <repo>
cd Projeto-NEXUS
```

## Passo 2: Configurar ambiente

- Criar arquivo local de ambiente (ver `config/env/`).
- Variaveis minimas por fluxo:
  - `KB_READER_PASSWORD`
  - `KB_INGEST_PASSWORD`
  - `QDRANT_URL`
  - `POSTGRES_HOST`, `POSTGRES_PORT`, `POSTGRES_DB`, `POSTGRES_USER`
  - Para painel com Google: `NEXUS_GOOGLE_CLIENT_ID`, `NEXUS_GOOGLE_ALLOWED_EMAILS`

## Passo 3: Setup do banco

- Criar banco `knowledge_base`.
- Aplicar migracoes em `database/migrations` (ordem lexicografica).
- Rodar ajuste de permissoes:

```
bash config/scripts/ensure_permissions.sh
```

## Passo 4: Compilar modulos

```
cargo build --workspace
```

Se necessario, compilar modulo especifico:

```
cargo build -p agente_intermediario
cargo build -p nexus_rag
cargo build -p nexus_mtp
cargo build -p nexus_validador
```

## Passo 5: Subir servicos (ordem)

1. Banco e Qdrant online.
2. `agente_intermediario` para ingestao inicial.
3. `nexus_validador` para aprovacoes.
4. `nexus_rag index` para indexacao.
5. `nexus_mtp` para treino/benchmark.
6. `nexus_control_server` para operacao remota.

## Erros comuns

- Falha de conexao DB: revisar host/porta/usuario/senha.
- RAG sem resultados: conferir se ha documentos aprovados e indexados.
- Painel sem autenticacao: validar `NEXUS_GOOGLE_CLIENT_ID` e allowlist.
- Script falhando por caminho: validar se esta usando a nova arvore (`src/`, `config/`).

## Checklist de validacao final

- `cargo build --workspace` finaliza sem erro.
- `nexus_rag status` mostra collections e deltas esperados.
- `nexus_validador` conecta ao banco e lista pendentes.
- `nexus_control_server` responde em `/api/health`.
- Fluxo completo `coleta -> validacao -> index -> consulta` funciona ponta a ponta.
