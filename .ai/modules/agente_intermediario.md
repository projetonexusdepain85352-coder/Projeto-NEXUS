# Módulo: agente_intermediario

- Responsabilidade única: coletar conteúdo técnico externo e ingerir no banco.
- Entradas: fontes web/PDF configuradas no código.
- Saídas: registros em `documents` e estado inicial em `validation`.
- Dependências internas: alimenta `validador`, `nexus_rag` e `nexus_mtp`.
- Dependências externas: internet, PostgreSQL.
- Variáveis obrigatórias: `KB_INGEST_PASSWORD`.
- Pontos de atenção para IA:
  - conexão hardcoded em `src/main.rs:1004` (`localhost:5432`);
  - filtros de qualidade e deduplicação influenciam todo o pipeline;
  - mudanças de scraping exigem validação cuidadosa de regressão.
