use sqlx::PgPool;
use std::env;
use tracing::{info, warn};
use uuid::Uuid;

use crate::error::{MtpError, Result};

#[derive(Debug, sqlx::FromRow)]
struct BenchmarkQuestion {
    question: String,
    expected_keywords: Vec<String>,
}

pub const DEFAULT_BENCHMARK_MIN_SCORE: f32 = 0.7;

pub fn benchmark_min_score() -> f32 {
    match env::var("NEXUS_BENCHMARK_MIN_SCORE") {
        Ok(raw) => match raw.trim().parse::<f32>() {
            Ok(val) if (0.0..=1.0).contains(&val) => val,
            _ => {
                warn!("Valor invalido para NEXUS_BENCHMARK_MIN_SCORE; usando padrao");
                DEFAULT_BENCHMARK_MIN_SCORE
            }
        },
        Err(_) => DEFAULT_BENCHMARK_MIN_SCORE,
    }
}

pub fn benchmark_passed(score: f32, min_score: f32) -> bool {
    score >= min_score
}
pub async fn run_benchmark(
    pool: &PgPool,
    model_id: Uuid,
    adapter_path: &str,
    base_model: &str,
    domain: &str,
) -> Result<f32> {
    let questions = fetch_benchmark_questions(pool, domain).await;
    let questions = match questions {
        Err(e) => {
            warn!(
                "Benchmarks indisponiveis para '{}': {}. Score = 0.0",
                domain, e
            );
            return Ok(0.0);
        }
        Ok(v) if v.is_empty() => {
            warn!(
                "Nenhuma pergunta de benchmark para '{}'. Score = 0.0",
                domain
            );
            return Ok(0.0);
        }
        Ok(v) => v,
    };

    info!(
        "{} perguntas para '{}' (modelo {})...",
        questions.len(),
        domain,
        model_id
    );

    run_python_benchmark(&questions, base_model, adapter_path)
}

async fn fetch_benchmark_questions(
    pool: &PgPool,
    domain: &str,
) -> std::result::Result<Vec<BenchmarkQuestion>, sqlx::Error> {
    sqlx::query_as::<_, BenchmarkQuestion>(
        "SELECT question, expected_keywords FROM benchmark_questions WHERE domain = $1 ORDER BY id",
    )
    .bind(domain)
    .fetch_all(pool)
    .await
}

fn run_python_benchmark(
    questions: &[BenchmarkQuestion],
    base_model: &str,
    adapter_path: &str,
) -> Result<f32> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let questions_json = serde_json::to_string(
        &questions
            .iter()
            .map(|q| {
                serde_json::json!({
                    "question": q.question,
                    "expected_keywords": q.expected_keywords,
                })
            })
            .collect::<Vec<_>>(),
    )
    .map_err(|e| MtpError::Other(format!("serialize questions: {e}")))?;

    let script = build_benchmark_script(base_model, adapter_path);

    // Salva script em arquivo temporario — evita corrupcao de args com python3 -c
    let script_path = std::path::PathBuf::from("/tmp/_nexus_benchmark_script.py");
    std::fs::write(&script_path, script.as_bytes())
        .map_err(|e| MtpError::Other(format!("write script: {e}")))?;

    info!("Iniciando inferencia Python (HF+PEFT+BnB)...");
    info!("Base model: {}", base_model);
    info!("Adapter:    {}", adapter_path);

    let mut child = Command::new("python3")
        .arg(&script_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("TORCHINDUCTOR_DISABLE", "1")
        .env("TORCH_COMPILE_DISABLE", "1")
        .spawn()
        .map_err(|e| MtpError::Other(format!("spawn python3: {e}")))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(questions_json.as_bytes())
            .map_err(|e| MtpError::Other(format!("write stdin: {e}")))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| MtpError::Other(format!("wait python3: {e}")))?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    for line in stderr.lines() {
        if !line.trim().is_empty() {
            info!("[py] {}", line);
        }
    }

    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        return Err(MtpError::Other(format!(
            "benchmark Python falhou (exit {}). Stderr:\n{}",
            code,
            &stderr[stderr.len().saturating_sub(2000)..]
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        if let Some(val) = line.strip_prefix("NEXUS_BENCHMARK_SCORE=") {
            return val
                .trim()
                .parse::<f32>()
                .map_err(|e| MtpError::Other(format!("parse score '{}': {e}", val.trim())));
        }
    }

    Err(MtpError::Other(format!(
        "NEXUS_BENCHMARK_SCORE nao encontrado no stdout. Stdout:\n{}",
        &stdout[stdout.len().saturating_sub(2000)..]
    )))
}

fn build_benchmark_script(base_model: &str, adapter_path: &str) -> String {
    format!(
        r####"
import os
os.environ["TORCHINDUCTOR_DISABLE"] = "1"
os.environ["TORCH_COMPILE_DISABLE"] = "1"

import sys
import json
import torch
from transformers import AutoModelForCausalLM, AutoTokenizer, BitsAndBytesConfig
from peft import PeftModel

BASE_MODEL  = "{base_model}"
ADAPTER_DIR = "{adapter_path}"
MAX_NEW_TOKENS = 80
EOS_TOKEN_ID   = 2

def resolve_model_path(model_id):
    import os, pathlib
    if os.path.isdir(model_id):
        return model_id
    home = os.environ.get("HOME", "/root")
    slug = model_id.replace("/", "--")
    snaps = pathlib.Path(home) / ".cache" / "huggingface" / "hub" / f"models--{{slug}}" / "snapshots"
    if snaps.exists():
        for snap in snaps.iterdir():
            if snap.is_dir():
                return str(snap)
    return model_id

print("[bench] Lendo perguntas do stdin...", file=sys.stderr, flush=True)
questions = json.load(sys.stdin)
print(f"[bench] {{len(questions)}} perguntas recebidas.", file=sys.stderr, flush=True)

model_path = resolve_model_path(BASE_MODEL)
print(f"[bench] Carregando modelo base de: {{model_path}}", file=sys.stderr, flush=True)

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

print(f"[bench] Carregando adapter de: {{ADAPTER_DIR}}", file=sys.stderr, flush=True)
import os as _os
if _os.path.isdir(ADAPTER_DIR):
    model = PeftModel.from_pretrained(base, ADAPTER_DIR)
    print("[bench] Adapter carregado.", file=sys.stderr, flush=True)
else:
    print(f"[bench] AVISO: adapter nao encontrado em {{ADAPTER_DIR}}. Usando modelo base.", file=sys.stderr, flush=True)
    model = base

model.eval()

hits = 0
for i, q in enumerate(questions):
    question = q["question"]
    keywords = [kw.lower() for kw in q["expected_keywords"]]
    nl = chr(10)
    prompt   = "### Instruction:" + nl + question + nl + nl + "### Response:" + nl
    inputs   = tokenizer(prompt, return_tensors="pt").to(model.device)

    with torch.no_grad():
        out = model.generate(
            **inputs,
            max_new_tokens=MAX_NEW_TOKENS,
            do_sample=False,
            eos_token_id=[2, 1542],
            pad_token_id=tokenizer.pad_token_id,
        )

    gen_ids  = out[0][inputs["input_ids"].shape[1]:]
    response = tokenizer.decode(gen_ids, skip_special_tokens=True).lower()

    hit = all(kw in response for kw in keywords)
    if hit:
        hits += 1

    print(f"[bench] Q{{i+1}}/{{len(questions)}}: hit={{hit}} | {{question[:60]}}...", file=sys.stderr, flush=True)

score = hits / len(questions) if questions else 0.0
print(f"[bench] Score final: {{hits}}/{{len(questions)}} = {{score:.4f}}", file=sys.stderr, flush=True)
print(f"NEXUS_BENCHMARK_SCORE={{score}}", flush=True)
"####,
        base_model = base_model,
        adapter_path = adapter_path,
    )
}

