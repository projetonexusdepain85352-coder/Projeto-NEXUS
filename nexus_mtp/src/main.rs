mod approval;
mod benchmark;
mod dataset;
mod db;
mod error;
mod trainer;

use std::{fs, os::unix::fs::symlink, path::PathBuf};

use clap::{Parser, Subcommand};
use sqlx::postgres::PgPoolOptions;
use tracing::info;
use uuid::Uuid;

use crate::error::{MtpError, Result};

const DEFAULT_BASE_MODEL: &str = "mistralai/Mistral-7B-Instruct-v0.3";
const DEFAULT_MAX_SEQ_LEN: u32 = 2048;
const DATASETS_DIR: &str = "./datasets";
const MODELS_DIR:   &str = "/opt/nexus/models";

#[derive(Parser)]
#[command(name = "nexus_mtp", about = "NEXUS Model Training Pipeline", version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Extrai dataset JSONL de documentos aprovados
    Extract {
        #[arg(long)] domain: String,
        #[arg(long, default_value = "1000")] max_samples: i64,
    },
    /// Treina modelo com QLoRA via unsloth
    Train {
        #[arg(long)] domain: String,
        #[arg(long)] dataset: PathBuf,
        #[arg(long, default_value = DEFAULT_BASE_MODEL)] base_model: String,
        #[arg(long, default_value = "3")] epochs: u32,
        #[arg(long, default_value = "16")] lora_r: u32,
    },
    /// Executa benchmark de inferencia
    Benchmark {
        #[arg(long)] model_id: Uuid,
    },
    /// TUI para aprovacao humana
    Approve,
    /// Deploy de modelo aprovado
    Deploy {
        #[arg(long)] model_id: Uuid,
    },
    /// Status por dominio
    Status,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("nexus_mtp=info".parse().unwrap()),
        )
        .init();
    if let Err(e) = run(Cli::parse()).await {
        eprintln!("\n[ERRO] {}", e);
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<()> {
    let db_url = build_db_url()?;
    let pool   = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .map_err(|e| MtpError::Other(format!("Banco: {e}")))?;

    match cli.command {
        Commands::Extract { domain, max_samples } =>
            cmd_extract(&pool, &domain, max_samples).await?,
        Commands::Train { domain, dataset, base_model, epochs, lora_r } =>
            cmd_train(&pool, &domain, &dataset, &base_model, epochs, lora_r).await?,
        Commands::Benchmark { model_id } =>
            cmd_benchmark(&pool, model_id).await?,
        Commands::Approve =>
            approval::run_approval_tui(&pool).await?,
        Commands::Deploy { model_id } =>
            cmd_deploy(&pool, model_id).await?,
        Commands::Status =>
            cmd_status(&pool).await?,
    }
    Ok(())
}

async fn cmd_extract(pool: &sqlx::PgPool, domain: &str, max_samples: i64) -> Result<()> {
    println!("=== NEXUS MTP -- Extract ===\nDominio: {}  Max: {}", domain, max_samples);
    let (path, doc_ids, total) =
        dataset::extract(pool, domain, max_samples, DATASETS_DIR).await?;
    println!("\nDocumentos: {}\nExemplos:   {}\nDataset:    {}", doc_ids.len(), total, path.display());
    Ok(())
}

async fn cmd_train(
    pool: &sqlx::PgPool, domain: &str, dataset: &PathBuf,
    base_model: &str, epochs: u32, lora_r: u32,
) -> Result<()> {
    dataset::validate_domain(domain)?;
    if !dataset.exists() {
        return Err(MtpError::Other(format!("Dataset nao encontrado: {}", dataset.display())));
    }

    let dataset_size = count_jsonl_lines(dataset)? as i32;
    let lora_alpha   = lora_r * 2;

    println!("=== NEXUS MTP -- Train ===");
    println!("Dominio: {}  Dataset: {} ({} exemplos)", domain, dataset.display(), dataset_size);
    println!("Modelo:  {}  Epochs: {}  LoRA r={} a={}", base_model, epochs, lora_r, lora_alpha);

    let config = serde_json::json!({
        "base_model": base_model,
        "epochs": epochs,
        "lora_r": lora_r,
        "lora_alpha": lora_alpha,
        "max_seq_len": DEFAULT_MAX_SEQ_LEN,
    });

    let cycle_id = db::create_training_cycle(pool, domain, base_model, &config, dataset_size).await?;
    info!("Ciclo criado: {}", cycle_id);

    let ts          = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let model_name  = format!("nexus-{}-{}", domain, ts);
    let output_dir  = PathBuf::from(MODELS_DIR).join("training").join(&model_name);
    let adapter_dir = PathBuf::from(MODELS_DIR).join("adapters").join(&model_name);

    let job = trainer::TrainJob {
        base_model:    base_model.to_string(),
        dataset_path:  dataset.clone(),
        domain:        domain.to_string(),
        epochs,
        lora_r,
        lora_alpha,
        max_seq_len:   DEFAULT_MAX_SEQ_LEN,
        learning_rate: 2e-4,
        output_dir:    output_dir.clone(),
        adapter_path:  adapter_dir.clone(),
        models_dir:    PathBuf::from(MODELS_DIR),
    };

    println!("\nIniciando treinamento...");
    let result = match trainer::run_training(&job) {
        Ok(r)  => r,
        Err(e) => { db::fail_training_cycle(pool, cycle_id).await?; return Err(e); }
    };

    println!("\n=== Concluido ===");
    println!("Steps: {}  Loss: {:?}", result.training_steps, result.final_loss);
    println!("Checksum: {}", result.adapter_checksum);

    db::complete_training_cycle(pool, cycle_id, result.final_loss).await?;

    let adapter_rel = adapter_dir
        .strip_prefix(MODELS_DIR)
        .unwrap_or(&adapter_dir)
        .to_string_lossy()
        .to_string();

    let doc_ids = load_doc_ids_from_sidecar(dataset)?;
    db::insert_lineage(pool, &doc_ids, cycle_id).await?;
    db::mark_used_in_training(pool, &doc_ids).await?;

    let model_id = db::create_model(
        pool, &model_name, domain, base_model,
        dataset_size, result.training_steps,
        &adapter_rel, &result.adapter_checksum, cycle_id,
    ).await?;

    println!("Modelo: {}", model_id);
    println!("Proximo: nexus_mtp benchmark --model-id {}", model_id);
    Ok(())
}

async fn cmd_benchmark(pool: &sqlx::PgPool, model_id: Uuid) -> Result<()> {
    let model = db::get_model(pool, model_id).await?;
    println!("=== Benchmark: {} ({}) ===", model.name, model.domain);

    let adapter_full = PathBuf::from(MODELS_DIR)
        .join(model.adapter_path.as_deref().unwrap_or(""));

    let score = benchmark::run_benchmark(
        pool, model_id,
        &adapter_full.to_string_lossy(),
        &model.base_model,
        &model.domain,
    ).await?;

    db::update_benchmark_score(pool, model_id, score).await?;
    println!("Score: {:.4}", score);
    println!("Proximo: nexus_mtp approve");
    Ok(())
}

async fn cmd_deploy(pool: &sqlx::PgPool, model_id: Uuid) -> Result<()> {
    let model = db::get_model(pool, model_id).await?;
    if model.status != "approved" {
        return Err(MtpError::NotApproved(model.status));
    }

    let adapter_full = PathBuf::from(MODELS_DIR)
        .join(model.adapter_path.as_deref().unwrap_or(""));
    if !adapter_full.exists() {
        return Err(MtpError::AdapterNotFound(adapter_full.to_string_lossy().to_string()));
    }

    let archived = db::archive_deployed_models(pool, &model.domain).await?;
    if archived > 0 { info!("{} modelo(s) anterior(es) arquivado(s).", archived); }

    let domain_dir   = PathBuf::from(MODELS_DIR).join(&model.domain);
    let symlink_path = domain_dir.join("current");
    fs::create_dir_all(&domain_dir)?;
    if symlink_path.exists() || symlink_path.is_symlink() {
        fs::remove_file(&symlink_path)?;
    }
    symlink(&adapter_full, &symlink_path)?;

    db::deploy_model(pool, model_id).await?;
    println!("=== Deploy ===\nModelo: {}\nLink:   {} -> {}", 
        model.name, symlink_path.display(), adapter_full.display());
    Ok(())
}

async fn cmd_status(pool: &sqlx::PgPool) -> Result<()> {
    let stats  = db::domain_stats(pool).await?;
    let active = db::active_model_per_domain(pool).await?;
    let active_map: std::collections::HashMap<_, _> = active.into_iter().collect();

    println!("=== NEXUS MTP -- Status ===\n");
    println!("{:<12} {:>14} {:>14} {:>10} {}", 
        "Dominio", "Docs Aprovados", "Usados Treino", "Modelos", "Modelo Ativo");
    println!("{}", "-".repeat(75));
    for s in &stats {
        let ativo = active_map.get(&s.domain).and_then(|v| v.as_deref()).unwrap_or("--");
        println!("{:<12} {:>14} {:>14} {:>10} {}", 
            s.domain, s.approved_docs, s.used_in_training, s.total_models, ativo);
    }
    if stats.is_empty() { println!("Nenhum dado."); }
    Ok(())
}

fn build_db_url() -> Result<String> {
    let pw = std::env::var("KB_INGEST_PASSWORD")?;
    Ok(format!("postgres://kb_ingest:{}@localhost:5432/knowledge_base", pw))
}

fn count_jsonl_lines(path: &PathBuf) -> Result<usize> {
    use std::io::{BufRead, BufReader};
    let f = fs::File::open(path)?;
    Ok(BufReader::new(f)
        .lines()
        .filter(|l| l.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false))
        .count())
}

/// Le o sidecar .ids gerado pelo subcomando extract.
/// Contem um UUID por linha.
fn load_doc_ids_from_sidecar(dataset: &PathBuf) -> Result<Vec<Uuid>> {
    let ids_path = dataset.with_extension("ids");
    if !ids_path.exists() {
        tracing::warn!(
            "Sidecar nao encontrado em {}. Lineage ficara vazia para este ciclo.",
            ids_path.display()
        );
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&ids_path)?;
    let ids: Vec<Uuid> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.trim().parse::<Uuid>())
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| MtpError::Other(format!("UUID invalido no sidecar: {e}")))?;
    tracing::info!("{} IDs carregados do sidecar.", ids.len());
    Ok(ids)
}
