use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

use crate::error::{MtpError, Result};

#[derive(Debug, sqlx::FromRow)]
struct BenchmarkQuestion {
    question:          String,
    expected_keywords: Vec<String>,
}

pub async fn run_benchmark(
    pool:          &PgPool,
    model_id:      Uuid,
    _adapter_path: &str,
    base_model:    &str,
    domain:        &str,
) -> Result<f32> {
    let questions = fetch_benchmark_questions(pool, domain).await;
    let questions = match questions {
        Err(e) => {
            warn!("Benchmarks indisponiveis para '{}': {}. Score = 0.0", domain, e);
            return Ok(0.0);
        }
        Ok(v) if v.is_empty() => {
            warn!("Nenhuma pergunta de benchmark para '{}'. Score = 0.0", domain);
            return Ok(0.0);
        }
        Ok(v) => v,
    };
    info!("{} perguntas para '{}' (modelo {})...", questions.len(), domain, model_id);
    run_candle_benchmark(&questions, base_model)
}

async fn fetch_benchmark_questions(
    pool: &PgPool, domain: &str,
) -> std::result::Result<Vec<BenchmarkQuestion>, sqlx::Error> {
    sqlx::query_as::<_, BenchmarkQuestion>(
        "SELECT question, expected_keywords FROM benchmark_questions WHERE domain = $1 ORDER BY id",
    )
    .bind(domain)
    .fetch_all(pool)
    .await
}

fn run_candle_benchmark(questions: &[BenchmarkQuestion], base_model: &str) -> Result<f32> {
    use candle_core::{DType, Device};
    use candle_nn::VarBuilder;
    use candle_transformers::models::mistral::{Config as MistralConfig, Model as MistralModel};
    use tokenizers::Tokenizer;

    let device = Device::cuda_if_available(0).unwrap_or(Device::Cpu);
    info!("Dispositivo: {:?}", device);

    let tokenizer_path = locate_hf_file(base_model, "tokenizer.json")?;
    let config_path    = locate_hf_file(base_model, "config.json")?;
    let weight_files   = find_weight_files(base_model)?;

    let tokenizer = Tokenizer::from_file(&tokenizer_path)
        .map_err(|e| MtpError::Other(format!("tokenizer: {e}")))?;
    let config: MistralConfig = serde_json::from_str(&std::fs::read_to_string(&config_path)?)?;

    // SAFETY: mmap de arquivos read-only durante execucao
    let vb = unsafe {
        VarBuilder::from_mmaped_safetensors(&weight_files, DType::F16, &device)
            .map_err(|e| MtpError::Other(format!("pesos: {e}")))?
    };

    let mut model = MistralModel::new(&config, vb)
        .map_err(|e| MtpError::Other(format!("MistralModel::new: {e}")))?;

    let mut hits = 0usize;
    for q in questions {
        match generate_greedy(&mut model, &tokenizer, &q.question, &device) {
            Ok(resp) => {
                let r   = resp.to_lowercase();
                let hit = q.expected_keywords.iter().all(|kw| r.contains(&kw.to_lowercase()));
                if hit { hits += 1; }
                info!("Q: {:.60}... | hit={}", q.question, hit);
            }
            Err(e) => warn!("Inferencia falhou: {}", e),
        }
    }
    Ok(hits as f32 / questions.len() as f32)
}

/// PONTO DE AJUSTE: se model.forward mudar de assinatura no commit compilado,
/// corrija os argumentos aqui.
/// Assinatura esperada: forward(&mut self, x: &Tensor, seqlen_offset: usize)
fn generate_greedy(
    model:     &mut candle_transformers::models::mistral::Model,
    tokenizer: &tokenizers::Tokenizer,
    question:  &str,
    device:    &candle_core::Device,
) -> Result<String> {
    use candle_core::Tensor;

    const MAX_NEW_TOKENS: usize = 200;
    const EOS_TOKEN: u32 = 2;

    let prompt = format!("[INST] {question} [/INST]");
    let enc    = tokenizer.encode(prompt, true)
        .map_err(|e| MtpError::Other(format!("encode: {e}")))?;
    let prompt_ids: Vec<u32> = enc.get_ids().to_vec();
    let prompt_len = prompt_ids.len();

    let mut input = Tensor::from_vec(prompt_ids, (1, prompt_len), device)
        .map_err(|e| MtpError::Other(format!("tensor prompt: {e}")))?;
    let mut generated: Vec<u32> = Vec::with_capacity(MAX_NEW_TOKENS);
    let mut offset = 0usize;

    for _ in 0..MAX_NEW_TOKENS {
        let logits = model.forward(&input, offset)
            .map_err(|e| MtpError::Other(format!("forward: {e}")))?;

        let last = logits.squeeze(0)
            .and_then(|t| { let s = t.dim(0)?; t.get(s - 1) })
            .map_err(|e| MtpError::Other(format!("squeeze: {e}")))?;

        let next_id = last.argmax(candle_core::D::Minus1)
            .and_then(|t| t.to_scalar::<u32>())
            .map_err(|e| MtpError::Other(format!("argmax: {e}")))?;

        generated.push(next_id);
        if next_id == EOS_TOKEN { break; }

        offset += input.dim(1).map_err(|e| MtpError::Other(format!("dim: {e}")))?;
        input   = Tensor::from_vec(vec![next_id], (1, 1usize), device)
            .map_err(|e| MtpError::Other(format!("tensor next: {e}")))?;
    }

    tokenizer.decode(&generated, true)
        .map_err(|e| MtpError::Other(format!("decode: {e}")))
}

fn locate_hf_file(model_id: &str, filename: &str) -> Result<std::path::PathBuf> {
    let home      = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    let snapshots = std::path::PathBuf::from(format!(
        "{home}/.cache/huggingface/hub/models--{}/snapshots",
        model_id.replace('/', "--")
    ));
    if snapshots.exists() {
        for e in std::fs::read_dir(&snapshots)?.flatten() {
            let c = e.path().join(filename);
            if c.exists() { return Ok(c); }
        }
    }
    Err(MtpError::AdapterNotFound(format!(
        "'{filename}' nao encontrado no cache HF para '{model_id}'.\n\
         Baixe: python3 -c \"from transformers import AutoModelForCausalLM; \
         AutoModelForCausalLM.from_pretrained('{model_id}')\""
    )))
}

fn find_weight_files(model_id: &str) -> Result<Vec<std::path::PathBuf>> {
    let home      = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    let snapshots = std::path::PathBuf::from(format!(
        "{home}/.cache/huggingface/hub/models--{}/snapshots",
        model_id.replace('/', "--")
    ));

    let snapshot_dir = std::fs::read_dir(&snapshots)?
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.path())
        .next()
        .ok_or_else(|| MtpError::AdapterNotFound(format!("Nenhum snapshot para '{model_id}'")))?;

    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&snapshot_dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("safetensors"))
        .collect();

    if files.is_empty() {
        return Err(MtpError::AdapterNotFound(format!(
            "Nenhum .safetensors para '{model_id}' em {}", snapshot_dir.display()
        )));
    }

    files.sort();
    info!("{} arquivo(s) de pesos encontrados.", files.len());
    Ok(files)
}
