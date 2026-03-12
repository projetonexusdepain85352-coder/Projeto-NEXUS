// SPDX-License-Identifier: Apache-2.0

#![allow(dead_code)]

use std::{
    collections::HashMap,
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
};

use clap::{Parser, Subcommand};
use nexus_mtp::{approval, benchmark, dataset, db, trainer};
use sqlx::postgres::PgPoolOptions;
use tracing::info;
use uuid::Uuid;

use nexus_mtp::error::{MtpError, Result};

const DEFAULT_BASE_MODEL: &str = "mistralai/Mistral-7B-Instruct-v0.3";
const DEFAULT_MAX_SEQ_LEN: u32 = 2048;
const DATASETS_DIR: &str = "./datasets";
const DEFAULT_MODELS_DIR: &str = "/opt/nexus/models";

const DEFAULT_STAGE_A_MIN_SECURITY: i64 = 35;
const DEFAULT_STAGE_A_MIN_RUST: i64 = 80;
const DEFAULT_STAGE_A_MIN_INFRA: i64 = 100;
const DEFAULT_STAGE_A_MIN_MLOPS: i64 = 40;
const DEFAULT_STAGE_A_MIN_TOTAL: i64 = 300;

#[derive(Debug, Clone)]
struct StageAGateConfig {
    min_security: i64,
    min_rust: i64,
    min_infra: i64,
    min_mlops: i64,
    min_total: i64,
    max_pending_total: Option<i64>,
}

#[derive(Parser)]
#[command(
    name = "nexus_mtp",
    about = "NEXUS Model Training Pipeline",
    version = "0.1.0"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Extrai dataset JSONL de documentos aprovados
    Extract {
        #[arg(long)]
        domain: String,
        #[arg(long, default_value = "1000")]
        max_samples: i64,
    },
    /// Treina modelo com QLoRA via unsloth
    Train {
        #[arg(long)]
        domain: String,
        #[arg(long)]
        dataset: PathBuf,
        #[arg(long, default_value = DEFAULT_BASE_MODEL)]
        base_model: String,
        #[arg(long, default_value = "3")]
        epochs: u32,
        #[arg(long, default_value = "16")]
        lora_r: u32,
    },
    /// Executa benchmark de inferencia
    Benchmark {
        #[arg(long)]
        model_id: Uuid,
    },
    /// TUI para aprovacao humana
    Approve,
    /// Deploy de modelo aprovado
    Deploy {
        #[arg(long)]
        model_id: Uuid,
    },
    /// Status por dominio
    Status,
    /// Gate de parada da Etapa A com criterios explicitos por dominio
    StageAGate {
        #[arg(long, default_value_t = DEFAULT_STAGE_A_MIN_SECURITY)]
        min_security: i64,
        #[arg(long, default_value_t = DEFAULT_STAGE_A_MIN_RUST)]
        min_rust: i64,
        #[arg(long, default_value_t = DEFAULT_STAGE_A_MIN_INFRA)]
        min_infra: i64,
        #[arg(long, default_value_t = DEFAULT_STAGE_A_MIN_MLOPS)]
        min_mlops: i64,
        #[arg(long, default_value_t = DEFAULT_STAGE_A_MIN_TOTAL)]
        min_total: i64,
        /// Limite opcional de pendentes totais para considerar Etapa A concluida
        #[arg(long)]
        max_pending_total: Option<i64>,
    },
}

fn init_logging() {
    let filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive("nexus_mtp=info".parse().unwrap());
    let is_production = std::env::var("NEXUS_ENV")
        .unwrap_or_default()
        .trim()
        .eq_ignore_ascii_case("production");
    if is_production {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .json()
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }
}

#[tokio::main]
async fn main() {
    init_logging();
    if let Err(e) = run(Cli::parse()).await {
        eprintln!("\n[ERRO] {}", e);
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<()> {
    let db_url = build_db_url()?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .map_err(|e| MtpError::Other(format!("Banco: {e}")))?;

    match cli.command {
        Commands::Extract {
            domain,
            max_samples,
        } => cmd_extract(&pool, &domain, max_samples).await?,
        Commands::Train {
            domain,
            dataset,
            base_model,
            epochs,
            lora_r,
        } => cmd_train(&pool, &domain, &dataset, &base_model, epochs, lora_r).await?,
        Commands::Benchmark { model_id } => cmd_benchmark(&pool, model_id).await?,
        Commands::Approve => approval::run_approval_tui(&pool).await?,
        Commands::Deploy { model_id } => cmd_deploy(&pool, model_id).await?,
        Commands::Status => cmd_status(&pool).await?,
        Commands::StageAGate {
            min_security,
            min_rust,
            min_infra,
            min_mlops,
            min_total,
            max_pending_total,
        } => {
            let cfg = StageAGateConfig {
                min_security,
                min_rust,
                min_infra,
                min_mlops,
                min_total,
                max_pending_total,
            };
            cmd_stage_a_gate(&pool, &cfg).await?;
        }
    }
    Ok(())
}

async fn cmd_extract(pool: &sqlx::PgPool, domain: &str, max_samples: i64) -> Result<()> {
    println!(
        "=== NEXUS MTP -- Extract ===\nDominio: {}  Max: {}",
        domain, max_samples
    );
    let (path, doc_ids, total) = dataset::extract(pool, domain, max_samples, DATASETS_DIR).await?;
    println!(
        "\nDocumentos: {}\nExemplos:   {}\nDataset:    {}",
        doc_ids.len(),
        total,
        path.display()
    );
    Ok(())
}

async fn cmd_train(
    pool: &sqlx::PgPool,
    domain: &str,
    dataset: &Path,
    base_model: &str,
    epochs: u32,
    lora_r: u32,
) -> Result<()> {
    dataset::validate_domain(domain)?;
    if !dataset.exists() {
        return Err(MtpError::Other(format!(
            "Dataset nao encontrado: {}",
            dataset.display()
        )));
    }

    let base_model = resolve_base_model(base_model);

    let dataset_size = count_jsonl_lines(dataset)? as i32;
    let lora_alpha = lora_r * 2;

    println!("=== NEXUS MTP -- Train ===");
    println!(
        "Dominio: {}  Dataset: {} ({} exemplos)",
        domain,
        dataset.display(),
        dataset_size
    );
    println!(
        "Modelo:  {}  Epochs: {}  LoRA r={} a={}",
        &base_model, epochs, lora_r, lora_alpha
    );

    let config = serde_json::json!({
        "base_model": base_model,
        "epochs": epochs,
        "lora_r": lora_r,
        "lora_alpha": lora_alpha,
        "max_seq_len": DEFAULT_MAX_SEQ_LEN,
    });

    let cycle_id =
        db::create_training_cycle(pool, domain, &base_model, &config, dataset_size).await?;
    info!("Ciclo criado: {}", cycle_id);

    let models_dir = resolve_models_dir();

    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let model_name = format!("nexus-{}-{}", domain, ts);
    let output_dir = models_dir.join("training").join(&model_name);
    let adapter_dir = models_dir.join("adapters").join(&model_name);

    let job = trainer::TrainJob {
        base_model: base_model.clone(),
        dataset_path: dataset.to_path_buf(),
        domain: domain.to_string(),
        epochs,
        lora_r,
        lora_alpha,
        max_seq_len: DEFAULT_MAX_SEQ_LEN,
        learning_rate: 2e-4,
        output_dir: output_dir.clone(),
        adapter_path: adapter_dir.clone(),
        models_dir: models_dir.clone(),
    };

    println!("\nIniciando treinamento...");
    let result = match trainer::run_training(&job) {
        Ok(r) => r,
        Err(e) => {
            db::fail_training_cycle(pool, cycle_id).await?;
            return Err(e);
        }
    };

    println!("\n=== Concluido ===");
    println!(
        "Steps: {}  Loss: {:?}",
        result.training_steps, result.final_loss
    );
    println!("Checksum: {}", result.adapter_checksum);

    db::complete_training_cycle(pool, cycle_id, result.final_loss).await?;

    let adapter_rel = adapter_dir
        .strip_prefix(&models_dir)
        .unwrap_or(&adapter_dir)
        .to_string_lossy()
        .to_string();

    let doc_ids = load_doc_ids_from_sidecar(dataset)?;
    db::insert_lineage(pool, &doc_ids, cycle_id).await?;
    db::mark_used_in_training(pool, &doc_ids).await?;

    let model_id = db::create_model(
        pool,
        &model_name,
        domain,
        &base_model,
        dataset_size,
        result.training_steps,
        &adapter_rel,
        &result.adapter_checksum,
        cycle_id,
    )
    .await?;

    println!("Modelo: {}", model_id);
    println!("Proximo: nexus_mtp benchmark --model-id {}", model_id);
    Ok(())
}

async fn cmd_benchmark(pool: &sqlx::PgPool, model_id: Uuid) -> Result<()> {
    let model = db::get_model(pool, model_id).await?;
    println!("=== Benchmark: {} ({}) ===", model.name, model.domain);

    let models_dir = resolve_models_dir();
    let adapter_full = models_dir.join(model.adapter_path.as_deref().unwrap_or(""));

    let score = benchmark::run_benchmark(
        pool,
        model_id,
        &adapter_full.to_string_lossy(),
        &model.base_model,
        &model.domain,
    )
    .await?;

    db::update_benchmark_score(pool, model_id, score).await?;
    println!("Score: {:.4}", score);
    println!("Proximo: nexus_mtp approve");
    Ok(())
}

pub fn ensure_model_deployable(model: &db::ModelRow, min_score: f32) -> Result<()> {
    if model.status != "approved" {
        return Err(MtpError::NotApproved(model.status.clone()));
    }
    let score = model.benchmark_score.ok_or(MtpError::BenchmarkMissing)?;
    if !benchmark::benchmark_passed(score, min_score) {
        return Err(MtpError::BenchmarkBelowThreshold { score, min_score });
    }
    Ok(())
}
async fn cmd_deploy(pool: &sqlx::PgPool, model_id: Uuid) -> Result<()> {
    let model = db::get_model(pool, model_id).await?;
    let min_score = benchmark::benchmark_min_score();
    ensure_model_deployable(&model, min_score)?;
    let models_dir = resolve_models_dir();
    let adapter_full = models_dir.join(model.adapter_path.as_deref().unwrap_or(""));
    if !adapter_full.exists() {
        return Err(MtpError::AdapterNotFound(
            adapter_full.to_string_lossy().to_string(),
        ));
    }

    let archived = db::archive_deployed_models(pool, &model.domain).await?;
    if archived > 0 {
        info!("{} modelo(s) anterior(es) arquivado(s).", archived);
    }

    let domain_dir = models_dir.join(&model.domain);
    let symlink_path = domain_dir.join("current");
    fs::create_dir_all(&domain_dir)?;
    if symlink_path.exists() || symlink_path.is_symlink() {
        fs::remove_file(&symlink_path)?;
    }
    symlink(&adapter_full, &symlink_path)?;

    db::deploy_model(pool, model_id).await?;
    println!(
        "=== Deploy ===\nModelo: {}\nLink:   {} -> {}",
        model.name,
        symlink_path.display(),
        adapter_full.display()
    );
    Ok(())
}

async fn cmd_status(pool: &sqlx::PgPool) -> Result<()> {
    let stats = db::domain_stats(pool).await?;
    let active = db::active_model_per_domain(pool).await?;
    let active_map: HashMap<_, _> = active.into_iter().collect();

    println!("=== NEXUS MTP -- Status ===\n");
    println!(
        "{:<12} {:>14} {:>14} {:>10} Modelo Ativo",
        "Dominio", "Docs Aprovados", "Usados Treino", "Modelos"
    );
    println!("{}", "-".repeat(75));
    for s in &stats {
        let ativo = active_map
            .get(&s.domain)
            .and_then(|v| v.as_deref())
            .unwrap_or("--");
        println!(
            "{:<12} {:>14} {:>14} {:>10} {}",
            s.domain, s.approved_docs, s.used_in_training, s.total_models, ativo
        );
    }
    if stats.is_empty() {
        println!("Nenhum dado.");
    }
    println!("\nDica: rode `nexus_mtp stage-a-gate` para validar criterio de parada da Etapa A.");
    Ok(())
}

async fn cmd_stage_a_gate(pool: &sqlx::PgPool, cfg: &StageAGateConfig) -> Result<()> {
    let rows = db::domain_validation_stats(pool).await?;

    let mut by_domain: HashMap<String, db::DomainValidationStats> = HashMap::new();
    let mut pending_total_all = 0i64;
    let mut rejected_total_all = 0i64;

    for row in rows {
        pending_total_all += row.pending_docs;
        rejected_total_all += row.rejected_docs;
        by_domain.insert(row.domain.clone(), row);
    }

    let targets = [
        ("security", cfg.min_security),
        ("rust", cfg.min_rust),
        ("infra", cfg.min_infra),
        ("mlops", cfg.min_mlops),
    ];

    println!("=== NEXUS MTP -- Stage A Gate ===\n");
    println!("Criterios ativos:");
    println!(
        "  security >= {} | rust >= {} | infra >= {} | mlops >= {} | total >= {}",
        cfg.min_security, cfg.min_rust, cfg.min_infra, cfg.min_mlops, cfg.min_total
    );
    if let Some(max_pending) = cfg.max_pending_total {
        println!("  pending_total <= {}", max_pending);
    } else {
        println!("  pending_total: sem limite (use --max-pending-total para ativar)");
    }
    println!();

    println!(
        "{:<10} {:>10} {:>10} {:>10} {:>10} {:>14}",
        "Dominio", "Pending", "Approved", "Rejected", "Min", "Status"
    );
    println!("{}", "-".repeat(72));

    let mut approved_total_target = 0i64;
    let mut gate_ok = true;
    let mut reasons: Vec<String> = Vec::new();

    for (domain, min_required) in targets {
        let (pending, approved, rejected) = by_domain
            .get(domain)
            .map(|s| (s.pending_docs, s.approved_docs, s.rejected_docs))
            .unwrap_or((0, 0, 0));

        approved_total_target += approved;
        let deficit = (min_required - approved).max(0);

        let status = if deficit == 0 {
            "OK".to_string()
        } else {
            gate_ok = false;
            reasons.push(format!("{} precisa de +{} aprovados", domain, deficit));
            format!("FALTA +{}", deficit)
        };

        println!(
            "{:<10} {:>10} {:>10} {:>10} {:>10} {:>14}",
            domain, pending, approved, rejected, min_required, status
        );
    }

    println!("{}", "-".repeat(72));

    let total_deficit = (cfg.min_total - approved_total_target).max(0);
    if total_deficit > 0 {
        gate_ok = false;
        reasons.push(format!("total precisa de +{} aprovados", total_deficit));
    }

    println!(
        "Total aprovado (dominios alvo): {} / {}",
        approved_total_target, cfg.min_total
    );
    println!("Total pendente (todos dominios): {}", pending_total_all);
    println!("Total rejeitado (todos dominios): {}", rejected_total_all);

    if let Some(max_pending) = cfg.max_pending_total
        && pending_total_all > max_pending
    {
        gate_ok = false;
        reasons.push(format!(
            "pendentes totais acima do limite ({} > {})",
            pending_total_all, max_pending
        ));
    }

    if gate_ok {
        println!("\nRESULTADO: PASS");
        println!(
            "Etapa A pode ser encerrada e o fluxo pode seguir para o primeiro ciclo de treino."
        );
        return Ok(());
    }

    println!("\nRESULTADO: FAIL");
    println!("Etapa A ainda nao atingiu os criterios de parada configurados.");
    Err(MtpError::StageAGateNotSatisfied(reasons.join(" | ")))
}

fn resolve_base_model(cli_base_model: &str) -> String {
    if cli_base_model != DEFAULT_BASE_MODEL {
        return cli_base_model.to_string();
    }
    match std::env::var("NEXUS_BASE_MODEL") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => cli_base_model.to_string(),
    }
}

fn resolve_models_dir() -> PathBuf {
    std::env::var("NEXUS_MODELS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_MODELS_DIR))
}

fn build_db_url() -> Result<String> {
    let pw = std::env::var("KB_INGEST_PASSWORD")?;
    let host = std::env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = std::env::var("POSTGRES_PORT").unwrap_or_else(|_| "5433".to_string());
    let db = std::env::var("POSTGRES_DB").unwrap_or_else(|_| "knowledge_base".to_string());
    let user = std::env::var("POSTGRES_INGEST_USER")
        .or_else(|_| std::env::var("POSTGRES_USER"))
        .unwrap_or_else(|_| "kb_ingest".to_string());
    let encoded_pw = url_encode(&pw);
    Ok(format!(
        "postgres://{}:{}@{}:{}/{}",
        user, encoded_pw, host, port, db
    ))
}

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => {
                out.push('%');
                out.push_str(&format!("{:02X}", b));
            }
        }
    }
    out
}

fn count_jsonl_lines(path: &Path) -> Result<usize> {
    use std::io::{BufRead, BufReader};
    let f = fs::File::open(path)?;
    Ok(BufReader::new(f)
        .lines()
        .filter(|l| l.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false))
        .count())
}

/// Le o sidecar .ids gerado pelo subcomando extract.
/// Contem um UUID por linha.
fn load_doc_ids_from_sidecar(dataset: &Path) -> Result<Vec<Uuid>> {
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
