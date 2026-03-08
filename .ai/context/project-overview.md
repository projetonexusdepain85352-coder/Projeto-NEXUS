# Project Overview

## O que o sistema NEXUS faz
NEXUS é uma plataforma de IA privada focada em conhecimento validado.
O pipeline coleta documentos técnicos, submete validação humana, indexa conteúdo aprovado em RAG e permite treinar modelos especializados.

## Problemas que resolve
- Governança de conhecimento: evita resposta sem evidência validada.
- Operação auditável: separa coleta, validação, indexação e treino.
- Evolução incremental: permite avançar por etapas com gates explícitos.

## Módulos e relações
```text
[agente_intermediario] ---> [PostgreSQL: documents/validation] <--- [validador]
                                      |
                                      v
                               [nexus_rag] ---> [Qdrant]
                                      |
                                      v
                                 consultas grounded

[PostgreSQL validated data] ---> [nexus_mtp] ---> treino/benchmark/aprovação/deploy

[nexus_control_server] ---> start/stop/terminal dos serviços
```

## Stack tecnológica
- Linguagens: Rust e Python.
- Banco relacional: PostgreSQL.
- Banco vetorial: Qdrant.
- Interface operacional: servidor HTTP Python + frontend web.
- Scripts operacionais: Bash e PowerShell.

## Dependências externas para rodar
- Rust toolchain + Cargo.
- Python 3.
- PostgreSQL acessível com usuários de leitura/ingestão.
- Qdrant acessível por URL.
- (Opcional) Cloudflare `cloudflared` para exposição externa do painel.
- (Opcional) Docker/WSL para ambiente de cópia operacional.
