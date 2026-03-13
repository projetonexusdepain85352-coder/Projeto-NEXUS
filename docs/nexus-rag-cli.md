# Nexus RAG CLI

## Visao geral

O `nexus_rag` e uma CLI para indexacao e consulta grounded. A politica e `STRICT_DB_ONLY` (sem fallback parametrico).

## Comandos principais

- `nexus_rag index`
- `nexus_rag query <texto> [--domain <dominio>] [--top <n>]`
- `nexus_rag status`

### Parametros do query

- `texto`: ate 4096 caracteres.
- `--domain`: opcional (`rust`, `infra`, `security`, `mlops`).
- `--top`: quantidade de chunks (default 5).

## Variaveis de ambiente

Qdrant:
- `QDRANT_URL` (obrigatorio)
- `QDRANT_API_KEY` (opcional)
- `NEXUS_ENV=production` exige https:// ou grpcs://

PostgreSQL:
- `KB_READER_PASSWORD` (obrigatorio)
- `POSTGRES_HOST` (default localhost)
- `POSTGRES_PORT` (default 5433)
- `POSTGRES_DB` (default knowledge_base)
- `POSTGRES_USER` (default kb_reader)

## Saida do query

Campos principais:
- `document_id`, `source`, `domain`, `doc_type`
- `chunk_index`, `chunk_total`, `chunk_text`
- `score`, `collection`

Exemplo (resumo):
```
NEXUS RAG - Strict Grounded Results
Query  : ...
Scope  : ...
Policy : STRICT_DB_ONLY (no parametric fallback)
MinScore: 0.35
Found  : 3 evidence chunk(s)
```

## Politica STRICT_DB_ONLY

- `STRICT_MIN_SCORE = 0.35` (em `src/nexus_rag/src/query.rs`).
- Sem evidencia: retorna `Ungrounded` e imprime `GROUNDING_DENIED`.
- Evidencia abaixo do threshold: nega a consulta.
- Evidencia sem `document_id`: nega a consulta.

## Metricas

- Endpoint: `http://127.0.0.1:9898/metrics` (default)
- Override: `NEXUS_METRICS_ADDR`
- Counter: `nexus_rag_queries_total{result="found|denied|below_threshold"}`
