# Database

[IMPLEMENTAÇÃO]
- Banco principal: PostgreSQL (`knowledge_base`).
- Artefatos versionados:
  - `database/migrations/001_mtp_schema.sql` (migração do pipeline MTP).
  - `database/schemas/SCHEMA_EXPECTED.md` (referência de schema esperado no RAG).
- Dumps e snapshots não são versionados; devem permanecer em `data/` local (ignorado pelo Git).

[OPERAÇÃO]
Setup básico em máquina limpa:
1. Criar banco e usuários (exemplo):
   - `createdb knowledge_base`
2. Aplicar migração:
   - `psql -d knowledge_base -f database/migrations/001_mtp_schema.sql`
3. Garantir permissões de leitura para `kb_reader`:
   - usar `config/scripts/ensure_permissions.sh`.

Seeds:
- `database/seeds/` está preparado para scripts de carga inicial.
- Atualmente não há seed versionado neste repositório.

Checklist rápido:
- Consegue conectar no banco com o usuário de ingestão.
- Consegue executar consulta simples com o usuário de leitura.
- Módulos `validador`, `nexus_rag` e `nexus_mtp` conseguem autenticar com as variáveis corretas.
