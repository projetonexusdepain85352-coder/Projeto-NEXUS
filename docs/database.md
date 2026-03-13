# Banco de Dados

## PostgreSQL (knowledge_base)

Tabelas principais:

- `documents`: documento bruto coletado.
- `validation`: status de validação (`approved`, `rejected`, `pending`).
- `training_cycles`: ciclos de treino (MTP).
- `models`: modelos e adapters treinados.
- `document_training_lineage`: vínculo documento → ciclo de treino.
- `benchmark_questions`: perguntas usadas no benchmark (referenciado em `nexus_mtp`).

Colunas essenciais (documentação de referência):

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

## Usuários e permissões

- `kb_admin`: administração.
- `kb_ingest`: ingestão e escrita.
- `kb_reader`: leitura usada pelo RAG.

Permissões mínimas para `kb_reader`:
```
GRANT SELECT ON documents, validation TO kb_reader;
```

## Qdrant

Collections por domínio:
- `nexus_security`
- `nexus_rust`
- `nexus_infra`
- `nexus_mlops`

Cada ponto contém payload com:
- `document_id`, `source`, `domain`, `doc_type`
- `chunk_index`, `chunk_total`, `chunk_text`
