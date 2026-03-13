# Nexus MTP

Pipeline de treino: extract -> train -> benchmark -> approve -> deploy.

## Variaveis de ambiente

PostgreSQL:
- `KB_INGEST_PASSWORD` (obrigatorio)
- `POSTGRES_HOST`, `POSTGRES_PORT`, `POSTGRES_DB`
- `POSTGRES_INGEST_USER` (default kb_ingest)
- `POSTGRES_USER` (fallback)

Treino e modelos:
- `NEXUS_BASE_MODEL` (override do modelo base)
- `NEXUS_MODELS_DIR` (default `/opt/nexus/models`)
- `NEXUS_BENCHMARK_MIN_SCORE` (default 0.7)

## Comandos CLI

### Extract

```
nexus_mtp extract --domain <rust|infra|security|mlops> --max-samples 1000
```

- Gera JSONL em `./datasets/<domain>_YYYYMMDD_HHMMSS.jsonl`.
- Gera sidecar `.ids` com IDs de documentos usados.

Formato do JSONL (1 linha por exemplo):
```
{"instruction":"Explique o seguinte conteudo tecnico:","input":"...","output":"","source":"..."}
```

### Train

```
nexus_mtp train --domain rust --dataset ./datasets/rust_YYYYMMDD_HHMMSS.jsonl --epochs 3 --lora-r 16
```

Parametros principais:
- `--domain` dominio.
- `--dataset` caminho do JSONL.
- `--base-model` (override do modelo base).
- `--epochs` (default 3).
- `--lora-r` (default 16; `lora_alpha = 2 * lora_r`).

Saida relevante:
- imprime `Modelo: <UUID>`.
- proximo passo sugerido: `nexus_mtp benchmark --model-id <UUID>`.

### Benchmark

```
nexus_mtp benchmark --model-id <UUID>
```

- Le perguntas em `benchmark_questions` (PostgreSQL).
- Executa inferencia via Python (HF + PEFT + BnB) e imprime `Score: <float>`.
- Atualiza `benchmark_score` do modelo.

### Approve

```
nexus_mtp approve
```

- Abre TUI para aprovacao humana.
- Atualiza `status` do modelo para `approved` ou `rejected`.

### Deploy

```
nexus_mtp deploy --model-id <UUID>
```

Regras:
- Exige `status=approved`.
- Exige `benchmark_score >= NEXUS_BENCHMARK_MIN_SCORE`.
- Cria link simbolico `models/<domain>/current` apontando para o adapter.

### Status

```
nexus_mtp status
```

Mostra contagens por dominio e modelo ativo.

### Stage A Gate

```
nexus_mtp stage-a-gate --min-security 35 --min-rust 80 --min-infra 100 --min-mlops 40 --min-total 300
```

Valida criterios de parada da Etapa A e retorna PASS/FAIL.

## Erros comuns

- `Dataset nao encontrado`: caminho invalido no train.
- `Benchmark missing`: modelo sem score atualizado.
- `Benchmark below threshold`: score abaixo do minimo.
- `Adapter not found`: path do adapter inexistente.

## Notas criticas (benchmark/trainer)

- Benchmark usa subprocesso Python (HF + PEFT + BnB).
- Prompt Alpaca obrigatorio:
  ```
  ### Instruction:
  {q}

  ### Response:
  ```
- Stop tokens recomendados: `eos_token_id = [2, 1542]` (EOS + `###`).
- `adapter_path` no banco deve ser relativo a `NEXUS_MODELS_DIR` (evitar `models/models/...`).
