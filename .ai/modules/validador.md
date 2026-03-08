# Módulo: validador

- Responsabilidade única: validar documentos e decidir status de aprovação.
- Entradas: documentos pendentes no PostgreSQL e interação humana.
- Saídas: atualização de `validation` (`approved`/`rejected`/`pending`) e estado de sessão.
- Dependências internas: depende do que `agente_intermediario` coleta; alimenta `nexus_rag` e `nexus_mtp`.
- Dependências externas: PostgreSQL, opcional endpoint local de IA (Ollama).
- Variáveis obrigatórias: `KB_INGEST_PASSWORD`.
- Variáveis relevantes: `POSTGRES_HOST`, `POSTGRES_PORT`, `POSTGRES_DB`, `POSTGRES_USER`.
- Pontos de atenção para IA:
  - defaults de DB estão em `src/main.rs:3052`;
  - o fluxo de sessão/revalidação é sensível a concorrência;
  - preservar comandos da TUI esperados por operadores.
