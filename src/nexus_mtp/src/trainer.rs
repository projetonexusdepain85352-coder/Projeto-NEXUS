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

pub fn run_training(job: &TrainJob) -> Result<TrainResult> {
    fs::create_dir_all(&job.output_dir)?;
    fs::create_dir_all(&job.adapter_path)?;

    let script_path = job.output_dir.join("_train_script.py");
    let script = build_python_script(job);
    fs::write(&script_path, &script)?;

    info!("Script gerado: {}", script_path.display());
    info!("Iniciando treinamento via python3...");

    let mut child = Command::new("python3")
        .arg(&script_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
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
    let base_model = sanitize_str(&job.base_model);
    let dataset_path = sanitize_str(&job.dataset_path.to_string_lossy());
    let adapter_path = sanitize_str(&job.adapter_path.to_string_lossy());
    let output_dir = sanitize_str(&job.output_dir.to_string_lossy());
    let max_seq_len = job.max_seq_len;
    let lora_r = job.lora_r;
    let lora_alpha = job.lora_alpha;
    let epochs = job.epochs;
    let lr = job.learning_rate;

    format!(
        "#!/usr/bin/env python3\n\
         # Gerado automaticamente pelo nexus_mtp\n\
         from unsloth import FastLanguageModel\n\
         from trl import SFTTrainer\n\
         from transformers import TrainingArguments\n\
         from datasets import load_dataset\n\
         \n\
         print('Carregando modelo: {base_model}', flush=True)\n\
         model, tokenizer = FastLanguageModel.from_pretrained(\n\
             model_name='{base_model}',\n\
             max_seq_length={max_seq_len},\n\
             dtype=None,\n\
             load_in_4bit=True,\n\
         )\n\
         \n\
         model = FastLanguageModel.get_peft_model(\n\
             model,\n\
             r={lora_r},\n\
             target_modules=['q_proj', 'k_proj', 'v_proj', 'o_proj'],\n\
             lora_alpha={lora_alpha},\n\
             lora_dropout=0.05,\n\
             bias='none',\n\
             use_gradient_checkpointing='unsloth',\n\
         )\n\
         \n\
         print('Carregando dataset: {dataset_path}', flush=True)\n\
         dataset = load_dataset('json', data_files='{dataset_path}', split='train')\n\
         \n\
         def format_alpaca(example):\n\
             return {{'text': (\n\
                 '### Instruction:\\n' + example['instruction'] + '\\n\\n'\n\
                 '### Input:\\n'       + example['input']       + '\\n\\n'\n\
                 '### Response:\\n'    + example['output']\n\
             )}}\n\
         \n\
         dataset = dataset.map(format_alpaca)\n\
         print(f'Dataset: {{len(dataset)}} exemplos.', flush=True)\n\
         \n\
         trainer = SFTTrainer(\n\
             model=model,\n\
             tokenizer=tokenizer,\n\
             train_dataset=dataset,\n\
             dataset_text_field='text',\n\
             max_seq_length={max_seq_len},\n\
             args=TrainingArguments(\n\
                 per_device_train_batch_size=1,\n\
                 gradient_accumulation_steps=8,\n\
                 warmup_steps=100,\n\
                 num_train_epochs={epochs},\n\
                 learning_rate={lr},\n\
                 fp16=True,\n\
                 output_dir='{output_dir}',\n\
                 save_steps=100,\n\
                 logging_steps=10,\n\
                 report_to='none',\n\
             ),\n\
         )\n\
         \n\
         print('Iniciando fine-tuning...', flush=True)\n\
         trainer.train()\n\
         \n\
         print('Salvando adapter...', flush=True)\n\
         model.save_pretrained('{adapter_path}')\n\
         tokenizer.save_pretrained('{adapter_path}')\n\
         print('NEXUS_TRAINING_COMPLETE', flush=True)\n"
    )
}

fn sanitize_str(s: &str) -> String {
    s.replace('\\', "/").replace('"', "_").replace('\'', "_")
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
