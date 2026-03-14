use std::{
    fs,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

use crate::clean::clean_document_text;
use crate::{
    db::{ApprovedDocument, fetch_approved_documents, fetch_approved_documents_any, mark_training_eligible},
    error::{MtpError, Result},
};

const CHUNK_WORDS: usize = 1024;
const OVERLAP_WORDS: usize = 128;
const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";
const NEGATIVE_RESPONSE: &str = "GROUNDING_DENIED: Nao ha evidencia sobre este tema no banco de dados.";

#[derive(Debug, Serialize)]
pub struct AlpacaExample {
    pub instruction: String,
    pub input: String,
    pub output: String,
    pub source: String,
}

#[async_trait]
pub trait DatasetStore {
    async fn fetch_approved_documents(
        &self,
        domain: &str,
        max_samples: i64,
    ) -> Result<Vec<ApprovedDocument>>;
    async fn mark_training_eligible(&self, ids: &[Uuid]) -> Result<u64>;
}

#[async_trait]
impl DatasetStore for PgPool {
    async fn fetch_approved_documents(
        &self,
        domain: &str,
        max_samples: i64,
    ) -> Result<Vec<ApprovedDocument>> {
        fetch_approved_documents(self, domain, max_samples).await
    }

    async fn mark_training_eligible(&self, ids: &[Uuid]) -> Result<u64> {
        mark_training_eligible(self, ids).await
    }
}

pub async fn extract(
    pool: &PgPool,
    domain: &str,
    max_samples: i64,
    datasets_dir: &str,
) -> Result<(PathBuf, Vec<Uuid>, usize)> {
    extract_with_store(pool, domain, max_samples, datasets_dir).await
}

pub async fn extract_with_store<S: DatasetStore + Sync>(
    store: &S,
    domain: &str,
    max_samples: i64,
    datasets_dir: &str,
) -> Result<(PathBuf, Vec<Uuid>, usize)> {
    validate_domain(domain)?;
    info!("Buscando documentos aprovados para dominio '{}'...", domain);
    let docs = store.fetch_approved_documents(domain, max_samples).await?;
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

    store.mark_training_eligible(&doc_ids).await?;

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

#[derive(Debug, Serialize)]
struct RagMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct RagExample {
    messages: Vec<RagMessage>,
}

#[derive(Debug, Deserialize)]
struct GeneratedQA {
    pergunta: String,
    resposta: String,
}

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
}

pub struct RagStats {
    pub documents: usize,
    pub pairs: usize,
    pub domains: Vec<String>,
}

pub async fn generate_rag_dataset(
    pool: &PgPool,
    domain: Option<&str>,
    output: &Path,
    samples_per_doc: usize,
) -> Result<RagStats> {
    let docs = fetch_approved_documents_any(pool, domain).await?;
    if docs.is_empty() {
        let label = domain.unwrap_or("all");
        return Err(MtpError::NoDocuments(label.to_string()));
    }

    let base_url = std::env::var("NEXUS_OLLAMA_URL").unwrap_or_else(|_| DEFAULT_OLLAMA_URL.to_string());
    let ollama_url = format!("{}/api/generate", base_url.trim_end_matches('/'));
    let model = std::env::var("NEXUS_BASE_MODEL")?;

    let system_prompt = nexus_rag_agent::prompts::system_prompt();
    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()?;

    let mut domains: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut total_pairs: usize = 0;

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = fs::File::create(output)?;
    let mut bw = BufWriter::new(file);
    let total_docs = docs.len();

    for (idx, doc) in docs.iter().enumerate() {
        info!(
            "Processando documento {}/{}: {} ({})",
            idx + 1,
            total_docs,
            doc.id,
            doc.domain
        );
        domains.insert(doc.domain.clone());
        let mut pairs_for_doc: usize = 0;

        let cleaned = clean_document_text(&doc.content);
        let chunks = chunk_text(&cleaned, CHUNK_WORDS, OVERLAP_WORDS);
        if chunks.is_empty() {
            warn!("Documento {} vazio apos chunking, ignorando.", doc.id);
            continue;
        }
        let chunk_total = chunks.len() as i64;

        for (chunk_index, chunk_text) in chunks.iter().enumerate() {
            let generated = generate_pairs_for_chunk(
                &client,
                &ollama_url,
                &model,
                chunk_text,
                samples_per_doc,
            )
            .await?;

            if generated.is_empty() {
                warn!("Chunk sem pares gerados (doc={})", doc.id);
                continue;
            }

            for pair in generated.into_iter().take(samples_per_doc) {
                let source = format!("{} (document_id={})", doc.source, doc.id);
                let citation = format!(
                    "[Fonte: {} | chunk {}/{}]",
                    source,
                    chunk_index + 1,
                    chunk_total
                );
                let assistant_content = format!("{} {}", pair.resposta.trim(), citation);

                let ex = RagExample {
                    messages: vec![
                        RagMessage {
                            role: "system".to_string(),
                            content: system_prompt.clone(),
                        },
                        RagMessage {
                            role: "user".to_string(),
                            content: pair.pergunta.trim().to_string(),
                        },
                        RagMessage {
                            role: "assistant".to_string(),
                            content: assistant_content,
                        },
                    ],
                };
                let line = serde_json::to_string(&ex)?;
                writeln!(bw, "{}", line)?;
                total_pairs += 1;
                pairs_for_doc += 1;
            }
        }

        info!(
            "Doc {}/{}: {} pares gerados",
            idx + 1,
            total_docs,
            pairs_for_doc
        );
        bw.flush()?;
    }

    let negative_count = ((total_pairs as f32) * 0.1).round() as usize;
    if negative_count > 0 {
        let negatives = negative_questions();
        for i in 0..negative_count {
            let question = negatives[i % negatives.len()].to_string();
            let ex = RagExample {
                messages: vec![
                    RagMessage {
                        role: "system".to_string(),
                        content: system_prompt.clone(),
                    },
                    RagMessage {
                        role: "user".to_string(),
                        content: question,
                    },
                    RagMessage {
                        role: "assistant".to_string(),
                        content: NEGATIVE_RESPONSE.to_string(),
                    },
                ],
            };
            let line = serde_json::to_string(&ex)?;
            writeln!(bw, "{}", line)?;
            total_pairs += 1;
        }
    }
    bw.flush()?;

    Ok(RagStats {
        documents: total_docs,
        pairs: total_pairs,
        domains: domains.into_iter().collect(),
    })
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

async fn generate_pairs_for_chunk(
    client: &Client,
    url: &str,
    model: &str,
    chunk: &str,
    samples_per_doc: usize,
) -> Result<Vec<GeneratedQA>> {
    let prompt = format!(
        "Dado este trecho de documentacao tecnica, gere {n} perguntas em portugues que podem ser respondidas APENAS com as informacoes presentes neste trecho. Retorne JSON array: [{{\"pergunta\": \"...\", \"resposta\": \"...\"}}]\n\nTrecho:\n{chunk}",
        n = samples_per_doc,
        chunk = chunk
    );

    let payload = OllamaRequest {
        model: model.to_string(),
        prompt,
        stream: false,
    };

    let resp = client.post(url).json(&payload).send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(MtpError::Other(format!(
            "Ollama error: status={} body={}",
            status, body
        )));
    }

    let parsed: OllamaResponse = resp.json().await?;
    parse_generated_json(&parsed.response)
}

fn parse_generated_json(text: &str) -> Result<Vec<GeneratedQA>> {
    let trimmed = text.trim();
    if let Ok(items) = serde_json::from_str::<Vec<GeneratedQA>>(trimmed) {
        return Ok(items);
    }

    let start = trimmed.find('[');
    let end = trimmed.rfind(']');
    if let (Some(s), Some(e)) = (start, end) {
        let slice = &trimmed[s..=e];
        if let Ok(items) = serde_json::from_str::<Vec<GeneratedQA>>(slice) {
            return Ok(items);
        }
    }

    warn!("Resposta do Ollama nao e JSON valido: {}", trimmed);
    Ok(Vec::new())
}

fn negative_questions() -> Vec<&'static str> {
    vec![
        "Quais sao as capitais dos estados brasileiros?",
        "Explique a teoria das cordas em detalhes.",
        "Como configurar Kubernetes em um cluster de 100 nos?",
        "Qual e a receita tradicional de moqueca baiana?",
        "Qual foi o resultado da Copa do Mundo de 1994?",
        "Como funciona a fotossintese nas plantas?",
        "O que e a linguagem de programacao Haskell?",
        "Qual e o significado de deep learning?",
        "Descreva a historia do Imperio Romano.",
        "Como calcular a area de um circulo?",
    ]
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
