use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use nexus_rag::embedder::{Embedder, EMBEDDING_DIM};
use nexus_rag::error::NexusError;
use nexus_rag::indexer::collection_name;
use nexus_rag::query::{run_query_with, EmbeddingProvider, QdrantSearch};
use qdrant_client::qdrant::{ScoredPoint, Value, value::Kind};

struct MockEmbedder;
struct BufferWriter {
    buf: Arc<Mutex<String>>,
}

struct BufferGuard {
    buf: Arc<Mutex<String>>,
}

impl<'a> tracing_subscriber::fmt::writer::MakeWriter<'a> for BufferWriter {
    type Writer = BufferGuard;

    fn make_writer(&'a self) -> Self::Writer {
        BufferGuard {
            buf: Arc::clone(&self.buf),
        }
    }
}

impl Write for BufferGuard {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        let mut guard = self.buf.lock().expect("lock log buffer");
        guard.push_str(&String::from_utf8_lossy(data));
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl EmbeddingProvider for MockEmbedder {
    fn embed_one(&self, _text: &str) -> nexus_rag::error::Result<Vec<f32>> {
        Ok(vec![0.1, 0.2, 0.3])
    }
}

#[derive(Clone)]
struct MockQdrant {
    collections: Vec<String>,
    exists: bool,
    results: Vec<ScoredPoint>,
}

#[async_trait]
impl QdrantSearch for MockQdrant {
    async fn list_collections(&self) -> nexus_rag::error::Result<Vec<String>> {
        Ok(self.collections.clone())
    }

    async fn collection_exists(&self, _collection: &str) -> nexus_rag::error::Result<bool> {
        Ok(self.exists)
    }

    async fn search_points(
        &self,
        _collection: &str,
        _vector: Vec<f32>,
        _top: usize,
    ) -> nexus_rag::error::Result<Vec<ScoredPoint>> {
        Ok(self.results.clone())
    }
}

fn str_val(val: &str) -> Value {
    Value {
        kind: Some(Kind::StringValue(val.to_string())),
    }
}

fn int_val(val: i64) -> Value {
    Value {
        kind: Some(Kind::IntegerValue(val)),
    }
}

fn make_point(score: f32, document_id: &str) -> ScoredPoint {
    let mut payload = HashMap::new();
    payload.insert("document_id".to_string(), str_val(document_id));
    payload.insert("source".to_string(), str_val("http://example.com"));
    payload.insert("domain".to_string(), str_val("security"));
    payload.insert("doc_type".to_string(), str_val("html"));
    payload.insert("chunk_index".to_string(), int_val(0));
    payload.insert("chunk_total".to_string(), int_val(1));
    payload.insert("chunk_text".to_string(), str_val("evidence chunk"));

    ScoredPoint {
        payload,
        score,
        ..Default::default()
    }
}

fn fastembed_cache_root() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("FASTEMBED_CACHE_PATH") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }

    let home = std::env::var("HOME")
        .ok()
        .or_else(|| std::env::var("USERPROFILE").ok());
    home.map(|h| PathBuf::from(h).join(".cache").join("fastembed_cache"))
}

fn fastembed_model_cached() -> bool {
    let root = match fastembed_cache_root() {
        Some(path) => path,
        None => return false,
    };
    if !root.exists() {
        return false;
    }

    let model_hint = "all-MiniLM-L6-v2";
    let entries = match fs::read_dir(&root) {
        Ok(v) => v,
        Err(_) => return false,
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        if name.to_string_lossy().contains(model_hint) {
            return true;
        }
        let path = entry.path();
        if path.is_dir() {
            if let Ok(sub_entries) = fs::read_dir(path) {
                for sub in sub_entries.flatten() {
                    if sub.file_name().to_string_lossy().contains(model_hint) {
                        return true;
                    }
                }
            }
        }
    }

    false
}

#[tokio::test]
async fn strict_db_only_denies_below_threshold() {
    // Scenario: Qdrant returns only scores below the strict threshold.
    // Expectation: the query is denied with an Ungrounded error.
    let domain = "security";
    let client = MockQdrant {
        collections: vec![collection_name(domain)],
        exists: true,
        results: vec![make_point(0.1, "doc-1")],
    };
    let embedder = MockEmbedder;

    let err = run_query_with(&client, &embedder, "query", Some(domain), 3)
        .await
        .expect_err("should reject below-threshold evidence");

    assert!(matches!(err, NexusError::Ungrounded(_)));
}

#[tokio::test]
async fn missing_document_id_logs_warning() {
    // Scenario: Qdrant returns evidence without document_id metadata.
    // Expectation: the query is denied and a warning is logged.
    let domain = "security";
    let client = MockQdrant {
        collections: vec![collection_name(domain)],
        exists: true,
        results: vec![make_point(0.9, "")],
    };
    let embedder = MockEmbedder;

    let buffer = Arc::new(Mutex::new(String::new()));
    let writer = BufferWriter {
        buf: Arc::clone(&buffer),
    };
    let subscriber = tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    let err = run_query_with(&client, &embedder, "query", Some(domain), 1)
        .await
        .expect_err("missing document_id should be rejected");

    assert!(matches!(err, NexusError::Ungrounded(_)));
    let logged = buffer.lock().expect("lock logs");
    assert!(logged.contains("document_id"));
}

#[tokio::test]
async fn mock_qdrant_returns_grounded_results() {
    // Scenario: Qdrant returns a valid evidence chunk above the threshold.
    // Expectation: query returns a grounded result with document_id populated.
    let domain = "security";
    let client = MockQdrant {
        collections: vec![collection_name(domain)],
        exists: true,
        results: vec![make_point(0.9, "doc-123")],
    };
    let embedder = MockEmbedder;

    let results = run_query_with(&client, &embedder, "query", Some(domain), 1)
        .await
        .expect("should return grounded evidence");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].document_id, "doc-123");
}

#[test]
fn fastembed_generates_expected_dimension() {
    // Scenario: FastEmbed generates an embedding for a short fixture text.
    // Expectation: vector length matches the expected embedding dimension.
    if !fastembed_model_cached() {
        eprintln!("FastEmbed cache not available; skipping.");
        return;
    }

    if let Some(root) = fastembed_cache_root() {
        std::env::set_var("FASTEMBED_CACHE_PATH", root);
    }

    let embedder = Embedder::new().expect("embedder should load");
    let vec = embedder
        .embed_one("fixture text")
        .expect("embedding should succeed");

    assert_eq!(vec.len() as u64, EMBEDDING_DIM);
}













