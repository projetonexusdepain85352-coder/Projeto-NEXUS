# NEXUS MTP - Relatorio de Troubleshooting do Benchmark
> **Documento para IAs e desenvolvedores.** Este arquivo documenta todos os erros encontrados, tentativas falhas e solucoes confirmadas durante a execucao do benchmark do `nexus_mtp`. O objetivo e evitar repeticao dos mesmos problemas nos proximos ciclos.

---

## Indice

1. [Contexto do Ambiente](#1-contexto-do-ambiente)
2. [Linha do Tempo dos Problemas](#2-linha-do-tempo-dos-problemas)
3. [Problema Raiz Real](#3-problema-raiz-real)
4. [O Que Tentamos e Falhou](#4-o-que-tentamos-e-falhou)
5. [O Que Funcionou](#5-o-que-funcionou)
6. [Solucao Final Adotada](#6-solucao-final-adotada)
7. [Alteracoes Permanentes no Codigo](#7-alteracoes-permanentes-no-codigo)
8. [Referencia Rapida para Futuras IAs](#8-referencia-rapida-para-futuras-ias)
9. [Resultado Final Confirmado](#9-resultado-final-confirmado)
10. [Observacoes no Banco de Dados](#10-observacoes-no-banco-de-dados)

---

## 1. Contexto do Ambiente

| Item | Valor |
|---|---|
| OS | Windows 11 + WSL2 (Ubuntu 24) |
| GPU | NVIDIA RTX 4050 Laptop (6.141 MiB VRAM) |
| Python | 3.12.3 |
| Torch | 2.10.0+cu128 |
| CUDA | 12.8 |
| Transformers | 5.2.0 |
| Base model | `unsloth/mistral-7b-instruct-v0.3-bnb-4bit` |
| Dominio benchmark | infra |

---

## 2. Linha do Tempo dos Problemas

### Problema 1 - Candle incompativel com BnB 4-bit
- **Sintoma:** `shape mismatch [8388608, 1]` ao carregar `q_proj`.
- **Causa:** Candle (Rust puro) nao suporta modelos BitsAndBytes 4-bit.
- **Solucao:** trocar para subprocess Python com HF + PEFT + BnB.

### Problema 2 - `adapter_path` duplicado (`models/models/...`)
- **Sintoma:** benchmark nao encontrava o adapter.
- **Causa:** DB ja tinha `models/...` e o codigo concatenava com `NEXUS_MODELS_DIR` (que ja termina em `models/`).
- **Solucao:** corrigir valor no banco para **relativo** (ex.: `adapters/nexus-infra-hf-20260310`).

### Problema 3 - `python3 -c` corrompe scripts longos
- **Sintoma:** script quebrado por shell quoting e comportamento inesperado.
- **Causa:** linha longa passada no `-c` era truncada/alterada.
- **Solucao:** salvar o script em `/tmp/_nexus_benchmark_script.py` e executar via arquivo.

### Problema 4 - Prompt errado no generate
- **Sintoma:** respostas incoerentes e baixa pontuacao.
- **Causa:** benchmark usava `[INST] ... [/INST]` (formato Mistral), mas o treino foi Alpaca.
- **Solucao:** usar prompt Alpaca:
  ```
  ### Instruction:
  {question}

  ### Response:
  ```

### Problema 5 - Stop token `###` ausente
- **Sintoma:** modelo entra em loop repetindo `###`.
- **Causa:** geracao parava apenas no EOS (id 2).
- **Solucao:** `eos_token_id = [2, 1542]` (EOS + token `###`).

### Problema 6 - Logs do subprocess parecem "instantaneos"
- **Sintoma:** logs surgiam apenas no final, parecendo que o `generate` nao rodou.
- **Causa:** `wait_with_output()` bloqueia ate o processo terminar e so entao imprime `stderr`.
- **Solucao:** comportamento esperado; se precisar log em tempo real, usar streaming.

### Problema 7 - Keywords do benchmark nao batem com vocabulario do modelo
- **Sintoma:** respostas corretas, mas sem match de keywords.
- **Causa:** modelo responde em ingles/termos diferentes do set original.
- **Solucao:** atualizar keywords no banco para refletir vocabulario real do modelo.

### Nota adicional - Pergunta sobre modulos do kernel (Q4)
- **Sintoma:** modelo entra em loop repetindo "The kernel will then load the module".
- **Causa provavel:** poucos exemplos sobre `modprobe`/`insmod` no dataset de infra.
- **Acao recomendada:** adicionar documentos sobre carregamento de modulos no proximo ciclo.

---

## 3. Problema Raiz Real

O benchmark original dependia de Candle para inferencia, mas o modelo base e os adapters foram treinados com BitsAndBytes 4-bit. Essa combinacao e **incompativel** e leva a erro de shape no carregamento. A partir dai, outros problemas apareceram por incompatibilidade de prompt, stop tokens e parametros de banco.

---

## 4. O Que Tentamos e Falhou

- Manter Candle (Rust puro) para inferencia com BnB 4-bit.
- Rodar script de benchmark via `python3 -c` (quebra com script longo).
- Usar prompt `[INST] ... [/INST]` em modelo treinado com Alpaca.
- Gerar sem stop token `###` (loop infinito).
- Esperar logs em tempo real usando `wait_with_output()` (nao acontece).

---

## 5. O Que Funcionou

- Subprocesso Python com HF + PEFT + BnB, igual ao trainer.
- Escrever script temporario em `/tmp/_nexus_benchmark_script.py`.
- Prompt Alpaca estrito.
- Stop tokens `[2, 1542]` (EOS + `###`).
- Atualizar keywords do banco para o vocabulario real do modelo.

---

## 6. Solucao Final Adotada

O `benchmark.rs` foi reescrito para gerar e executar um script Python temporario com HF + PEFT + BnB. O prompt Alpaca e os stop tokens foram padronizados, e os dados do banco foram corrigidos.

---

## 7. Alteracoes Permanentes no Codigo

### `src/nexus_mtp/src/benchmark.rs`

- Subprocesso Python (nao Candle) para inferencia.
- Script salvo em `/tmp/_nexus_benchmark_script.py` (nao `-c`).
- Prompt fixo:
  ```
  ### Instruction:
  {question}

  ### Response:
  ```
- Stop tokens obrigatorios:
  ```
  eos_token_id = [2, 1542]
  ```
- Logs do Python capturados apos `wait_with_output()` (comportamento esperado).

---

## 8. Referencia Rapida para Futuras IAs

### [ALERTA] Armadilhas Criticas

| Armadilha | Sintoma | Solucao |
|---|---|---|
| Candle + BnB 4-bit | `shape mismatch` em `q_proj` | Usar Python HF + PEFT + BnB |
| `adapter_path` com `models/` | adapter nao encontrado | `adapter_path` deve ser relativo a `NEXUS_MODELS_DIR` |
| Prompt errado | respostas incoerentes | Alpaca: `### Instruction` + `### Response` |
| Stop token ausente | loop com `###` | `eos_token_id = [2, 1542]` |
| `python3 -c` | script corrompido | usar arquivo temporario |

### Checklist antes de rodar benchmark

- [ ] Adapter path no banco eh relativo (ex.: `adapters/<model>`).
- [ ] Benchmark usa Python subprocess (HF + PEFT + BnB).
- [ ] Prompt Alpaca e stop tokens configurados.
- [ ] Keywords do banco refletem vocabulario real do modelo.

---

## 9. Resultado Final Confirmado

- **Dominio:** infra
- **Modelo:** `nexus-infra-hf-20260310`
- **Score final:** 9/10 = 0.90
- **Data:** 2026-03-11

---

## 10. Observacoes no Banco de Dados

### `models`
- `adapter_path` corrigido para `adapters/nexus-infra-hf-20260310`
- `status = deployed`
- `benchmark_score = 0.90`

### `benchmark_questions` (dominio infra)

- "Quais mecanismos base de isolamento..." -> `{namespaces,cgroups,isolation,resources}`
- "Qual e o objetivo principal do VACUUM" -> `{vacuum,storage,reclaim,rows}`
- "Qual funcao SQL recarrega..." -> `{reload,postgresql,server,libraries}`
- "Quais comandos sao usados para carregar..." -> `{kernel,module,load,runtime}`
- "No Dockerfile, qual a diferenca CMD/ENT" -> `{entrypoint,cmd,executado,imagem}`
- "O que o comando systemctl enable faz" -> `{systemctl,enable,boot,unit}`
- "Para que serve o journalctl" -> `{journal,systemd,log,binary}`
- "Qual arquivo do PostgreSQL controla auth" -> `{authentication,postgresql,methods,section}`
- "Quais secoes aparecem em unit file" -> `{unit,service,common,sections}`
- "O que a instrucao EXPOSE faz" -> `{expose,port,publish,container}`

---

*Gerado em 2026-03-11. Ambiente: WSL2, RTX 4050 6GB, Torch 2.10.0, Transformers 5.2.0, BnB 4-bit.*