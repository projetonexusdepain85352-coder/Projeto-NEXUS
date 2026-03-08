mod approval;
mod clean;
mod db;
mod embedder;
mod error;
mod indexer;
mod qdrant_builder;
mod query;

// FIX 3: todos os imports no topo — sem `use` dentro de funções,
// sem duplicatas entre run_status() e count_distinct_docs().
use std::collections::{BTreeSet, HashMap, HashSet};

use clap::{Parser, Subcommand};
use qdrant_client::Qdrant;
use qdrant_client::qdrant::{PayloadIncludeSelector, PointId, ScrollPointsBuilder, value::Kind};
use tokio::signal;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use error::{NexusError, Result};

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(
    name = "nexus_rag",
    version = "0.1.0",
    about = "NEXUS private AI — RAG component"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Index all approved documents from PostgreSQL into Qdrant.
    Index,
    /// Query the RAG system for relevant chunks.
    Query {
        text: String,
        #[arg(long)]
        domain: Option<String>,
        #[arg(long, default_value_t = 5)]
        top: usize,
    },
    /// Show index statistics.
    Status,
}

// ── Logging ───────────────────────────────────────────────────────────────────

fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .json()
                .with_current_span(false)
                .with_span_list(false),
        )
        .init();
}

// ── Graceful shutdown ─────────────────────────────────────────────────────────

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let term = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { tracing::info!(signal = "SIGINT",  "Shutdown requested") },
        _ = term   => { tracing::info!(signal = "SIGTERM", "Shutdown requested") },
    }
}

// ── Entry-point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    init_logging();
    let cli = Cli::parse();

    let result = tokio::select! {
        r = run(cli) => r,
        _ = shutdown_signal() => {
            tracing::info!("Graceful shutdown — exiting cleanly");
            Ok(())
        }
    };

    if let Err(e) = result {
        tracing::error!(error = %e, "Fatal error");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Index => {
            let pool = db::connect().await?;
            let count = db::count_approved_documents(&pool).await?;

            // FIX 6: gate de aprovação humana só ativo quando NEXUS_ENV=production.
            // Em desenvolvimento/staging/CI, o gate é silenciosamente ignorado.
            tokio::task::spawn_blocking(move || {
                approval::require_human_approval(
                    "RAG Index",
                    &format!(
                        "{} documento(s) aprovados serão adicionados à base de conhecimento",
                        count
                    ),
                )
            })
            .await
            .map_err(|e| NexusError::Config(format!("spawn_blocking failed: {e}")))??;

            indexer::run_index(&pool).await?;
        }

        Commands::Query { text, domain, top } => {
            query::run_query(&text, domain.as_deref(), top).await?;
        }

        Commands::Status => {
            run_status().await?;
        }
    }
    Ok(())
}

// ── Status ────────────────────────────────────────────────────────────────────

async fn run_status() -> Result<()> {
    // FIX 3: sem `use` aqui — todos os tipos já importados no topo do arquivo.
    let client = qdrant_builder::build_qdrant_client()?;
    let pool = db::connect().await?;

    let all_collections = client.list_collections().await.map_err(error::qdrant_err)?;

    let mut qdrant_points: HashMap<String, u64> = HashMap::new();
    for desc in all_collections.collections {
        if !desc.name.starts_with("nexus_") {
            continue;
        }
        let info = client
            .collection_info(&desc.name)
            .await
            .map_err(error::qdrant_err)?;
        let pts = info
            .result
            .as_ref()
            .and_then(|r| r.points_count)
            .unwrap_or(0);
        qdrant_points.insert(desc.name, pts);
    }

    let pg_counts = db::fetch_approved_by_domain(&pool).await?;

    let mut indexed_counts: HashMap<String, u64> = HashMap::new();
    for coll_name in qdrant_points.keys() {
        let domain_key = coll_name
            .strip_prefix("nexus_")
            .unwrap_or(coll_name)
            .to_string();
        let count = count_distinct_docs(&client, coll_name).await?;
        indexed_counts.insert(domain_key, count);
    }

    println!();
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║                     NEXUS RAG — System Status                        ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");
    println!();
    println!("  ● Qdrant Collections");
    println!("    {:40}  {:>12}", "Collection", "Points");
    println!("    {}", "─".repeat(55));

    if qdrant_points.is_empty() {
        println!("    (no nexus_* collections found)");
    } else {
        let mut sorted: Vec<_> = qdrant_points.iter().collect();
        sorted.sort_by_key(|(k, _)| k.as_str());
        for (name, pts) in &sorted {
            println!("    {:40}  {:>12}", name, pts);
        }
    }

    println!();
    println!("  ● PostgreSQL — Approved Documents by Domain");
    println!(
        "    {:30}  {:>10}  {:>10}  {:>12}",
        "Domain", "Approved", "Indexed", "Delta"
    );
    println!("    {}", "─".repeat(68));

    let mut all_domains: BTreeSet<String> = BTreeSet::new();
    all_domains.extend(pg_counts.keys().cloned());
    all_domains.extend(
        qdrant_points
            .keys()
            .map(|k| k.strip_prefix("nexus_").unwrap_or(k).to_string()),
    );

    let (mut ga, mut gi, mut gd) = (0i64, 0u64, 0i64);

    for domain in &all_domains {
        let approved = pg_counts.get(domain).copied().unwrap_or(0);
        let indexed = indexed_counts.get(domain).copied().unwrap_or(0);
        let delta = approved - indexed as i64;
        ga += approved;
        gi += indexed;
        gd += delta;

        let delta_disp = match delta.cmp(&0) {
            std::cmp::Ordering::Greater => format!("⚠  {:>+8}", delta),
            std::cmp::Ordering::Less => format!("?  {:>+8}", delta),
            std::cmp::Ordering::Equal => format!("✓  {:>8}", 0),
        };
        println!(
            "    {:30}  {:>10}  {:>10}  {}",
            domain, approved, indexed, delta_disp
        );
    }

    println!("    {}", "─".repeat(68));
    let grand_disp = if gd > 0 {
        format!("⚠  {:>+8}", gd)
    } else {
        format!("✓  {:>8}", 0)
    };
    println!("    {:30}  {:>10}  {:>10}  {}", "TOTAL", ga, gi, grand_disp);
    println!();
    if gd > 0 {
        println!(
            "  ⚠  {} document(s) not yet indexed. Run `nexus_rag index`.",
            gd
        );
    } else {
        println!("  ✓  All approved documents are indexed.");
    }
    println!();
    Ok(())
}

// ── Count distinct document_ids via paginated scroll ─────────────────────────

/// Counts distinct `document_id` values in a collection via paginated scroll.
///
/// Capped at MAX_SCROLL_PAGES pages (256 points each = 256 000 points max)
/// to prevent unbounded memory use. Beyond the cap the count is approximate
/// and a warning is logged.
async fn count_distinct_docs(client: &Qdrant, collection: &str) -> Result<u64> {
    // FIX 3: sem `use` duplicados aqui — tipos já importados no topo.
    const MAX_SCROLL_PAGES: usize = 1_000;

    let mut unique: HashSet<String> = HashSet::new();
    let mut offset: Option<PointId> = None;
    let mut pages = 0usize;

    loop {
        if pages >= MAX_SCROLL_PAGES {
            tracing::warn!(
                collection = collection,
                pages = pages,
                "count_distinct_docs: scroll page cap reached, count is approximate"
            );
            break;
        }

        let mut builder = ScrollPointsBuilder::new(collection)
            .limit(256u32)
            .with_payload(PayloadIncludeSelector {
                fields: vec!["document_id".to_string()],
            });

        if let Some(ref off) = offset {
            builder = builder.offset(off.clone());
        }

        let resp = client.scroll(builder).await.map_err(error::qdrant_err)?;
        pages += 1;

        for point in &resp.result {
            if let Some(v) = point.payload.get("document_id") {
                if let Some(Kind::StringValue(s)) = v.kind.as_ref() {
                    unique.insert(s.clone());
                }
            }
        }

        match resp.next_page_offset {
            None => break,
            Some(next) => match next.point_id_options {
                None => break,
                Some(_) => offset = Some(next),
            },
        }
    }

    Ok(unique.len() as u64)
}
