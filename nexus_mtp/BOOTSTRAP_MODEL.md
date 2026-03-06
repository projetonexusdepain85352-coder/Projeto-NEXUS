# Bootstrap Model (Dev Only)

Date downloaded: 2026-03-06
Model: `Qwen/Qwen2.5-0.5B-Instruct`

Local path (Windows):
`C:\Users\dulan\OneDrive\Documentos\GitHub\Projeto-NEXUS\models\bootstrap\qwen2.5-0.5b-instruct`

Local path (WSL):
`/mnt/c/Users/dulan/OneDrive/Documentos/GitHub/Projeto-NEXUS/models/bootstrap/qwen2.5-0.5b-instruct`

## Use in NEXUS MTP

Option 1 (recommended for session):

```bash
export NEXUS_BASE_MODEL="/mnt/c/Users/dulan/OneDrive/Documentos/GitHub/Projeto-NEXUS/models/bootstrap/qwen2.5-0.5b-instruct"
nexus_mtp train --domain infra --dataset ./datasets/infra_YYYYMMDD_HHMMSS.jsonl --epochs 1 --lora-r 8
```

Option 2 (one-off command):

```bash
nexus_mtp train \
  --domain infra \
  --dataset ./datasets/infra_YYYYMMDD_HHMMSS.jsonl \
  --base-model /mnt/c/Users/dulan/OneDrive/Documentos/GitHub/Projeto-NEXUS/models/bootstrap/qwen2.5-0.5b-instruct
```

## Notes

- This model is for bootstrap/development only.
- Production model flow must still pass your MTP approval/deploy pipeline.
