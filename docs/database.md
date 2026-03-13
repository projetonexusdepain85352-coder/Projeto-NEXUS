# Banco de Dados

## PostgreSQL (knowledge_base)

Tabelas principais:

- `documents`: documento bruto coletado.
- `validation`: status de validacao (`approved`, `rejected`, `pending`).
- `training_cycles`: ciclos de treino (MTP).
- `models`: modelos e adapters treinados.
- `document_training_lineage`: vinculo documento -> ciclo de treino.
- `benchmark_questions`: perguntas usadas no benchmark.

Colunas essenciais (documentacao de referencia):

`documents`
- `id` (UUID)
- `source` (TEXT)
- `domain` (TEXT)
- `doc_type` (TEXT)
- `content` (TEXT)
- `content_length` (INTEGER)
- `content_hash` (TEXT)
- `collected_at` (TIMESTAMPTZ)

`validation`
- `document_id` (UUID FK)
- `status` (TEXT)

## Usuarios e permissoes

- `kb_admin`: administracao.
- `kb_ingest`: ingestao e escrita.
- `kb_reader`: leitura usada pelo RAG.

Permissoes minimas para `kb_reader`:
```
GRANT SELECT ON documents, validation TO kb_reader;
```

## Qdrant

Collections por dominio:
- `nexus_security`
- `nexus_rust`
- `nexus_infra`
- `nexus_mlops`

Cada ponto contem payload com:
- `document_id`, `source`, `domain`, `doc_type`
- `chunk_index`, `chunk_total`, `chunk_text`
