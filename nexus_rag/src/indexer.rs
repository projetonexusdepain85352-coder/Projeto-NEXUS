//! Indexing pipeline: PostgreSQL approved docs → Qdrant vector store.

use std::collections::HashMap;

use qdrant_client::Qdrant;
use qdrant_client::qdrant::{
    Condition, CountPointsBuilder, CreateCollectionBuilder, CreateFieldIndexCollectionBuilder,
    Distance, FieldCondition, FieldType, Filter, Match, PointStruct, UpsertPointsBuilder, Value,
    VectorParamsBuilder, condition::ConditionOneOf, r#match::MatchValue, value::Kind,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::clean::clean_document_text;
use crate::db::{DocumentRecord, fetch_approved_documents};
use crate::embedder::{EMBEDDING_DIM, Embedder};
use crate::error::{NexusError, Result, qdrant_err};

const WORDS_PER_CHUNK: usize = 400;
const OVERLAP_WORDS: usize = 50;

// ── Collection naming ────────────────────────────────────────────────────────

pub fn collection_name(domain: &str) -> String {
    let s: String = domain
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    format!("nexus_{}", s)
}

// ── Chunking with overlap ────────────────────────────────────────────────────

fn chunk_text(text: &str, words_per_chunk: usize, overlap: usize) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return Vec::new();
    }
    let step = words_per_chunk.saturating_sub(overlap).max(1);
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < words.len() {
        let end = (start + words_per_chunk).min(words.len());
        chunks.push(words[start..end].join(" "));
        if end == words.len() {
            break;
        }
        start += step;
    }
    chunks
}

// ── Qdrant helpers ───────────────────────────────────────────────────────────

async fn ensure_collection(client: &Qdrant, name: &str) -> Result<()> {
    if client.collection_exists(name).await.map_err(qdrant_err)? {
        tracing::debug!(collection = name, "Collection already exists");
        return Ok(());
    }
    tracing::info!(collection = name, "Creating Qdrant collection");
    client
        .create_collection(
            CreateCollectionBuilder::new(name)
                .vectors_config(VectorParamsBuilder::new(EMBEDDING_DIM, Distance::Cosine)),
        )
        .await
        .map_err(qdrant_err)?;
    client
        .create_field_index(CreateFieldIndexCollectionBuilder::new(
            name,
            "document_id",
            FieldType::Keyword,
        ))
        .await
        .map_err(qdrant_err)?;
    tracing::info!(collection = name, dim = EMBEDDING_DIM, "Collection created");
    Ok(())
}

async fn is_document_indexed(client: &Qdrant, collection: &str, doc_id: &Uuid) -> Result<bool> {
    if !client
        .collection_exists(collection)
        .await
        .map_err(qdrant_err)?
    {
        return Ok(false);
    }
    let filter = Filter {
        must: vec![Condition {
            condition_one_of: Some(ConditionOneOf::Field(FieldCondition {
                key: "document_id".to_string(),
                r#match: Some(Match {
                    match_value: Some(MatchValue::Keyword(doc_id.to_string())),
                }),
                ..Default::default()
            })),
        }],
        ..Default::default()
    };
    let result = client
        .count(
            CountPointsBuilder::new(collection)
                .filter(filter)
                .exact(true),
        )
        .await
        .map_err(qdrant_err)?;
    Ok(result.result.map_or(0, |r| r.count) > 0)
}

// ── Payload helpers ──────────────────────────────────────────────────────────

fn str_val(s: &str) -> Value {
    Value {
        kind: Some(Kind::StringValue(s.to_string())),
    }
}
fn int_val(n: i64) -> Value {
    Value {
        kind: Some(Kind::IntegerValue(n)),
    }
}

fn build_payload(
    document_id: &Uuid,
    source: &str,
    domain: &str,
    doc_type: &str,
    chunk_index: usize,
    chunk_total: usize,
    chunk_text: &str,
) -> HashMap<String, Value> {
    let mut m = HashMap::new();
    m.insert("document_id".to_string(), str_val(&document_id.to_string()));
    m.insert("source".to_string(), str_val(source));
    m.insert("domain".to_string(), str_val(domain));
    m.insert("doc_type".to_string(), str_val(doc_type));
    m.insert("chunk_index".to_string(), int_val(chunk_index as i64));
    m.insert("chunk_total".to_string(), int_val(chunk_total as i64));
    m.insert("chunk_text".to_string(), str_val(chunk_text));
    m
}

// ── Per-document indexing ────────────────────────────────────────────────────

enum IndexOutcome {
    Indexed { chunks: usize },
    Skipped,
}

async fn index_document(
    client: &Qdrant,
    embedder: &Embedder,
    doc: &DocumentRecord,
) -> Result<IndexOutcome> {
    let cleaned_content = clean_document_text(&doc.content);
    if cleaned_content.trim().is_empty() {
        return Err(NexusError::Config(format!(
            "Document {} has empty content",
            doc.id
        )));
    }
    let coll = collection_name(&doc.domain);
    if is_document_indexed(client, &coll, &doc.id).await? {
        return Ok(IndexOutcome::Skipped);
    }
    ensure_collection(client, &coll).await?;

    let chunks = chunk_text(&cleaned_content, WORDS_PER_CHUNK, OVERLAP_WORDS);
    let chunk_total = chunks.len();
    if chunk_total == 0 {
        return Err(NexusError::Config(format!(
            "Document {} produced 0 chunks",
            doc.id
        )));
    }

    let embeddings = embedder.embed_batch(&chunks)?;
    if embeddings.len() != chunk_total {
        return Err(NexusError::Embedding(format!(
            "Expected {} embeddings for doc {}, got {}",
            chunk_total,
            doc.id,
            embeddings.len()
        )));
    }

    let points: Vec<PointStruct> = chunks
        .iter()
        .enumerate()
        .zip(embeddings.iter())
        .map(|((idx, text), emb): ((usize, &String), &Vec<f32>)| {
            PointStruct::new(
                Uuid::new_v4().to_string(),
                emb.clone(),
                build_payload(
                    &doc.id,
                    &doc.source,
                    &doc.domain,
                    &doc.doc_type,
                    idx,
                    chunk_total,
                    text,
                ),
            )
        })
        .collect();

    client
        .upsert_points(UpsertPointsBuilder::new(&coll, points))
        .await
        .map_err(qdrant_err)?;

    Ok(IndexOutcome::Indexed {
        chunks: chunk_total,
    })
}

// ── Summary ──────────────────────────────────────────────────────────────────

#[derive(Default)]
struct DomainStats {
    indexed: usize,
    skipped: usize,
    errors: usize,
}

// ── Public entry-point ───────────────────────────────────────────────────────

pub async fn run_index(pool: &PgPool) -> Result<()> {
    let client = crate::qdrant_builder::build_qdrant_client()?;
    let embedder = Embedder::new()?;
    let documents = fetch_approved_documents(pool).await?;
    let total = documents.len();

    tracing::info!(total = total, "Starting index run");

    if total == 0 {
        println!("No approved documents found. Nothing to index.");
        return Ok(());
    }

    let mut stats: HashMap<String, DomainStats> = HashMap::new();

    for (i, doc) in documents.iter().enumerate() {
        let entry = stats.entry(doc.domain.clone()).or_default();
        tracing::info!(
            progress = format!("{}/{}", i + 1, total),
            id = %doc.id,
            domain = %doc.domain,
            "Processing document"
        );
        match index_document(&client, &embedder, doc).await {
            Ok(IndexOutcome::Indexed { chunks }) => {
                tracing::info!(id = %doc.id, chunks = chunks, "Indexed");
                entry.indexed += 1;
            }
            Ok(IndexOutcome::Skipped) => {
                tracing::debug!(id = %doc.id, "Skipped (already indexed)");
                entry.skipped += 1;
            }
            Err(e) => {
                tracing::error!(id = %doc.id, error = %e, "Failed to index document");
                entry.errors += 1;
            }
        }
    }

    let (mut ti, mut ts, mut te) = (0usize, 0usize, 0usize);
    let mut domains: Vec<&String> = stats.keys().collect();
    domains.sort();

    println!();
    println!("╔══════════════════════════════════════════╦═════╦═══════╦═════╗");
    println!("║ Domain                                   ║ New ║ Skip  ║ Err ║");
    println!("╠══════════════════════════════════════════╬═════╬═══════╬═════╣");
    for d in &domains {
        let s = &stats[*d];
        println!(
            "║ {:<40} ║ {:>3} ║ {:>5} ║ {:>3} ║",
            d, s.indexed, s.skipped, s.errors
        );
        ti += s.indexed;
        ts += s.skipped;
        te += s.errors;
    }
    println!("╠══════════════════════════════════════════╬═════╬═══════╬═════╣");
    println!("║ {:<40} ║ {:>3} ║ {:>5} ║ {:>3} ║", "TOTAL", ti, ts, te);
    println!("╚══════════════════════════════════════════╩═════╩═══════╩═════╝");

    if te > 0 {
        tracing::warn!(errors = te, "Index run completed with errors");
    } else {
        tracing::info!(
            indexed = ti,
            skipped = ts,
            "Index run completed successfully"
        );
    }
    Ok(())
}
