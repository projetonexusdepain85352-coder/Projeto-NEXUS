# Visao Geral

NEXUS e uma plataforma de IA privada focada em conhecimento validado. O objetivo e responder somente com base em documentos aprovados por validacao humana, evitando respostas sem evidencia.

## Principio central

Respostas 100% ancoradas em documentos validados. Nao ha fallback parametrico: se nao houver evidencia suficiente, o sistema nega a resposta.

## Stack tecnologica

- Linguagens: Rust e Python.
- Banco relacional: PostgreSQL (knowledge_base).
- Banco vetorial: Qdrant.
- Modelos locais: Ollama (Mistral).
- Observabilidade: logging com `tracing`.

## Por que 100% local

- Dados sensiveis permanecem no ambiente local.
- Controle total de privacidade e custos.
- Evolucao incremental do conhecimento sem depender de APIs externas.

## Estado atual do banco (snapshot)

Snapshot de validacao em `data/github_backups/container_sync/20260306_112657/validation_stats.txt`:
- Aprovados: 1394
- Rejeitados: 14
- Total: 1408

Dominios ativos no RAG (collections `nexus_*`): `infra`, `rust`, `mlops`, `security`.
