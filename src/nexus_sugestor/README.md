# Nexus Sugestor

[IMPLEMENTAÇÃO]
Este módulo é um servidor Python simples (`servidor.py`) que recebe texto via socket UNIX (`/tmp/nexus_sugestor.sock`) e classifica utilidade/confiança do conteúdo para treino, usando API local do Ollama (`http://localhost:11434/api/generate`) com modelo `mistral`.

Decisões técnicas:
- IPC local por `AF_UNIX` para integração rápida com o `validador`.
- Prompt fixo para forçar resposta JSON curta (`util`, `confianca`, `motivo`).
- Fallback defensivo quando a resposta do modelo não vem em JSON válido.

Dependências externas:
- Python 3.
- Ollama ativo localmente com modelo compatível.

[OPERAÇÃO]
Execução isolada:
```bash
python3 src/nexus_sugestor/servidor.py
```

Integração típica:
- É iniciado por `config/scripts/iniciar_validador.sh` antes do binário do validador.
- O socket usado é `/tmp/nexus_sugestor.sock`.

Variáveis de ambiente:
- Atualmente não há variáveis obrigatórias no código; URL/modelo estão fixos no arquivo.
- Se for necessário parametrizar ambiente, externalizar `OLLAMA_URL` e `OLLAMA_MODEL` para variáveis de ambiente.
