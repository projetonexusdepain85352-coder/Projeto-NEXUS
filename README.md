# Projeto N.E.X.U.S

## Visão Geral
NEXUS é um sistema de IA privada com coleta, validação humana, indexação RAG e pipeline de treino especializado.

## Arquitetura (alto nível)
- `src/agente_intermediario`: coleta de fontes técnicas e ingestão no PostgreSQL.
- `src/validador`: validação humana/TUI dos documentos.
- `src/nexus_rag`: indexação em Qdrant e consulta grounded.
- `src/nexus_mtp`: extração de dataset, treino, benchmark, aprovação e deploy de modelo.
- `src/nexus_control_server`: painel web e orquestração operacional dos serviços.
- `src/nexus_sugestor`: serviço auxiliar de sugestão usado no fluxo do validador.

## Estrutura do Repositório
- `src/`: código-fonte dos módulos.
- `config/`: scripts operacionais, exemplos de ambiente e configs auxiliares.
- `database/`: migrações e schema de referência.
- `docs/`: documentação de arquitetura e runbooks.
- `.ai/`: documentação para agentes de IA.
- `tests/`: espaço para integração/e2e.

## Pré-requisitos Globais
- Rust (toolchain estável) e Cargo.
- Python 3.
- PostgreSQL e Qdrant acessíveis.
- (Opcional) Docker/WSL para ambiente de operação atual.

## Como rodar do zero (5 passos)
1. Clonar o repositório e entrar na raiz.
2. Configurar variáveis de ambiente (ver `config/env` e READMEs dos módulos).
3. Inicializar banco (ver `database/README.md`).
4. Compilar módulos Rust via workspace (`cargo build --workspace`).
5. Iniciar serviços na ordem: coleta/validação -> RAG -> MTP -> painel de controle.

## Referências
- Política de grounding: `docs/architecture/NEXUS_GROUNDING_POLICY.md`.
- Runbook legado do painel: `docs/runbooks/nexus_control_server_README.md`.
