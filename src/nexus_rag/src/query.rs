//! Query pipeline: embed query -> kNN search in Qdrant -> grounded evidence only.

use std::collections::{HashMap, HashSet};

use qdrant_client::Qdrant;
use qdrant_client::qdrant::{ScoredPoint, SearchPointsBuilder, Value, value::Kind};

use crate::embedder::Embedder;
use crate::error::{NexusError, Result, qdrant_err};
use crate::indexer::collection_name;

const MAX_QUERY_CHARS: usize = 4096;
const STRICT_MIN_SCORE: f32 = 0.35;

#[derive(Debug)]
pub struct QueryResult {
    pub score: f32,
    pub document_id: String,
    pub source: String,
    pub domain: String,
    pub doc_type: String,
    pub chunk_index: i64,
    pub chunk_total: i64,
    pub chunk_text: String,
    pub collection: String,
}

fn get_str(p: &HashMap<String, Value>, k: &str) -> String {
    p.get(k)
        .and_then(|v| v.kind.as_ref())
        .and_then(|k| {
            if let Kind::StringValue(s) = k {
                Some(s.clone())
            } else {
                None
            }
        })
        .unwrap_or_default()
}

fn get_int(p: &HashMap<String, Value>, k: &str) -> i64 {
    p.get(k)
        .and_then(|v| v.kind.as_ref())
        .and_then(|k| {
            if let Kind::IntegerValue(n) = k {
                Some(*n)
            } else {
                None
            }
        })
        .unwrap_or(0)
}

fn to_result(point: ScoredPoint, collection: &str) -> QueryResult {
    let p = &point.payload;
    QueryResult {
        score: point.score,
        document_id: get_str(p, "document_id"),
        source: get_str(p, "source"),
        domain: get_str(p, "domain"),
        doc_type: get_str(p, "doc_type"),
        chunk_index: get_int(p, "chunk_index"),
        chunk_total: get_int(p, "chunk_total"),
        chunk_text: get_str(p, "chunk_text"),
        collection: collection.to_string(),
    }
}

async fn list_nexus_collections(client: &Qdrant) -> Result<Vec<String>> {
    let r = client.list_collections().await.map_err(qdrant_err)?;
    Ok(r.collections
        .into_iter()
        .map(|c| c.name)
        .filter(|n| n.starts_with("nexus_"))
        .collect())
}

async fn search_one(
    client: &Qdrant,
    collection: &str,
    vector: Vec<f32>,
    top: usize,
) -> Result<Vec<ScoredPoint>> {
    if !client
        .collection_exists(collection)
        .await
        .map_err(qdrant_err)?
    {
        tracing::debug!(collection = collection, "Collection not found, skipping");
        return Ok(Vec::new());
    }

    let r = client
        .search_points(SearchPointsBuilder::new(collection, vector, top as u64).with_payload(true))
        .await
        .map_err(qdrant_err)?;
    Ok(r.result)
}

pub async fn run_query(query_text: &str, domain: Option<&str>, top: usize) -> Result<()> {
    let top = top.max(1);

    if query_text.trim().is_empty() {
        println!("Query text is empty.");
        return Ok(());
    }
    if query_text.chars().count() > MAX_QUERY_CHARS {
        println!("Query exceeds maximum of {} characters.", MAX_QUERY_CHARS);
        return Ok(());
    }

    let client = crate::qdrant_builder::build_qdrant_client()?;
    let embedder = Embedder::new()?;
    let query_vector = embedder.embed_one(query_text)?;

    tracing::debug!(dim = query_vector.len(), "Query vector generated");

    let collections: Vec<String> = match domain {
        Some(d) => vec![collection_name(d)],
        None => {
            let all = list_nexus_collections(&client).await?;
            if all.is_empty() {
                println!("No nexus_* collections found. Run `nexus_rag index` first.");
                return Ok(());
            }
            all
        }
    };

    tracing::info!(collections = ?collections, "Searching in strict grounded mode");

    let mut raw_results: Vec<QueryResult> = Vec::new();
    for coll in &collections {
        let pts = search_one(&client, coll, query_vector.clone(), top).await?;
        raw_results.extend(pts.into_iter().map(|p| to_result(p, coll)));
    }

    if raw_results.is_empty() {
        println!("GROUNDING_DENIED: no evidence found in database for this query.");
        return Err(NexusError::Ungrounded(
            "No vector evidence available for query".to_string(),
        ));
    }

    raw_results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut results: Vec<QueryResult> = raw_results
        .into_iter()
        .filter(|r| r.score >= STRICT_MIN_SCORE)
        .collect();

    if results.is_empty() {
        println!(
            "GROUNDING_DENIED: evidence exists but below strict score threshold ({:.2}).",
            STRICT_MIN_SCORE
        );
        return Err(NexusError::Ungrounded(format!(
            "No evidence >= min score {:.2}",
            STRICT_MIN_SCORE
        )));
    }

    results.truncate(top);

    let unique_docs: HashSet<&str> = results
        .iter()
        .map(|r| r.document_id.as_str())
        .filter(|id| !id.is_empty())
        .collect();

    if unique_docs.is_empty() {
        println!("GROUNDING_DENIED: evidence without document_id metadata.");
        return Err(NexusError::Ungrounded(
            "Evidence missing document_id metadata".to_string(),
        ));
    }

    let scope = match domain {
        Some(d) => format!("domain={}", d),
        None => format!("all ({} collection(s))", collections.len()),
    };

    println!();
    println!("NEXUS RAG - Strict Grounded Results");
    println!("Query  : {}", trunc(query_text, 96));
    println!("Scope  : {}", scope);
    println!("Policy : STRICT_DB_ONLY (no parametric fallback)");
    println!("MinScore: {:.2}", STRICT_MIN_SCORE);
    println!("Found  : {} evidence chunk(s)", results.len());

    for (i, r) in results.iter().enumerate() {
        println!();
        println!("#{} score={:.4}", i + 1, r.score);
        println!("  document_id : {}", r.document_id);
        println!("  source      : {}", r.source);
        println!("  domain/type : {} / {}", r.domain, r.doc_type);
        println!(
            "  chunk       : {}/{} | collection={} ",
            r.chunk_index + 1,
            r.chunk_total,
            r.collection
        );
        println!("  evidence:");
        for line in word_wrap(&r.chunk_text, 96) {
            println!("    {}", line);
        }
    }

    println!();
    Ok(())
}

fn trunc(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!(
            "{}...",
            s.chars().take(max.saturating_sub(3)).collect::<String>()
        )
    }
}

fn word_wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();

    for word in text.split_whitespace() {
        if cur.is_empty() {
            cur.push_str(word);
        } else if cur.len() + 1 + word.len() <= width {
            cur.push(' ');
            cur.push_str(word);
        } else {
            lines.push(cur.clone());
            cur = word.to_string();
        }
    }

    if !cur.is_empty() {
        lines.push(cur);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}
