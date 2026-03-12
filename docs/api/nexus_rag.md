# NEXUS RAG Query API

## Visao geral
O `nexus_rag` expõe uma interface CLI para indexacao e consulta grounded.
A politica e STRICT_DB_ONLY: nenhum fallback parametric o.

## Comandos CLI
- `nexus_rag index`
- `nexus_rag query <texto> [--domain <dominio>] [--top <n>]`
- `nexus_rag status`

### Parametros do query
- `texto`: string de consulta (max 4096 chars)
- `--domain`: opcional (rust, infra, security, mlops)
- `--top`: quantidade de chunks retornados (default 5)

## Variaveis de ambiente
Qdrant:
- `QDRANT_URL` (obrigatorio)
- `QDRANT_API_KEY` (opcional)
- `NEXUS_ENV=production` exige URL https:// ou grpcs://

PostgreSQL (somente leitura):
- `KB_READER_PASSWORD` (obrigatorio)
- `POSTGRES_HOST` (default localhost)
- `POSTGRES_PORT` (default 5433)
- `POSTGRES_DB` (default knowledge_base)
- `POSTGRES_USER` (default kb_reader)

## Formato de saida (query)
O comando imprime resultados com os campos:
- `document_id`
- `source`
- `domain`
- `doc_type`
- `chunk_index` / `chunk_total`
- `chunk_text`
- `score`
- `collection`

Exemplo (resumo):
```
NEXUS RAG - Strict Grounded Results
Query  : ...
Scope  : ...
Policy : STRICT_DB_ONLY (no parametric fallback)
MinScore: 0.35
Found  : 3 evidence chunk(s)

#1 score=0.8123
  document_id : 123e4567-e89b-12d3-a456-426614174000
  source      : https://...
  domain/type : rust / rfc
  chunk       : 2/6 | collection=nexus_rust
  evidence:
    ...
```

## Politica STRICT_DB_ONLY
- Threshold fixo: `STRICT_MIN_SCORE = 0.35` (em `src/nexus_rag/src/query.rs`).
- Se nao houver evidencia no Qdrant, retorna erro `Ungrounded` e imprime:
  `GROUNDING_DENIED: no evidence found in database for this query.`
- Se houver evidencia mas todas abaixo do threshold, retorna erro `Ungrounded`.
- Se algum resultado vier sem `document_id`, a consulta e negada.

## Ajuste do threshold
Hoje o threshold e constante no codigo. Para alterar:
1. Edite `STRICT_MIN_SCORE` em `src/nexus_rag/src/query.rs`.
2. Recompile o binario.

## Exemplos
```bash
nexus_rag index

nexus_rag query "como funciona borrow checker" --domain rust --top 5

nexus_rag status
```

## Metrics
- Endpoint: `http://127.0.0.1:9898/metrics` (default)
- Override com `NEXUS_METRICS_ADDR` (ex: `0.0.0.0:9898`).
- Counter: `nexus_rag_queries_total{result="found|denied|below_threshold"}`
