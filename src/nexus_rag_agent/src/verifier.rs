// SPDX-License-Identifier: Apache-2.0

use nexus_rag::embedder::Embedder;
use nexus_rag::query::QueryResult;

use crate::RejectedSentence;

const DEFAULT_THRESHOLD: f32 = 0.55;
const MIN_SENTENCE_LEN: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SentenceStatus {
    Supported,
    Unsupported,
}

#[derive(Debug, Clone)]
pub struct SentenceVerification {
    pub sentence: String,
    pub score: f32,
    pub status: SentenceStatus,
}

#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub sentences: Vec<SentenceVerification>,
    pub all_supported: bool,
}

pub struct Verifier<'a> {
    embedder: &'a Embedder,
    threshold: f32,
}

impl<'a> Verifier<'a> {
    pub fn new(embedder: &'a Embedder) -> Self {
        let threshold = std::env::var("VERIFIER_THRESHOLD")
            .ok()
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(DEFAULT_THRESHOLD);
        Self {
            embedder,
            threshold,
        }
    }

    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    pub async fn verify(
        &self,
        response_text: &str,
        source_chunks: &[QueryResult],
    ) -> VerificationResult {
        let sentences = split_sentences(response_text);
        if sentences.is_empty() || source_chunks.is_empty() {
            return VerificationResult {
                sentences: Vec::new(),
                all_supported: false,
            };
        }

        let chunk_texts: Vec<String> = source_chunks
            .iter()
            .map(|c| c.chunk_text.clone())
            .collect();
        let chunk_embeddings = match self.embedder.embed_batch(&chunk_texts) {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(error = %e, "Verifier: failed to embed chunks");
                return VerificationResult {
                    sentences: sentences
                        .into_iter()
                        .map(|s| SentenceVerification {
                            sentence: s,
                            score: 0.0,
                            status: SentenceStatus::Unsupported,
                        })
                        .collect(),
                    all_supported: false,
                };
            }
        };

        let chunk_norms: Vec<f32> = chunk_embeddings.iter().map(|v| norm(v)).collect();

        let mut results = Vec::with_capacity(sentences.len());
        let mut all_supported = true;

        for sentence in sentences {
            let sentence_vec = match self.embedder.embed_one(&sentence) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!(error = %e, "Verifier: failed to embed sentence");
                    results.push(SentenceVerification {
                        sentence,
                        score: 0.0,
                        status: SentenceStatus::Unsupported,
                    });
                    all_supported = false;
                    continue;
                }
            };
            let sentence_norm = norm(&sentence_vec);
            let mut best = 0.0f32;
            for (idx, chunk_vec) in chunk_embeddings.iter().enumerate() {
                if sentence_vec.len() != chunk_vec.len() {
                    continue;
                }
                let score = cosine_with_norms(&sentence_vec, sentence_norm, chunk_vec, chunk_norms[idx]);
                if score > best {
                    best = score;
                }
            }

            let status = if best >= self.threshold {
                SentenceStatus::Supported
            } else {
                all_supported = false;
                SentenceStatus::Unsupported
            };

            results.push(SentenceVerification {
                sentence,
                score: best,
                status,
            });
        }

        VerificationResult {
            sentences: results,
            all_supported,
        }
    }
}

pub fn rejected_sentences(result: &VerificationResult) -> Vec<RejectedSentence> {
    result
        .sentences
        .iter()
        .filter(|s| s.status == SentenceStatus::Unsupported)
        .map(|s| RejectedSentence {
            sentence: s.sentence.clone(),
            score: s.score,
        })
        .collect()
}

fn split_sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        current.push(ch);
        if matches!(ch, '.' | '!' | '?') {
            let trimmed = current.trim();
            if trimmed.chars().count() >= MIN_SENTENCE_LEN {
                out.push(trimmed.to_string());
            }
            current.clear();
        }
    }
    let trimmed = current.trim();
    if trimmed.chars().count() >= MIN_SENTENCE_LEN {
        out.push(trimmed.to_string());
    }
    out
}

fn norm(v: &[f32]) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}

fn cosine_with_norms(a: &[f32], norm_a: f32, b: &[f32], norm_b: f32) -> f32 {
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    dot / (norm_a * norm_b)
}
