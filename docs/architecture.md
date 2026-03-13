# Arquitetura

## Pipeline completo (texto)

```
coleta (agente_intermediario)
  → PostgreSQL (documents/validation)
    → validação humana (nexus_validador)
      → indexação RAG (nexus_rag)
        → Qdrant (nexus_<domain>)
          → agente RAG (nexus_rag_agent)
```

## Validador e Sugestor

- `nexus_validador` é uma TUI em Rust.
- `nexus_sugestor` é um serviço Python opcional, acessado via socket UNIX.
- Socket padrão: `/tmp/nexus_sugestor.sock` (configurável em `NEXUS_SUGESTOR_SOCKET`).
- O sugestor consulta o Ollama local e responde JSON com `util`, `confianca`, `motivo`.

## Controle de serviços

- `nexus_control_server` é um backend HTTP em Python para iniciar/parar serviços.
- Usa `services.json` para descrever comandos e ambiente de cada serviço.

## Fluxo do nexus_rag_agent

```
query
  → Qdrant (STRICT_MIN_SCORE = 0.35, top_k = 5)
  → prompt com [CHUNK_X]
  → Ollama (Mistral)
  → verificador holístico (best_score >= VERIFIER_THRESHOLD)
  → resposta grounded ou GROUNDING_DENIED
```

### denied_reason

- `no_chunks`: Qdrant não retornou evidências acima do threshold.
- `verifier_failed`: best_score do verificador abaixo do threshold.
- `insufficient_context`: modelo declarou falta de informação.

## Verificador holístico

- Embeda a resposta inteira e compara com cada chunk recuperado.
- `best_score` = maior similaridade cosine com qualquer chunk.
- `supported = best_score >= VERIFIER_THRESHOLD`.
- `VERIFIER_THRESHOLD` default 0.55 (recomendado 0.45 em `.env`).

## Citation Engine

- Contexto enviado ao modelo contém chunks numerados: `[CHUNK_1] ...`.
- O prompt exige citação `[CHUNK_X]` por sentença.
- O verificador remove tags `[CHUNK_X]` antes de embedar.

## Qdrant

- Collections por domínio: `nexus_security`, `nexus_rust`, `nexus_infra`, `nexus_mlops`.
- URL obrigatória via `QDRANT_URL` (gRPC 6336).

## Embedding e chunking

- Modelo: `all-MiniLM-L6-v2`
- Dimensão: 384
- Chunking: 400 palavras com overlap 50 (em `nexus_rag/src/indexer.rs`)
