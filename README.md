# Projeto N.E.X.U.S

## Neural Execution and Unified Systems

> Objetivo: construir uma IA autonoma, privada e distribuida, com aprendizado continuo, resposta multimodal e controle de infraestrutura, iniciando em RTX 4050 8GB / 16GB RAM e evoluindo para cluster heterogeneo.

## Estado Atual (atualizado em 06/03/2026)

- Fase: `Fase 0 / Etapa A` (popular base imutavel)
- Progresso estimado: `~60%`
- Base de conhecimento (ultima contagem registrada): `1.408 documentos`
- Validacao: `5 approved`, `1 rejected`, restante `pending`
- Modelo especializado em producao: `ainda nao`

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
- Fluxo atual ainda e o principal gargalo operacional.

### 4) Nexus RAG (Rust + Qdrant)

- Embeddings: `all-MiniLM-L6-v2` (dim=384).
- Comandos: `index`, `query`, `status`.
- Politica ativa: modo estrito de grounding por evidencia validada.

### 5) Nexus MTP (Rust + Python/unsloth)

- Pipeline disponivel: `extract -> train -> benchmark -> approve -> deploy -> status`.
- Base model planejado para ciclos iniciais: familia Mistral 7B (QLoRA 4-bit).
- Sem deploy de modelo ainda (dependente de massa critica validada).

### 6) Servidor de Controle Web (novo)

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

## Operacao Diaria (resumo)

1. Subir infraestrutura de dados (`PostgreSQL` e `Qdrant`).
2. Rodar coleta no Agente Intermediario.
3. Validar lote pendente no Validador TUI.
4. Indexar aprovados no RAG.
5. Rodar ciclo do MTP para dominio com massa critica.
6. Aprovar manualmente antes de qualquer deploy.

## Seguranca e Segredos

- Nao versionar segredos em README, codigo ou commits.
- Usar variaveis de ambiente para senhas/tokens/chaves.
- Recomenda-se rotacionar credenciais ja expostas em historico/local.

## Limitacoes e Gargalos Atuais

| Item | Impacto | Mitigacao sugerida |
| --- | --- | --- |
| Baixo volume de documentos aprovados | Bloqueia treino util e deploy do 1o modelo | Mutirao de validacao por dominio prioritario com metas diarias (ex.: 80-120 docs/dia) |
| Validacao manual e lenta | Crescimento da base nao acompanha coleta | Priorizar fila por score/criticidade e ativar triagem semiautomatica com revisao humana |
| Ambiente Windows + WSL para producao | Maior risco operacional e performance inconsistente | Migrar runtime critico para Linux nativo (host principal) |
| Credenciais ja circularam em texto | Risco de seguranca operacional | Rotacionar senhas/tokens e centralizar em `.env` local nao versionado + cofre de segredos |
| Ausencia de modelo em producao | Nucleo ainda sem resposta especializada propria | Fechar criterio de parada da Etapa A e executar primeiro ciclo completo do MTP |
| Qdrant com pouca/nenhuma colecao util | RAG sem ganho pratico em producao | Indexar imediatamente apos aprovacoes por dominio e monitorar cobertura por colecao |
| Watchdog/RTS/IO/DRO ainda nao implantados | Governanca e resiliencia incompletas | Entrega faseada: Watchdog minimo -> RTS minimo -> DR minimo -> IO |
| Acesso remoto em fase inicial | Superficie de ataque pode aumentar | Expor apenas via tunel HTTPS + allowlist Google + token forte + sem porta aberta no roteador |

## Proximos Passos Prioritarios

1. Fechar validacao manual pendente para atingir massa critica por dominio.
2. Indexar aprovados e validar qualidade de recuperacao no RAG.
3. Executar primeiro ciclo completo MTP (`extract/train/benchmark/approve/deploy`).
4. Implantar watchdog minimo com criterios mensuraveis e alerta.
5. Migrar componentes criticos para Linux host e formalizar baseline de observabilidade.

---

Se quiser, no proximo passo eu tambem gero uma versao `README_OPERACAO.md` (runbook objetivo, so comandos e troubleshooting) e uma `README_ARQUITETURA.md` (visao tecnica detalhada por componente).
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
