# Projeto N.E.X.U.S

## Neural Execution and Unified Systems

> Objetivo: construir uma IA autonoma, privada e distribuida, com aprendizado continuo, resposta multimodal e controle de infraestrutura, iniciando em RTX 4050 8GB / 16GB RAM e evoluindo para cluster heterogeneo.

---

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

---

## Visao Resumida do Projeto

NEXUS e dividido em componentes independentes, com isolamento e trilha de auditoria:

- Nucleo Central (Core): logica critica em Rust, sem acesso direto a internet.
- Agente Intermediario: unico componente com internet, responsavel por coleta e ingestao.
- Base Imutavel (PostgreSQL): fonte unica de verdade para documentos e validacoes.
- RAG (Qdrant + embeddings): memoria vetorial para consulta de evidencias.
- MTP (Model Training Pipeline): extracao, dataset, treino, benchmark, aprovacao e deploy.
- Interface/Controle: painel web para operacao local/remota com autenticacao.
- Camadas futuras: Watchdog, RTS, HO, NO, IO, DRO, Sandbox avancada e cluster completo.

---

## Principios Imutaveis

- Nucleo nunca acessa internet diretamente.
- Nenhuma mudanca critica sem aprovacao humana.
- Conhecimento novo so entra apos validacao.
- Nenhum modelo generico em producao.
- Logs e rastreabilidade sao obrigatorios.
- Complexidade so aumenta com evidencia real.

---

## Regra de Grounding (Obrigatoria)

O agente deve responder somente com base em evidencia validada no banco.

- Sem fallback parametrico ("melhor palpite").
- Sem resposta quando nao houver evidencia suficiente.
- Toda resposta precisa ser rastreavel a `document_id/source`.

Referencia: `NEXUS_GROUNDING_POLICY.md`.

---

## Mapa de Funcionalidades (operacional)

| Componente | Status | Operacao disponivel hoje |
| --- | --- | --- |
| PostgreSQL (`pg_copia`) | Operacional | start/stop, queries, grants, validacao |
| Agente Intermediario | Operacional | coleta tier-1 para `documents` + `validation` |
| Validador TUI (`nexus_validador`) | Operacional | aprovacao/rejeicao/pulo/revalidacao IA |
| Nexus RAG (`nexus_rag`) | Operacional | `status`, `index`, `query` |
| Nexus MTP (`nexus_mtp`) | Operacional | `extract`, `train`, `benchmark`, `approve`, `deploy`, `status`, `stage-a-gate` |
| Nexus Control Server | Operacional | painel web local/remoto, start/stop servicos, logs |
| Backup snapshots | Operacional | snapshot local/container/github |
| Container sync dump | Operacional | dump+schema+stats para `github_backups/container_sync` |
| Watchdog / RTS / IO / DRO | Planejado | sem rotina de producao nesta fase |

---

## Manual Operacional Completo

## 1) Preparacao de ambiente (global)

### 1.1 Pre-flight checklist

1. Entrar no repo raiz `Projeto-NEXUS`.
2. Confirmar Docker em execucao.
3. Confirmar WSL funcional.
4. Confirmar que vai operar em copias (`pg_copia`, `qdrant_copia`).
5. Confirmar variaveis de credencial no shell atual.

Comandos:

```bash
docker ps -a | grep -E "pg_copia|qdrant_copia"
```

Resultado esperado:
- ambos os containers aparecem
- status `Up` (ou iniciar no passo seguinte)

### 1.2 Subir infraestrutura base

```bash
docker start pg_copia qdrant_copia
docker ps | grep -E "pg_copia|qdrant_copia"
```

### 1.3 Resolver host/ports automaticamente

```bash
source scripts/run_copy_env.sh
```

Esse script exporta:
- `POSTGRES_HOST`
- `POSTGRES_PORT` (`5433` default em modo copia)
- `POSTGRES_DB` (`knowledge_base`)
- `POSTGRES_USER` (`kb_reader` default)
- `KB_READER_PASSWORD`
- `KB_INGEST_PASSWORD`
- `QDRANT_URL`
- `NEXUS_ENV`

Estrategia de host:
- tenta `host.docker.internal` quando alcancavel
- fallback para gateway WSL quando necessario

---

## 2) PostgreSQL operacional (base imutavel)

### 2.1 Health-check de leitura (`kb_reader`)

```bash
source scripts/ensure_permissions.sh
nexus_pg_check_reader
```

### 2.2 Auto-correção de grants

```bash
nexus_pg_reapply_reader_grants
nexus_pg_check_reader
```

SQL aplicado pelo script:

```sql
GRANT USAGE ON SCHEMA public TO kb_reader;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO kb_reader;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO kb_reader;
```

Fallback de privilegio:
- tentativa via `kb_ingest`
- fallback via `kb_admin` (se `KB_ADMIN_PASSWORD` estiver definido)

### 2.3 Queries de monitoramento

```sql
-- status por origem de decisao
SELECT decided_by, status, COUNT(*)
FROM validation
GROUP BY decided_by, status
ORDER BY decided_by, status;

-- pendentes por dominio
SELECT d.domain, COUNT(*)
FROM validation v
JOIN documents d ON d.id = v.document_id
WHERE v.status = 'pending'
GROUP BY d.domain
ORDER BY COUNT(*) DESC;
```

### 2.4 Operacao segura de parada

```bash
docker stop pg_copia
```

Nunca apagar dados sem backup confirmado.

---

## 3) Agente Intermediario (coleta)

## 3.1 O que faz

- crawling de fontes tier-1
- extracao HTML/PDF
- filtros de qualidade
- hash/deduplicacao
- insercao em `documents`
- criacao de estado `pending` em `validation`

## 3.2 Build

```bash
cd agente_intermediario
cargo build --release
```

## 3.3 Execucao

```bash
export KB_INGEST_PASSWORD='***'
cargo run --release
```

### 3.4 Observacao importante de conexao

No codigo atual, a conexao do agente esta hardcoded para:
- `host=localhost`
- `port=5432`
- `user=kb_ingest`
- `dbname=knowledge_base`

Ou seja:
- para usar direto em `pg_copia:5433`, hoje e necessario ajustar rota/porta ou adaptar codigo.

## 3.5 Fontes coletadas (resumo)

- Security: OWASP, RFC8446, NIST, NVD
- Rust: reference, nomicon, book, std, cargo
- Infra: kernel docs, docker docs, postgres docs, systemd
- MLOps: arxiv QLoRA, HF PEFT, HF Transformers, llama.cpp

---

## 4) Validador TUI (nexus_validador v0.2.0)

## 4.1 O que e

Validador TUI em Rust para revisar documentos no PostgreSQL e marcar `approved` / `rejected` / `pending`, com:

- heuristica local deterministica
- integracao opcional com Ollama (`mistral`)
- decisao humana no loop principal (com autoacoes em alguns cenarios)

Dominios suportados: `rust`, `infra`, `security`, `mlops`.

## 4.2 Versao e build

- Package: `nexus_validador`
- Versao: `0.2.0`
- Edition: `2024`
- Fonte: `validador/Cargo.toml`

Dependencias reais:

| Crate | Uso |
| --- | --- |
| `postgres` (with-uuid-1) | Banco PostgreSQL |
| `chrono` | Datas/timestamps |
| `serde_json` | serializacao |
| `ureq` | chamadas HTTP ao Ollama |

`tokio-postgres` e `crossterm` nao estao no `Cargo.toml` atual.

## 4.3 Requisitos do validador

- Rust stable
- `pg_copia` ativo
- Ollama com modelo `mistral`

Observacao:
- validador nao usa Qdrant diretamente.

## 4.4 Inicializacao recomendada

```bash
~/projeto/iniciar_validador.sh
```

Fluxo interno do script:
1. source `scripts/run_copy_env.sh` (se existir)
2. source `scripts/ensure_permissions.sh` (se existir)
3. health-check `kb_reader`
4. reapply grants se necessario
5. sobe `nexus_sugestor/servidor.py`
6. abre validador com fallback:
   - `target/release/nexus_validador`
   - `target/release/validador`
   - `cargo run --release`

## 4.5 Conexao de banco usada pela TUI (codigo atual)

Hardcoded no `main.rs`:

```text
host=172.23.160.1
port=5433
dbname=knowledge_base
user=kb_ingest
password=$KB_INGEST_PASSWORD
```

## 4.6 Comandos da TUI

| Tecla/comando | Acao |
| --- | --- |
| `a` | Aprovar |
| `r` | Rejeitar |
| `u` | Marcar inutil |
| `p` | Pular |
| `b` | Browser |
| `i` | Sugestao |
| `h` | Toggle heuristica |
| `t` | Toggle auto-IA |
| `x` | Parar auto-IA |
| `v` | Voltar |
| `?` | Conteudo completo |
| `e` | Stats |
| `z` | Config |
| `s` | Salvar |
| `q` | Sair |
| `help` | Ajuda |
| `ria` | Revalidacao automatica por IA |

## 4.7 IA e limiares

- Endpoint: `http://localhost:11434/api/generate`
- Modelo: `mistral`
- Constante local: `CONFIANCA_MINIMA = 60`
- Defaults runtime:
  - `threshold_ia = 80`
  - `threshold_heuristica = 60`
  - `timeout_ollama = 30`

## 4.8 Sessao

- `validador/nexus_session.txt`
- `validador/nexus_session_state.json`

Nota:
- `validador/nexus_config.json` nao existe no workspace atual.

---

## 5) Nexus RAG (status/index/query)

## 5.1 Variaveis de ambiente

Obrigatorias:
- `KB_READER_PASSWORD`
- `QDRANT_URL`

Opcionais:
- `POSTGRES_HOST`
- `POSTGRES_PORT`
- `POSTGRES_DB`
- `POSTGRES_USER`
- `QDRANT_API_KEY`
- `NEXUS_ENV`

## 5.2 Build

```bash
cd nexus_rag
cargo build --release
```

## 5.3 Status

```bash
cargo run --release -- status
```

Uso:
- compara aprovados no PostgreSQL com indexados no Qdrant
- exibe delta por dominio

## 5.4 Index

```bash
cargo run --release -- index
```

Comportamento real:
- indexa apenas `approved`
- limpa conteudo antes de chunking
- no fim do processo tenta reaplicar grants de `kb_reader`

## 5.5 Query

```bash
cargo run --release -- query "sua pergunta" --domain infra --top 5
```

### 5.6 Erros tipicos

- `Embedding ... Protobuf parsing failed`: cache de embedding corrompido
- `Document ... has empty content`: documento vira vazio apos limpeza

---

## 6) Nexus MTP (pipeline de treino)

## 6.1 Comandos suportados

- `extract`
- `train`
- `benchmark`
- `approve`
- `deploy`
- `status`
- `stage-a-gate`

## 6.2 Build

```bash
cd nexus_mtp
cargo build --release
```

## 6.3 Conexao de banco (codigo atual)

MTP usa `KB_INGEST_PASSWORD` e URL hardcoded:

```text
postgres://kb_ingest:<senha>@localhost:5432/knowledge_base
```

### Implicacao operacional

- para usar com `pg_copia:5433`, hoje e necessario ajustar mapeamento/porta ou adaptar codigo.

## 6.4 Fluxo de operacao minimo

```bash
# 1) extrair dataset
nexus_mtp extract --domain infra --max-samples 1000

# 2) treinar
nexus_mtp train --domain infra --dataset ./datasets/<arquivo>.jsonl

# 3) benchmark
nexus_mtp benchmark --model-id <uuid>

# 4) aprovacao humana (TUI)
nexus_mtp approve

# 5) deploy
nexus_mtp deploy --model-id <uuid>

# 6) status
nexus_mtp status
```

## 6.5 Gate de parada Etapa A

```bash
nexus_mtp stage-a-gate
```

Com parametros de override disponiveis (`--min-security`, `--min-rust`, `--min-infra`, `--min-mlops`, `--min-total`, `--max-pending-total`).

---

## 7) Nexus Control Server (operacao web)

## 7.1 Objetivo

Gerenciar servicos do NEXUS pelo navegador (incluindo Google Chrome), com autenticacao por token e opcionalmente Google.

## 7.2 Defaults reais

- Host default: `127.0.0.1`
- Porta default: `8787`
- Config de servicos: `nexus_control_server/services.json`

## 7.3 Variaveis de autenticacao

- `NEXUS_CONTROL_TOKEN`
- `NEXUS_GOOGLE_CLIENT_ID`
- `NEXUS_GOOGLE_ALLOWED_EMAILS`

## 7.4 Subir local

```powershell
python nexus_control_server/server.py
```

Painel local:
- `http://localhost:8787`

## 7.5 Endpoints API

- `GET /api/health`
- `GET /api/auth/config`
- `POST /api/auth/google`
- `GET /api/services`
- `POST /api/services/<name>/start`
- `POST /api/services/<name>/stop`
- `GET /api/logs/<name>?lines=200`

## 7.6 Exposicao externa (Cloudflare Tunnel)

Scripts disponiveis:
- `nexus_control_server/scripts/start_quick_tunnel_wsl.sh`
- `nexus_control_server/scripts/setup_named_tunnel_wsl.sh`
- `nexus_control_server/scripts/start_named_tunnel_wsl.sh`

Regra:
- nao abrir porta 8787 no roteador
- publicar via tunnel HTTPS

---

## 8) Backup, versionamento e sincronizacao

## 8.1 Snapshot de codigo (padrao)

```powershell
powershell -ExecutionPolicy Bypass -File scripts/backup_snapshot.ps1 -Label "nome_da_mudanca" -CommitGithubBackup -PushGithubBackup
```

Esse fluxo gera:
- archive local em `backups/code_snapshots`
- snapshot em `github_backups/<timestamp_label>`
- copia do archive no container (`/var/backups/nexus_code`) quando habilitado

## 8.2 Sync de container para github_backups

```powershell
powershell -ExecutionPolicy Bypass -File scripts/sync_container_to_github.ps1 -ContainerName pg_copia -Push
```

Esse script coleta:
- `docker inspect` do container e imagem
- dump completo (`knowledge_base.dump`)
- dump de schema (`knowledge_base_schema.sql`)
- stats de validacao
- `manifest.json` com hashes

## 8.3 Politica operacional

- toda mudanca funcional: commit + push
- toda mudanca funcional: snapshot de backup
- evitar commitar artefatos de runtime (`.fastembed_cache`, logs temporarios)

---

## 9) Runbooks de incidente

## 9.1 Erro no validador: `Falha ao conectar ao banco de dados: db error`

1. `source scripts/run_copy_env.sh`
2. `source scripts/ensure_permissions.sh`
3. `nexus_pg_check_reader || nexus_pg_reapply_reader_grants`
4. rerodar `iniciar_validador.sh`

## 9.2 RAG com falha de embedding

1. identificar cache corrompido
2. limpar apenas o cache do modelo afetado
3. rerodar `nexus_rag index`
4. confirmar com `nexus_rag status`

## 9.3 Host WSL mudou apos reboot

1. rerodar `source scripts/run_copy_env.sh`
2. validar host resolvido exibido pelo script
3. retestar conexao DB e Qdrant

## 9.4 Index com delta residual

1. rodar `nexus_rag status`
2. rerodar `nexus_rag index`
3. inspecionar erros por documento
4. separar erros de conteudo vazio para ajuste de limpeza

---

## 10) Estrutura de arquivos (operacional)

```text
Projeto-NEXUS/
+-- README.md
+-- NEXUS_GROUNDING_POLICY.md
+-- iniciar_validador.sh
+-- scripts/
¦   +-- run_copy_env.sh
¦   +-- ensure_permissions.sh
¦   +-- backup_snapshot.ps1
¦   +-- sync_container_to_github.ps1
+-- agente_intermediario/
+-- validador/
+-- nexus_rag/
+-- nexus_mtp/
+-- nexus_control_server/
+-- nexus_sugestor/
+-- backups/
+-- github_backups/
+-- logs/
```

---

## 11) Limitacoes e gargalos atuais

| Item | Impacto | Mitigacao sugerida |
| --- | --- | --- |
| `10` aprovados de `security` nao indexados | Cobertura incompleta no RAG para seguranca | identificar docs que viram vazio apos limpeza e ajustar regra de limpeza por dominio |
| `126 pending` ainda sem decisao | Atrasa treino especializado | mutirao de validacao por lotes diarios |
| Agente e MTP hardcoded em `localhost:5432` | friccao no modo copia (`5433`) | padronizar configuracao via env para todos os componentes |
| Dependencia de roteamento WSL/host | risco de intermitencia | manter `run_copy_env.sh` como etapa obrigatoria |
| Sem modelo especializado em producao | sistema ainda sem resposta especializada propria | fechar Etapa A e executar primeiro ciclo MTP completo |
| Watchdog/RTS/IO/DRO ainda nao implantados | governanca e resiliencia incompletas | entrega faseada apos consolidacao da Etapa A |

---

## 12) Proximos passos priorizados

1. Fechar os `126 pending` restantes com foco em qualidade.
2. Resolver os `10` documentos `security` nao indexados.
3. Unificar configuracao de DB por env em Agente e MTP (remover hardcode `5432`).
4. Executar primeiro ciclo completo MTP (`extract -> train -> benchmark -> approve -> deploy`).
5. Consolidar runbook Linux nativo para reduzir fragilidade operacional do WSL.
