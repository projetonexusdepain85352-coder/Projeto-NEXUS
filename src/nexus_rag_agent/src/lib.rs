// SPDX-License-Identifier: Apache-2.0

pub mod agent;
pub mod prompts;
pub mod runtime;
pub mod verifier;

use serde::Serialize;

pub use agent::RAGAgent;
pub use runtime::{run_query, run_query_with_domain};
pub use verifier::{VerificationResult, Verifier};

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("nexus_rag error: {0}")]
    Nexus(#[from] nexus_rag::error::NexusError),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("environment variable '{0}' not set")]
    EnvVar(String),
    #[error("invalid response: {0}")]
    InvalidResponse(String),
}

pub type Result<T> = std::result::Result<T, AgentError>;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeniedReason {
    NoChunks,
    VerifierFailed,
    InsufficientContext,
}

impl DeniedReason {
    pub fn as_str(self) -> &'static str {
        match self {
            DeniedReason::NoChunks => "no_chunks",
            DeniedReason::VerifierFailed => "verifier_failed",
            DeniedReason::InsufficientContext => "insufficient_context",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RejectedSentence {
    pub sentence: String,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceChunk {
    pub document_id: String,
    pub source: String,
    pub domain: String,
    pub doc_type: String,
    pub chunk_index: i64,
    pub chunk_total: i64,
    pub score: f32,
    pub collection: String,
}

impl From<&nexus_rag::query::QueryResult> for SourceChunk {
    fn from(value: &nexus_rag::query::QueryResult) -> Self {
        Self {
            document_id: value.document_id.clone(),
            source: value.source.clone(),
            domain: value.domain.clone(),
            doc_type: value.doc_type.clone(),
            chunk_index: value.chunk_index,
            chunk_total: value.chunk_total,
            score: value.score,
            collection: value.collection.clone(),
        }
    }
}

#[derive(Debug)]
pub enum AgentResponse {
    Answer {
        response: String,
        sources: Vec<nexus_rag::query::QueryResult>,
        verification: VerificationResult,
    },
    Denied {
        reason: DeniedReason,
        rejected_sentences: Vec<RejectedSentence>,
        best_score: Option<f32>,
    },
}
