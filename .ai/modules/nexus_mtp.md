# Módulo: nexus_mtp

- Responsabilidade única: pipeline de dataset/treino/benchmark/aprovação/deploy.
- Entradas: documentos aprovados e arquivos de dataset.
- Saídas: ciclos de treino, métricas e artefatos de modelo.
- Dependências internas: base validada por `validador` e dados do `agente_intermediario`.
- Dependências externas: PostgreSQL, stack Rust ML (`candle`, `tokenizers`).
- Variáveis obrigatórias: `KB_INGEST_PASSWORD`.
- Pontos de atenção para IA:
  - conexão DB hardcoded em `src/main.rs:490` (avaliar refatoração);
  - validar caminhos de modelo/base antes de treinar;
  - manter gate de aprovação humana no fluxo.
