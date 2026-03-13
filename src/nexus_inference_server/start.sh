#!/usr/bin/env bash
set -euo pipefail

if [ -z "${NEXUS_BASE_MODEL:-}" ]; then
  echo "Erro: NEXUS_BASE_MODEL nao definido."
  exit 1
fi

python3 - <<'PY'
import importlib
import sys

modules = ["torch", "transformers", "peft", "bitsandbytes", "accelerate", "flask"]
missing = []
for m in modules:
    try:
        importlib.import_module(m)
    except Exception:
        missing.append(m)

if missing:
    print("Dependencias Python ausentes: " + ", ".join(missing))
    print("Execute: pip install -r requirements.txt")
    sys.exit(1)
PY

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec python3 "$DIR/server.py"
