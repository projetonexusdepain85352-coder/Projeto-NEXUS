# NEXUS MTP - Relatorio de Troubleshooting do Trainer

Documento para IAs e desenvolvedores. Registra erros, tentativas falhas e solucoes confirmadas durante o primeiro ciclo real de treino do `nexus_mtp`.

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
10. [Acao Requerida no Codigo](#10-acao-requerida-no-codigo)

---

## 1. Contexto do Ambiente

| Item | Valor |
|---|---|
| Hardware | NVIDIA GeForce RTX 4050 Laptop GPU |
| VRAM total | 6.141 MiB (~5.997 GB disponiveis para o processo) |
| OS | Windows 11 + WSL2 (Ubuntu 24) |
| Python | 3.12.3 |
| Torch | 2.10.0+cu128 |
| CUDA Toolkit | 12.8 |
| Triton | 3.6.0 |
| Unsloth | 2026.3.4 |
| Transformers | 5.2.0 |
| TRL | versao com `SFTConfig` (nova API) |
| Modelo base | `unsloth/mistral-7b-instruct-v0.3-bnb-4bit` |
| Dataset | `datasets/infra_20260309_164905.jsonl` - 2.111 exemplos |
| LoRA | r=16, alpha=32, dropout=0.05, target: q/k/v/o_proj |
| Epochs | 3 - total de 792 steps |

Uso de VRAM apos carregar o modelo: ~5.843 MiB de 6.141 MiB - sobram apenas ~300 MB livres para o forward pass.

---

## 2. Linha do Tempo dos Problemas

### Ciclo 1 - `nexus-infra-20260309_230718` (PID 4351)
- Rodou por 70+ minutos sem completar nenhum step
- Diretorio de output vazio - nenhum `checkpoint-*` ou `trainer_state.json`
- Log parado em `Trainable parameters = 13,631,488`
- GPU a 100%, 5.843 MiB ocupados
- Diagnostico inicial (errado): TorchInductor travando compilacao

### Ciclo 2 - `nexus-infra-20260310_003053` (PID 5939)
- Apos adicionar `TORCHINDUCTOR_DISABLE=1` e `TORCH_COMPILE_DISABLE=1`
- Mesmo resultado: GPU 100%, diretorio vazio, log parado no mesmo ponto
- Rodou por mais 50 minutos sem steps
- Processo morreu silenciosamente sem log de erro (stderr estava sendo engolido pelo pipe do Rust)

### Diagnostico Real - Descoberto rodando o script diretamente

Ao rodar `python3 _train_script.py` diretamente (sem o pipe do `nexus_mtp`), o erro real apareceu:

```
RuntimeError: Unsloth: No or negligible GPU memory available for fused cross entropy.
```

O processo estava travando no primeiro forward pass, nao na compilacao. O erro ocorria em:

```
unsloth_zoo/fused_losses/cross_entropy_loss.py -> _get_chunk_multiplier()
```

---

## 3. Problema Raiz Real

### Causa Principal

O Unsloth 2026.3.4 com Mistral usa uma implementacao propria de cross entropy chamada `unsloth_fused_ce_loss`, hardcoded em `unsloth/models/mistral.py` linha 330. Essa funcao verifica a VRAM livre em tempo de execucao:

```
free, total = torch.cuda.mem_get_info(0)
free_gb = free / 1024 / 1024 / 1024
free_gb = free_gb * 0.5   # usa apenas 50% da VRAM livre
target_gb = free_gb

if target_gb <= 1e-9:
    raise RuntimeError("Unsloth: No or negligible GPU memory available for fused cross entropy.")
```

Com o modelo ocupando ~5.843 MiB dos 6.141 MiB disponiveis, restam ~298 MB livres. O calculo resulta em `0.298 * 0.5 = 0.149 GB`, tornando o chunking inviavel para o tamanho de sequencia configurado.

### Por Que o Processo Ficava "Travado"

O Unsloth usa `@torch.compile` com `unsloth_compiled_cache/` gerado no cwd do processo. O processo passava todo o tempo compilando o primeiro step, que imediatamente falhava com OOM - mas o erro era silenciado pelo pipe do `nexus_mtp`.

### Por Que o Stderr Estava Sendo Perdido

O `nexus_mtp` capturava stdout e stderr do processo filho. O traceback estava sendo capturado, mas como o processo morria antes de o Rust gravar no log, o erro nunca aparecia no arquivo.

---

## 4. O Que Tentamos e Falhou

### [FAIL] Tentativa 1: Variaveis de ambiente de compilacao

```
.env("TORCHINDUCTOR_DISABLE", "1")
.env("TORCH_COMPILE_DISABLE", "1")
.env("UNSLOTH_COMPILE_DISABLE", "1")
.env("UNSLOTH_ENABLE_CCE", "0")
```

Falhou: o problema nao era compilacao - era OOM na fused cross entropy.

### [FAIL] Tentativa 2: `UNSLOTH_USE_FUSED_CE=0`

Falhou: o Unsloth 2026.3.4 ignora essa variavel.

### [FAIL] Tentativa 3: Reduzir `max_seq_length` para 512

Falhou: o consumo de VRAM do modelo quantizado em 4bit nao muda com `max_seq_length`.

### [FAIL] Tentativa 4: Patch em `cross_entropy_loss.py`

```
sed -i 's/free_gb = free_gb * 0.5/free_gb = free_gb * 0.25/' ...
```

Falhou: o Unsloth regenera `unsloth_compiled_cache/UnslothSFTTrainer.py` no cwd a cada execucao.

### [FAIL] Tentativa 5: Patch em `mistral.py` + limpeza de cache

```
sed -i 's/torch_compile = True,/torch_compile = False,/' ...
rm -rf unsloth_compiled_cache/
```

Falhou: o Unsloth regenerou o cache imediatamente.

### [FAIL] Tentativa 6: `SFTTrainer(tokenizer=tokenizer)`

Falhou: `TypeError: unexpected keyword argument 'tokenizer'` - parametro renomeado para `processing_class`.

### [FAIL] Tentativa 7: `SFTConfig(max_seq_length=1024)`

Falhou: `TypeError: unexpected keyword argument 'max_seq_length'` - parametro renomeado para `max_length`.

---

## 5. O Que Funcionou

### [OK] Diagnostico: Rodar o script Python diretamente

Em vez de `cargo run ... | tee log`, rodar:

```
python3 models/training/<ciclo>/_train_script.py
```

### [OK] Solucao: HuggingFace puro + PEFT + TRL (nova API)

```
import os
os.environ["TORCHINDUCTOR_DISABLE"] = "1"
os.environ["TORCH_COMPILE_DISABLE"] = "1"

import torch
from peft import LoraConfig, get_peft_model
from transformers import AutoModelForCausalLM, AutoTokenizer, BitsAndBytesConfig
from trl import SFTTrainer, SFTConfig
from datasets import load_dataset

MODEL_PATH = "/home/dulan/.cache/huggingface/hub/models--unsloth--mistral-7b-instruct-v0.3-bnb-4bit/snapshots/d5f623888f1415cf89b5c208d09cb620694618ee"

model = AutoModelForCausalLM.from_pretrained(
    MODEL_PATH,
    quantization_config=BitsAndBytesConfig(
        load_in_4bit=True,
        bnb_4bit_compute_dtype=torch.bfloat16,
        bnb_4bit_use_double_quant=True,
        bnb_4bit_quant_type="nf4",
    ),
    device_map="auto",
)

tokenizer = AutoTokenizer.from_pretrained(MODEL_PATH)

tokenizer.pad_token = tokenizer.eos_token

model = get_peft_model(model, LoraConfig(
    r=16, lora_alpha=32,
    target_modules=["q_proj", "k_proj", "v_proj", "o_proj"],
    lora_dropout=0.05, bias="none", task_type="CAUSAL_LM",
))

trainer = SFTTrainer(
    model=model,
    processing_class=tokenizer,
    train_dataset=dataset,
    args=SFTConfig(
        dataset_text_field="text",
        max_length=1024,
        per_device_train_batch_size=1,
        gradient_accumulation_steps=8,
        warmup_steps=100,
        num_train_epochs=3,
        learning_rate=2e-4,
        fp16=False, bf16=True,
        output_dir="<OUTPUT_DIR>",
        save_steps=100, logging_steps=10,
        report_to="none",
    ),
)

trainer.train()
model.save_pretrained("<ADAPTER_DIR>")
tokenizer.save_pretrained("<ADAPTER_DIR>")
print("NEXUS_TRAINING_COMPLETE", flush=True)
```

### [OK] Formato de prompt Alpaca + stop tokens (### / 1542)

Prompt minimo:

```
### Instruction:
{pergunta}

### Response:
```

Stop tokens recomendados:

```
eos_token_id = [2, 1542]  # EOS + token "###"
```

---

## 6. Solucao Final Adotada

O template do `_train_script.py` em `src/nexus_mtp/src/trainer.rs` foi migrado para HuggingFace puro + PEFT + TRL (nova API `SFTConfig`). O Unsloth foi removido completamente do pipeline de treino.

---

## 7. Alteracoes Permanentes no Codigo

### `src/nexus_mtp/src/trainer.rs` - envvars injetadas no processo filho

```
.env("TORCHINDUCTOR_DISABLE", "1")
.env("TORCH_COMPILE_DISABLE", "1")
.env("UNSLOTH_COMPILE_DISABLE", "1")
.env("UNSLOTH_ENABLE_CCE", "0")
.env("UNSLOTH_CE_LOSS_N_CHUNKS", "4096")
```

### Patches aplicados no Unsloth (workarounds, nao solucao permanente)

```
unsloth/models/mistral.py linha 340: torch_compile = False  (era True)
unsloth_zoo/fused_losses/cross_entropy_loss.py linha 123: free_gb * 0.25  (era 0.5)
```

---

## 8. Referencia Rapida para Futuras IAs

### Armadilhas Criticas

| Armadilha | Sintoma | Solucao |
|---|---|---|
| Unsloth fused CE com ~6GB VRAM | GPU 100%, nenhum step, sem log de erro | Usar HuggingFace puro + PEFT |
| `unsloth_compiled_cache/` regenerado | Patches no site-packages sem efeito | Cache e regenerado no cwd; fonte nao e relido |
| `UNSLOTH_USE_FUSED_CE=0` | Sem efeito | Variavel ignorada no Unsloth 2026.3.4 |
| `SFTTrainer(tokenizer=...)` | `TypeError` imediato | Usar `processing_class=tokenizer` |
| `SFTConfig(max_seq_length=...)` | `TypeError` imediato | Usar `max_length=` |
| Stderr perdido no pipe do Rust | Processo morre silenciosamente | Rodar `python3 _train_script.py` direto |
| Prompt fora do formato Alpaca | Respostas incoerentes/score baixo | Usar `### Instruction ... ### Response` |
| Stop token `###` ausente | Loop repetindo `###` | `eos_token_id = [2, 1542]` |

### Checklist Antes de Iniciar um Ciclo de Treino

- [ ] Template usa HuggingFace puro (nao Unsloth)
- [ ] `SFTTrainer` usa `processing_class=tokenizer`
- [ ] `SFTConfig` usa `max_length=` (nao `max_seq_length=`)
- [ ] Testar script gerado diretamente com `python3 _train_script.py`
- [ ] Verificar apos 5 min se `checkpoint-100` aparece no diretorio de output

### Diagnostico Rapido

```
python3 models/training/<ciclo_id>/_train_script.py 2>&1 | tail -30
ls -la models/training/<ciclo_id>/
nvidia-smi
```

---

## 9. Resultado Final Confirmado

### Ciclo Bem-Sucedido - `nexus-infra-hf-20260310`

| Metrica | Valor |
|---|---|
| Steps | 792 / 792 |
| Epochs | 3 |
| Tempo | ~6 horas |
| Loss inicial | 1.833 |
| Loss final | 1.187 (-35%) |
| Acuracia tokens (inicio) | 61.4% |
| Acuracia tokens (fim) | 71.7% |
| Train loss medio | 1.372 |

### Adapter Salvo

```
models/adapters/nexus-infra-hf-20260310/
+-- adapter_model.safetensors   # 54 MB
+-- adapter_config.json         # r=16, alpha=32
+-- tokenizer.json
+-- tokenizer_config.json
+-- chat_template.jinja
+-- README.md
```

---

## 10. Acao Requerida no Codigo

O template do `trainer.rs` foi migrado para HuggingFace puro. Os proximos dominios a treinar sao: `rust` (1.884 exemplos), `mlops` (277), `security` (204).

---

Gerado em 2026-03-10. Ambiente: WSL2, RTX 4050 6GB, Unsloth 2026.3.4, Transformers 5.2.0, TRL nova API.
