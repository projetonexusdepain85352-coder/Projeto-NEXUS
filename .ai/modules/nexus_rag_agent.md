# Módulo: nexus_rag_agent

- Responsabilidade: agente RAG com verificador pós-geração.
  Recebe queries, busca no Qdrant, sintetiza com Ollama e
  verifica cada sentença da resposta contra os chunks recuperados.
- Binário: nexus_agent_server
- Endpoints: POST /query, GET /health
- Variáveis obrigatórias: NEXUS_OLLAMA_URL, NEXUS_BASE_MODEL,
  QDRANT_URL, POSTGRES_HOST, POSTGRES_PORT, POSTGRES_DB,
  POSTGRES_USER, KB_READER_PASSWORD, NEXUS_AGENT_PORT,
  VERIFIER_THRESHOLD
- Pontos de atenção para IA:
  - QDRANT_URL deve usar porta gRPC (6336), não REST (6335)
  - POSTGRES_HOST deve ser IP do gateway WSL, não localhost
    (descobrir com: ip route | grep default | awk '{print $3}')
  - Ollama deve estar rodando antes de iniciar o servidor
  - O verificador usa threshold configurável via VERIFIER_THRESHOLD
    (default 0.55); respostas com sentenças abaixo do threshold
    retornam GROUNDING_DENIED
