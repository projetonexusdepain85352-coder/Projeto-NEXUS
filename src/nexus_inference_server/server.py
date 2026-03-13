#!/usr/bin/env python3
import os
import logging
from typing import Optional

from flask import Flask, request, jsonify

import torch
from transformers import AutoModelForCausalLM, AutoTokenizer, BitsAndBytesConfig
from peft import PeftModel

app = Flask(__name__)

logging.basicConfig(level=logging.INFO, format="[nexus_inference] %(message)s")
logger = logging.getLogger("nexus_inference")

MODEL: Optional[torch.nn.Module] = None
TOKENIZER: Optional[AutoTokenizer] = None

DEFAULT_BASE_MODEL = "unsloth/mistral-7b-instruct-v0.3-bnb-4bit"
MAX_NEW_TOKENS = 512
TEMPERATURE = 0.1
REPETITION_PENALTY = 1.1


def resolve_model_path(model_id: str) -> str:
    import pathlib

    if os.path.isdir(model_id):
        return model_id
    home = os.environ.get("HOME", "/root")
    slug = model_id.replace("/", "--")
    snaps = pathlib.Path(home) / ".cache" / "huggingface" / "hub" / f"models--{slug}" / "snapshots"
    if snaps.exists():
        for snap in snaps.iterdir():
            if snap.is_dir():
                return str(snap)
    return model_id


def format_prompt(system: str, user: str) -> str:
    return f"[INST] {system}\n\n{user} [/INST]"


def load_model():
    global MODEL, TOKENIZER

    base_model = os.environ.get("NEXUS_BASE_MODEL", DEFAULT_BASE_MODEL).strip()
    if not base_model:
        raise RuntimeError("NEXUS_BASE_MODEL nao definido")

    adapter_path = os.environ.get("NEXUS_ADAPTER_PATH")
    model_path = resolve_model_path(base_model)

    logger.info("Carregando modelo base: %s", model_path)

    use_bnb = "bnb-4bit" not in base_model

    if use_bnb:
        bnb_config = BitsAndBytesConfig(
            load_in_4bit=True,
            bnb_4bit_compute_dtype=torch.bfloat16,
            bnb_4bit_use_double_quant=True,
            bnb_4bit_quant_type="nf4",
        )

        tokenizer = AutoTokenizer.from_pretrained(model_path)
        if tokenizer.pad_token is None:
            tokenizer.pad_token = tokenizer.eos_token

        base = AutoModelForCausalLM.from_pretrained(
            model_path,
            quantization_config=bnb_config,
            device_map="auto",
            torch_dtype=torch.bfloat16,
        )
    else:
        tokenizer = AutoTokenizer.from_pretrained(
            model_path,
            trust_remote_code=True,
        )
        if tokenizer.pad_token is None:
            tokenizer.pad_token = tokenizer.eos_token

        base = AutoModelForCausalLM.from_pretrained(
            model_path,
            device_map="auto",
            torch_dtype=torch.float16,
            trust_remote_code=True,
        )

    model = base
    adapter_loaded = False
    if adapter_path and adapter_path.strip():
        if os.path.isdir(adapter_path):
            logger.info("Carregando adapter LoRA: %s", adapter_path)
            model = PeftModel.from_pretrained(base, adapter_path)
            adapter_loaded = True
        else:
            logger.warning("Adapter nao encontrado em %s; usando modelo base.", adapter_path)

    model.eval()

    logger.info("Modelo pronto. Adapter aplicado: %s", "sim" if adapter_loaded else "nao")

    MODEL = model
    TOKENIZER = tokenizer


@app.get("/health")
def health():
    return jsonify({"status": "ok"})


@app.post("/api/generate")
def generate():
    if MODEL is None or TOKENIZER is None:
        return jsonify({"error": "model_not_loaded"}), 500

    payload = request.get_json(silent=True) or {}
    prompt = payload.get("prompt", "")
    if not isinstance(prompt, str) or not prompt.strip():
        return jsonify({"error": "prompt_required"}), 400

    system_prompt = os.environ.get("NEXUS_SYSTEM_PROMPT", "")
    full_prompt = format_prompt(system_prompt, prompt)

    tokenizer = TOKENIZER
    model = MODEL

    inputs = tokenizer(full_prompt, return_tensors="pt")
    inputs = {k: v.to(model.device) for k, v in inputs.items()}

    with torch.no_grad():
        output = model.generate(
            **inputs,
            max_new_tokens=MAX_NEW_TOKENS,
            do_sample=True,
            temperature=TEMPERATURE,
            repetition_penalty=REPETITION_PENALTY,
            pad_token_id=tokenizer.eos_token_id,
            eos_token_id=tokenizer.eos_token_id,
        )

    gen_ids = output[0][inputs["input_ids"].shape[1]:]
    response_text = tokenizer.decode(gen_ids, skip_special_tokens=True)

    return jsonify({"response": response_text})


if __name__ == "__main__":
    load_model()
    port = int(os.environ.get("NEXUS_INFERENCE_PORT", "11434"))
    host = os.environ.get("NEXUS_INFERENCE_HOST", "0.0.0.0")
    logger.info("Servidor iniciado em %s:%s", host, port)
    app.run(host=host, port=port)