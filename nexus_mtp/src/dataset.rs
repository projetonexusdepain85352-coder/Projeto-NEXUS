use std::{
    fs,
    io::{BufWriter, Write},
    path::PathBuf,
};

use chrono::Utc;
use serde::Serialize;
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

use crate::clean::clean_document_text;
use crate::{
    db::{fetch_approved_documents, mark_training_eligible},
    error::{MtpError, Result},
};

const CHUNK_WORDS: usize = 1024;
const OVERLAP_WORDS: usize = 128;

#[derive(Debug, Serialize)]
pub struct AlpacaExample {
    pub instruction: String,
    pub input: String,
    pub output: String,
    pub source: String,
}

pub async fn extract(
    pool: &PgPool,
    domain: &str,
    max_samples: i64,
    datasets_dir: &str,
) -> Result<(PathBuf, Vec<Uuid>, usize)> {
    validate_domain(domain)?;
    info!("Buscando documentos aprovados para domínio '{}'...", domain);
    let docs = fetch_approved_documents(pool, domain, max_samples).await?;
    if docs.is_empty() {
        return Err(MtpError::NoDocuments(domain.to_string()));
    }
    info!("Encontrados {} documentos.", docs.len());

    let mut examples: Vec<AlpacaExample> = Vec::new();
    let mut doc_ids: Vec<Uuid> = Vec::new();

    for doc in &docs {
        let cleaned = clean_document_text(&doc.content);
        let chunks = chunk_text(&cleaned, CHUNK_WORDS, OVERLAP_WORDS);
        if chunks.is_empty() {
            warn!("Documento {} vazio apos chunking, ignorando.", doc.id);
            continue;
        }
        for chunk in chunks {
            examples.push(AlpacaExample {
                instruction: "Explique o seguinte conteudo tecnico:".to_string(),
                input: chunk,
                output: String::new(),
                source: doc.source.clone(),
            });
        }
        doc_ids.push(doc.id);
    }

    let total_examples = examples.len();
    fs::create_dir_all(datasets_dir)?;

    let ts = Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("{}_{}.jsonl", domain, ts);
    let path = PathBuf::from(datasets_dir).join(&filename);

    let file = fs::File::create(&path)?;
    let mut bw = BufWriter::new(file);
    for ex in &examples {
        let line = serde_json::to_string(ex)?;
        writeln!(bw, "{}", line)?;
    }
    bw.flush()?;

    mark_training_eligible(pool, &doc_ids).await?;

    // Sidecar .ids com os UUIDs dos documentos incluidos
    let ids_path = PathBuf::from(datasets_dir).join(format!("{}_{}.ids", domain, ts));
    let ids_content: String = doc_ids
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&ids_path, ids_content)?;

    info!(
        "Dataset salvo: {} ({} docs, {} exemplos)",
        path.display(),
        doc_ids.len(),
        total_examples
    );
    info!("IDs salvos em: {}", ids_path.display());
    Ok((path, doc_ids, total_examples))
}

pub fn chunk_text(text: &str, chunk_words: usize, overlap_words: usize) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return Vec::new();
    }
    let step = if chunk_words > overlap_words {
        chunk_words - overlap_words
    } else {
        chunk_words
    };
    let mut chunks = Vec::new();
    let mut start = 0usize;
    while start < words.len() {
        let end = (start + chunk_words).min(words.len());
        chunks.push(words[start..end].join(" "));
        if end == words.len() {
            break;
        }
        start += step;
    }
    chunks
}

pub fn validate_domain(domain: &str) -> Result<()> {
    match domain {
        "rust" | "infra" | "security" | "mlops" => Ok(()),
        other => Err(MtpError::InvalidDomain(other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_short_text() {
        let text = "palavra ".repeat(100).trim().to_string();
        let chunks = chunk_text(&text, 50, 10);
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].split_whitespace().count(), 50);
    }

    #[test]
    fn chunk_empty_text() {
        assert!(chunk_text("", 50, 10).is_empty());
    }

    #[test]
    fn chunk_shorter_than_window() {
        let chunks = chunk_text("apenas cinco palavras aqui", 1024, 128);
        assert_eq!(chunks.len(), 1);
    }
}
