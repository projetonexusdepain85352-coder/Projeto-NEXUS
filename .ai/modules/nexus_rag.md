# Módulo: nexus_rag

- Responsabilidade única: indexar e consultar conhecimento validado com grounding estrito.
- Entradas: documentos `approved` no PostgreSQL e parâmetros de consulta.
- Saídas: pontos indexados no Qdrant e respostas com evidências.
- Dependências internas: dados produzidos por `agente_intermediario` e decisões do `validador`.
- Dependências externas: PostgreSQL, Qdrant, `fastembed`.
- Variáveis obrigatórias: `KB_READER_PASSWORD`, `QDRANT_URL`.
- Pontos de atenção para IA:
  - respeitar a política de grounding (`STRICT_DB_ONLY`);
  - não relaxar filtros de confiança sem justificativa;
  - validar compatibilidade entre schema esperado e banco real.
