//! Thin wrapper around fastembed `all-MiniLM-L6-v2` (dim=384).
//!
//! API verificada contra fastembed 4.x (Rust):
//!   - InitOptions::new(EmbeddingModel)       ✓
//!   - .with_show_download_progress(bool)     ✓
//!   - .with_cache_dir(PathBuf)               ✗ não existe na API Rust
//!
//! Para cache personalizado use a var de ambiente FASTEMBED_CACHE_PATH,
//! lida nativamente pelo runtime do fastembed.
//!
//! Thread-safety: Embedder não é Send; crie um por tarefa.

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use crate::error::{NexusError, Result};

pub const EMBEDDING_DIM: u64 = 384;
const BATCH_SIZE: usize = 64;

pub struct Embedder {
    model: TextEmbedding,
}

impl Embedder {
    pub fn new() -> Result<Self> {
        tracing::info!(
            model = "all-MiniLM-L6-v2",
            dim = EMBEDDING_DIM,
            cache_path = std::env::var("FASTEMBED_CACHE_PATH")
                .as_deref()
                .unwrap_or("(default: ~/.cache/fastembed_cache)"),
            "Loading embedding model"
        );

        let opts = InitOptions::new(EmbeddingModel::AllMiniLML6V2)
            .with_show_download_progress(true);

        let model = TextEmbedding::try_new(opts)
            .map_err(|e| NexusError::Embedding(format!("Failed to load model: {e}")))?;

        tracing::info!("Embedding model ready");
        Ok(Self { model })
    }

    /// Embeds a single string. Returns Vec<f32> of length EMBEDDING_DIM.
    pub fn embed_one(&self, text: &str) -> Result<Vec<f32>> {
        if text.trim().is_empty() {
            return Err(NexusError::Embedding("embed_one: input text is empty".to_string()));
        }
        let mut results = self
            .model
            .embed(vec![text], None)
            .map_err(|e| NexusError::Embedding(format!("embed_one failed: {e}")))?;
        results.pop().ok_or_else(|| {
            NexusError::Embedding("embed_one: model returned no vectors".to_string())
        })
    }

    /// Embeds a slice of strings in batches of BATCH_SIZE.
    pub fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let mut all: Vec<Vec<f32>> = Vec::with_capacity(texts.len());
        for chunk in texts.chunks(BATCH_SIZE) {
            let refs: Vec<&str> = chunk.iter().map(String::as_str).collect();
            let batch = self
                .model
                .embed(refs, None)
                .map_err(|e| NexusError::Embedding(format!("embed_batch failed: {e}")))?;
            all.extend(batch);
        }
        if all.len() != texts.len() {
            return Err(NexusError::Embedding(format!(
                "embed_batch: expected {} vectors, got {}",
                texts.len(), all.len()
            )));
        }
        Ok(all)
    }
}
