use std::{
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use sha3::{Digest, Sha3_256};
use tracing::{error, info, warn};

use crate::error::{MtpError, Result};

#[allow(dead_code)]
pub struct TrainJob {
    pub base_model: String,
    pub dataset_path: PathBuf,
    pub domain: String,
    pub epochs: u32,
    pub lora_r: u32,
    pub lora_alpha: u32,
    pub max_seq_len: u32,
    pub learning_rate: f64,
    pub output_dir: PathBuf,
    pub adapter_path: PathBuf,
    pub models_dir: PathBuf,
}

pub struct TrainResult {
    pub training_steps: i32,
    pub final_loss: Option<f32>,
    pub adapter_path: PathBuf,
    pub adapter_checksum: String,
}

pub fn training_env_overrides() -> Vec<(&'static str, &'static str)> {
    vec![
        ("TORCHINDUCTOR_DISABLE", "1"),
        ("TORCH_COMPILE_DISABLE", "1"),
        ("UNSLOTH_ENABLE_CCE", "0"),
        ("UNSLOTH_COMPILE_DISABLE", "1"),
        ("UNSLOTH_CE_LOSS_N_CHUNKS", "4096"),
    ]
}
pub fn run_training(job: &TrainJob) -> Result<TrainResult> {
    fs::create_dir_all(&job.output_dir)?;
    fs::create_dir_all(&job.adapter_path)?;

    let script_path = job.output_dir.join("_train_script.py");
    let script = build_python_script(job);
    fs::write(&script_path, &script)?;

    info!("Script gerado: {}", script_path.display());
    info!("Iniciando treinamento via python3...");

    let mut cmd = Command::new("python3");
    cmd.arg(&script_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    for (key, value) in training_env_overrides() {
        cmd.env(key, value);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| MtpError::Other(format!("Falha ao iniciar python3: {e}")))?;

    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");

    let stderr_handle = std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        let mut lines = Vec::new();
        for line in reader.lines().map_while(|l| l.ok()) {
            eprintln!("[python stderr] {}", line);
            lines.push(line);
        }
        lines
    });

    let mut training_complete = false;
    let mut total_steps = 0i32;
    let mut last_loss: Option<f32> = None;

    let reader = BufReader::new(stdout);
    for line in reader.lines().map_while(|l| l.ok()) {
        println!("[train] {}", line);
        if line.contains("NEXUS_TRAINING_COMPLETE") {
            training_complete = true;
            continue;
        }
        if let Some(parsed) = parse_log_line(&line) {
            if let Some(step) = parsed.step {
                total_steps = step;
            }
            if let Some(loss) = parsed.loss {
                last_loss = Some(loss);
                info!("step={} loss={:.4}", total_steps, loss);
            }
        }
    }

    let stderr_lines = stderr_handle.join().unwrap_or_default();
    let status = child.wait().map_err(|e| MtpError::Other(e.to_string()))?;

    if !status.success() || !training_complete {
        let code = status.code().unwrap_or(-1);
        let stderr = stderr_lines.join("\n");
        error!("Treinamento falhou (codigo {})", code);
        return Err(MtpError::TrainingFailed { code, stderr });
    }

    info!("Calculando checksum do adapter...");
    let checksum = compute_adapter_checksum(&job.adapter_path)?;
    let _ = fs::remove_file(&script_path);

    Ok(TrainResult {
        training_steps: total_steps,
        final_loss: last_loss,
        adapter_path: job.adapter_path.clone(),
        adapter_checksum: checksum,
    })
}

fn build_python_script(job: &TrainJob) -> String {
    let model_path = sanitize_str(&resolve_model_path(&job.base_model));
    let dataset_path = sanitize_str(&job.dataset_path.to_string_lossy());
    let adapter_path = sanitize_str(&job.adapter_path.to_string_lossy());
    let output_dir = sanitize_str(&job.output_dir.to_string_lossy());
    let max_seq_len = job.max_seq_len;
    let lora_r = job.lora_r;
    let lora_alpha = job.lora_alpha;
    let epochs = job.epochs;
    let lr = job.learning_rate;

    format!(
        r####"#!/usr/bin/env python3
# Gerado automaticamente pelo nexus_mtp
import os
os.environ["TORCHINDUCTOR_DISABLE"] = "1"
os.environ["TORCH_COMPILE_DISABLE"] = "1"

import torch
from peft import LoraConfig, get_peft_model
from transformers import AutoModelForCausalLM, AutoTokenizer, BitsAndBytesConfig
from trl import SFTTrainer, SFTConfig
from datasets import load_dataset

MODEL_PATH = "{model_path}"
print('Carregando modelo: {model_path}', flush=True)

bnb_config = BitsAndBytesConfig(
    load_in_4bit=True,
    bnb_4bit_compute_dtype=torch.bfloat16,
    bnb_4bit_use_double_quant=True,
    bnb_4bit_quant_type="nf4",
)

model = AutoModelForCausalLM.from_pretrained(
    MODEL_PATH,
    quantization_config=bnb_config,
    device_map="auto",
)
tokenizer = AutoTokenizer.from_pretrained(MODEL_PATH)
tokenizer.pad_token = tokenizer.eos_token

model = get_peft_model(model, LoraConfig(
    r={lora_r},
    lora_alpha={lora_alpha},
    target_modules=["q_proj", "k_proj", "v_proj", "o_proj"],
    lora_dropout=0.05,
    bias="none",
    task_type="CAUSAL_LM",
))
model.print_trainable_parameters()

print('Carregando dataset: {dataset_path}', flush=True)
dataset = load_dataset("json", data_files="{dataset_path}", split="train")
dataset = dataset.map(lambda x: {{
    "text": "### Instruction:\n" + x["instruction"] +
            "\n\n### Input:\n" + x["input"] +
            "\n\n### Response:\n" + x["output"]
}})
print(f"Dataset: {{len(dataset)}} exemplos.", flush=True)

trainer = SFTTrainer(
    model=model,
    processing_class=tokenizer,
    train_dataset=dataset,
    args=SFTConfig(
        dataset_text_field="text",
        max_length={max_seq_len},
        per_device_train_batch_size=1,
        gradient_accumulation_steps=8,
        warmup_steps=100,
        num_train_epochs={epochs},
        learning_rate={lr},
        fp16=False,
        bf16=True,
        output_dir="{output_dir}",
        save_steps=100,
        logging_steps=10,
        report_to="none",
    ),
)

print("Iniciando fine-tuning...", flush=True)
trainer.train()

print("Salvando adapter...", flush=True)
model.save_pretrained("{adapter_path}")
tokenizer.save_pretrained("{adapter_path}")
print("NEXUS_TRAINING_COMPLETE", flush=True)
"####
    )
}

fn sanitize_str(s: &str) -> String {
    s.replace('\\', "/").replace(['"', '\''], "_")
}

fn resolve_model_path(base_model: &str) -> String {
    let path = Path::new(base_model);
    let looks_like_path = path.is_absolute()
        || base_model.starts_with("./")
        || base_model.starts_with("../")
        || base_model.starts_with('/')
        || base_model.contains('\\')
        || base_model.contains(':');
    if looks_like_path {
        base_model.to_string()
    } else {
        "/home/dulan/.cache/huggingface/hub/models--unsloth--mistral-7b-instruct-v0.3-bnb-4bit/snapshots/d5f623888f1415cf89b5c208d09cb620694618ee".to_string()
    }
}

pub fn compute_adapter_checksum(adapter_dir: &Path) -> Result<String> {
    let mut hasher = Sha3_256::new();
    let mut entries: Vec<_> = fs::read_dir(adapter_dir)?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());

    let mut found_any = false;
    for entry in &entries {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext == "safetensors" || ext == "bin" {
            hasher.update(&fs::read(&path)?);
            found_any = true;
        }
    }

    if !found_any {
        warn!(
            "Nenhum .safetensors em {}; hash sobre nomes.",
            adapter_dir.display()
        );
        for entry in &entries {
            hasher.update(entry.file_name().to_string_lossy().as_bytes());
        }
    }

    Ok(hex::encode(hasher.finalize()))
}

struct LogEntry {
    step: Option<i32>,
    loss: Option<f32>,
}

fn parse_log_line(line: &str) -> Option<LogEntry> {
    let trimmed = line.trim();
    if !trimmed.starts_with('{') {
        return None;
    }
    let v: serde_json::Value = serde_json::from_str(trimmed).ok()?;
    Some(LogEntry {
        step: v.get("step").and_then(|s| s.as_i64()).map(|s| s as i32),
        loss: v.get("loss").and_then(|l| l.as_f64()).map(|l| l as f32),
    })
}





