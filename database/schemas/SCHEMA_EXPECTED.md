# NEXUS RAG — Schema esperado (referência apenas)

> **ATENÇÃO:** Este arquivo é documentação de referência.
> Não execute nenhum SQL daqui. O banco já existe com schema próprio.

O módulo `nexus_rag` espera as seguintes tabelas no banco `knowledge_base`:

## `documents`

| Coluna           | Tipo         | Notas                        |
|------------------|--------------|------------------------------|
| `id`             | UUID         | PK                           |
| `source`         | TEXT         | URL ou caminho de origem     |
| `domain`         | TEXT         | Ex: `rust`, `infra`          |
| `doc_type`       | TEXT         | Ex: `documentation`, `code`  |
| `content`        | TEXT         | Conteúdo completo            |
| `content_length` | INTEGER      | Tamanho em bytes             |
| `content_hash`   | TEXT         | Hash SHA-256 do conteúdo     |
| `collected_at`   | TIMESTAMPTZ  | Data de ingestão             |

## `validation`

| Coluna         | Tipo   | Notas                                          |
|----------------|--------|------------------------------------------------|
| `document_id`  | UUID   | FK → `documents.id`                            |
| `status`       | TEXT   | `'approved'` \| `'rejected'` \| `'pending'`   |

## Permissões necessárias
```sql
GRANT SELECT ON documents, validation TO kb_reader;
```

## Consultas utilizadas pelo módulo

- `SELECT ... FROM documents d INNER JOIN validation v ON v.document_id = d.id WHERE v.status = 'approved'`
- `SELECT d.domain, COUNT(*) FROM documents d INNER JOIN validation v ... GROUP BY d.domain`
