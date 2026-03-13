# API do Agente RAG

## Porta

- `8765`

## Endpoints

### GET /health

Resposta:
```
ok
```

### POST /query

Request JSON:
```
{"query":"...", "domain":"security"}
```

Campos:
- `query`: texto da pergunta.
- `domain`: opcional (`rust`, `infra`, `mlops`, `security`).

Response JSON completo:
```
{
  "response": "...",
  "sources": [...],
  "grounded": true,
  "denied_reason": null,
  "best_score": 0.75,
  "rejected_sentences": []
}
```

## denied_reason

- `no_chunks`: Qdrant não retornou chunks com score ≥ 0.35.
- `verifier_failed`: best_score do verificador abaixo do threshold.
- `insufficient_context`: modelo declarou falta de informação.

## Exemplos de curl

Grounded:
```
curl -s -X POST http://localhost:8765/query \
  -H 'Content-Type: application/json' \
  -d '{"query":"what is SQL injection?","domain":"security"}'
```

insufficient_context:
```
curl -s -X POST http://localhost:8765/query \
  -H 'Content-Type: application/json' \
  -d '{"query":"what is the history of the Roman Empire?","domain":"rust"}'
```

no_chunks:
```
curl -s -X POST http://localhost:8765/query \
  -H 'Content-Type: application/json' \
  -d '{"query":"receita de bolo","domain":"security"}'
```

## Variáveis de ambiente

- `NEXUS_OLLAMA_URL` (default http://localhost:11434)
- `NEXUS_BASE_MODEL` (default `mistral`)
- `VERIFIER_THRESHOLD` (default 0.55, recomendado 0.45)
- `QDRANT_URL` (gRPC 6336)
- `QDRANT_API_KEY` (opcional)
- `POSTGRES_HOST`, `POSTGRES_PORT`, `POSTGRES_DB`, `POSTGRES_USER`
- `KB_READER_PASSWORD`
- `NEXUS_AGENT_HOST`, `NEXUS_AGENT_PORT`
