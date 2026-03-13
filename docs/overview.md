# Visão Geral

O NEXUS é uma plataforma de IA privada focada em conhecimento validado. O objetivo é responder somente com base em documentos aprovados por validação humana, evitando respostas sem evidência.

## Princípio central

Respostas 100% ancoradas em documentos validados. Não há fallback paramétrico: se não houver evidência suficiente, o sistema nega a resposta.

## Stack tecnológica

- Linguagens: Rust e Python.
- Banco relacional: PostgreSQL (knowledge_base).
- Banco vetorial: Qdrant.
- Modelos locais: Ollama (Mistral).
- Observabilidade: logging com `tracing`.

## Por que 100% local

- Dados sensíveis permanecem no ambiente local.
- Controle total de privacidade e custos.
- Evolução incremental do conhecimento sem depender de APIs externas.

## Estado atual do banco (snapshot)

Snapshot de validação em `data/github_backups/container_sync/20260306_112657/validation_stats.txt`:
- Aprovados: 1394
- Rejeitados: 14
- Total: 1408

Domínios ativos no RAG (collections `nexus_*`): `infra`, `rust`, `mlops`, `security`.
