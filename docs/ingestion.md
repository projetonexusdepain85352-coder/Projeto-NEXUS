# Pipeline de Ingestão

## Fluxo geral

```
agente_intermediario → PostgreSQL (documents/validation) → nexus_validador
→ nexus_rag (indexação) → Qdrant (nexus_<domain>)
```

## Domínios e collections

- `security` → `nexus_security`
- `rust` → `nexus_rust`
- `infra` → `nexus_infra`
- `mlops` → `nexus_mlops`

## Chunking

- 400 palavras por chunk
- overlap de 50 palavras
- definido em `nexus_rag/src/indexer.rs`

## Score mínimo de busca

- `STRICT_MIN_SCORE = 0.35` em `nexus_rag/src/query.rs`

## Adicionar novos documentos

1. Coletar com `agente_intermediario` e inserir em `documents`.
2. Aprovar no `nexus_validador` (status `approved`).
3. Rodar `nexus_rag index` para subir chunks no Qdrant.

Não há necessidade de retreinar o modelo para que novos documentos sejam usados pelo agente.
