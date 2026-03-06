# Projeto N.E.X.U.S

## Neural Execution and Unified Systems

> Objetivo: construir uma IA autonoma, privada e distribuida, com aprendizado continuo, resposta multimodal e controle de infraestrutura, iniciando em RTX 4050 8GB / 16GB RAM e evoluindo para cluster heterogeneo.

## Estado Atual (atualizado em 06/03/2026)

- Fase: `Fase 0 / Etapa A` (popular base imutavel)
- Progresso estimado: `~68%`
- Base de conhecimento (pg_copia): `1.408 documentos`
- Validacao (pg_copia): `1.268 approved`, `14 rejected`, `126 pending`
- Modelo especializado em producao: `ainda nao`

### Ultima execucao em copias (06/03/2026)

- Containers usados:
  - `pg_copia` (`5433`)
  - `qdrant_copia` (`6335/6336`)
- `nexus_rag status` antes do index: `1268 approved / 0 indexed`
- `nexus_rag index` concluido: `1258 indexed`, `10 erros` (dominio `security`)
- `nexus_rag status` depois do index:
  - `infra`: `709/709`
  - `rust`: `384/384`
  - `mlops`: `118/118`
  - `security`: `47/57` (`delta 10`)

## Visao Resumida do Projeto

NEXUS e dividido em componentes independentes, com isolamento e trilha de auditoria:

- Nucleo Central (Core): logica critica em Rust, sem acesso direto a internet.
- Agente Intermediario: unico componente com internet, responsavel por coleta e ingestao.
- Base Imutavel (PostgreSQL): fonte unica de verdade para documentos e validacoes.
- RAG (Qdrant + embeddings): memoria vetorial para consulta de evidencias.
- MTP (Model Training Pipeline): extracao, dataset, treino, benchmark, aprovacao e deploy.
- Interface/Controle: painel web para operacao local/remota com autenticacao.
- Camadas futuras: Watchdog, RTS, HO, NO, IO, DRO, Sandbox avancada e cluster completo.

## Principios Imutaveis

- Nucleo nunca acessa internet diretamente.
- Nenhuma mudanca critica sem aprovacao humana.
- Conhecimento novo so entra apos validacao.
- Nenhum modelo generico em producao.
- Logs e rastreabilidade sao obrigatorios.
- Complexidade so aumenta com evidencia real.

## Componentes Implementados

### 1) PostgreSQL (base imutavel)

- Rodando em Docker (`postgres:17`) com persistencia em disco.
- Estrutura principal:
  - `documents`
  - `validation`
  - `traceability`
- Papeis:
  - `kb_admin` (uso administrativo/manual)
  - `kb_ingest` (ingestao)
  - `kb_reader` (leitura)

### 2) Agente Intermediario v2 (Rust)

- Coleta tier-1 de documentacao, RFCs, CVEs e fontes tecnicas.
- Extracao HTML/PDF com filtros de qualidade e deduplicacao por hash.
- Todo documento entra como `pending`.

### 3) Validador TUI v2 (Rust)

- Aprovacao/rejeicao manual com sessao persistente.
- Heuristica local para sugestao (sem depender de API externa).
- Fluxo ainda e gargalo operacional principal.

### 4) Nexus RAG (Rust + Qdrant)

- Embeddings: `all-MiniLM-L6-v2` (dim=384).
- Comandos: `index`, `query`, `status`.
- Limpeza de conteudo integrada antes de chunking para remover cabecalhos/TOC/lixo de navegacao.
- Politica ativa: modo estrito de grounding por evidencia validada.

### 5) Nexus MTP (Rust + Python/unsloth)

- Pipeline disponivel: `extract -> train -> benchmark -> approve -> deploy -> status`.
- Base model planejado para ciclos iniciais: familia Mistral 7B (QLoRA 4-bit).
- Sem deploy de modelo ainda (dependente de massa critica validada).

### 6) Servidor de Controle Web

- Pasta: `nexus_control_server/`
- Painel para gerenciar servicos via navegador (Google Chrome).
- Autenticacao por token e opcionalmente por conta Google.
- Endpoints para saude, listagem, start/stop e logs.
- Suporte a tunel seguro via Cloudflare (quick e named tunnel).

## Regra de Grounding (Obrigatoria)

O agente deve responder somente com base em evidencia validada no banco.

- Sem fallback parametrico ("melhor palpite").
- Sem resposta quando nao houver evidencia suficiente.
- Toda resposta precisa ser rastreavel a `document_id/source`.

Referencia: `NEXUS_GROUNDING_POLICY.md`.

## Operacao em Copias (padrao atual)

1. Subir `pg_copia` e `qdrant_copia`.
2. Carregar ambiente:

```bash
source scripts/run_copy_env.sh
```

3. Validar permissao de leitura e corrigir grants quando necessario:

```bash
source scripts/ensure_permissions.sh
nexus_pg_check_reader || nexus_pg_reapply_reader_grants
```

4. Rodar RAG:

```bash
cd nexus_rag
cargo run --release -- status
cargo run --release -- index
```

## Resiliencia de Permissoes (novo)

Mudancas aplicadas para evitar o erro `Falha ao conectar ao banco de dados: db error` no validador:

- `nexus_rag/src/indexer.rs`
  - no final do `run_index`, tenta reaplicar:
    - `GRANT USAGE ON SCHEMA public TO kb_reader`
    - `GRANT SELECT ON ALL TABLES IN SCHEMA public TO kb_reader`
    - `ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO kb_reader`
- `scripts/ensure_permissions.sh`
  - funcao `nexus_pg_check_reader` (`SELECT 1` com `kb_reader`)
  - funcao `nexus_pg_reapply_reader_grants` (tenta `kb_ingest`, fallback `kb_admin` se disponivel)
- `iniciar_validador.sh`
  - executa health-check de `kb_reader` antes de abrir TUI
  - tenta autocorrecao de grants se falhar
  - so continua se conexao ficar OK
  - fallback de execucao do validador: binario precompilado ou `cargo run --release`

## Seguranca e Segredos

- Nao versionar segredos em README, codigo ou commits.
- Usar variaveis de ambiente para senhas/tokens/chaves.
- Rotacionar credenciais ja expostas em historico/local.

## Limitacoes e Gargalos Atuais

| Item | Impacto | Mitigacao sugerida |
| --- | --- | --- |
| `10` aprovados de `security` nao indexados | Cobertura incompleta no RAG para seguranca | Rodar `nexus_rag index` com log focado nesses docs, identificar causa (conteudo vazio pos-limpeza, payload, ou erro de embed) e corrigir regra no `clean.rs` |
| Validacao manual ainda pendente (`126 pending`) | Atrasa treino especializado completo | Fechar pendencias por lote e por dominio prioritario com meta diaria objetiva |
| Dependencia de WSL gateway dinamico | Pode quebrar conexao apos reboot | `run_copy_env.sh` ja prioriza `host.docker.internal` e faz fallback automatico para gateway atual |
| Permissao de `kb_reader` pode derivar de grants incompletos em ambientes divergentes | Quebra do validador na abertura | Health-check + autocorrecao ja implementados; manter `ensure_permissions.sh` no bootstrap operacional |
| Ausencia de modelo em producao | Nucleo ainda sem resposta especializada propria | Fechar Etapa A e executar primeiro ciclo completo do MTP |
| Watchdog/RTS/IO/DRO ainda nao implantados | Governanca e resiliencia incompletas | Entrega faseada: Watchdog minimo -> RTS minimo -> DR minimo -> IO |

## Proximos Passos Prioritarios

1. Fechar os `126 pending` restantes.
2. Corrigir os `10` `security` aprovados que ainda nao indexaram.
3. Executar `nexus_mtp extract` com dados ja limpos para o dominio priorizado.
4. Rodar primeiro ciclo `train -> benchmark -> approve -> deploy`.
5. Consolidar runbook Linux nativo para reduzir fragilidade operacional em WSL.

## Backup Policy (obrigatorio)

Para cada alteracao relevante de codigo, gerar backup em duas camadas:

1. Local + container Docker (`/var/backups/nexus_code`)
2. Snapshot para versionamento no GitHub (`github_backups/`)

Script padrao:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/backup_snapshot.ps1 -Label "nome_da_mudanca" -CommitGithubBackup -PushGithubBackup
```

Sem push:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/backup_snapshot.ps1 -Label "nome_da_mudanca"
```
