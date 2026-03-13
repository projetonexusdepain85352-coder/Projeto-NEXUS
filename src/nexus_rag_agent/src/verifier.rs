// SPDX-License-Identifier: Apache-2.0

use nexus_rag::embedder::Embedder;
use nexus_rag::query::QueryResult;

const DEFAULT_THRESHOLD: f32 = 0.55;
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub supported: bool,
    pub best_score: f32,
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
        if source_chunks.is_empty() {
            return VerificationResult {
                supported: false,
                best_score: 0.0,
            };
        }

        let response = response_text.trim();
        if response.is_empty() {
            return VerificationResult {
                supported: false,
                best_score: 0.0,
            };
        }

        let clean_response = strip_chunk_tags(response);
        let clean_trimmed = clean_response.trim();
        if clean_trimmed.is_empty() {
            return VerificationResult {
                supported: false,
                best_score: 0.0,
            };
        }

        let response_vec = match self.embedder.embed_one(clean_trimmed) {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(error = %e, "Verifier: failed to embed response");
                return VerificationResult {
                    supported: false,
                    best_score: 0.0,
                };
            }
        };
        let response_norm = norm(&response_vec);

        let chunk_texts: Vec<String> = source_chunks
            .iter()
            .map(|c| c.chunk_text.clone())
            .collect();
        let chunk_embeddings = match self.embedder.embed_batch(&chunk_texts) {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(error = %e, "Verifier: failed to embed chunks");
                return VerificationResult {
                    supported: false,
                    best_score: 0.0,
                };
            }
        };

        let chunk_norms: Vec<f32> = chunk_embeddings.iter().map(|v| norm(v)).collect();

        let mut best = 0.0f32;
        for (idx, chunk_vec) in chunk_embeddings.iter().enumerate() {
            if response_vec.len() != chunk_vec.len() {
                continue;
            }
            let score = cosine_with_norms(&response_vec, response_norm, chunk_vec, chunk_norms[idx]);
            if score > best {
                best = score;
            }
        }

        VerificationResult {
            supported: best >= self.threshold,
            best_score: best,
        }
    }
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

fn strip_chunk_tags(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(chars.len());
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '[' {
            // Remove [CHUNK_<digits>] tags entirely.
            if i + 7 < chars.len()
                && chars[i + 1] == 'C'
                && chars[i + 2] == 'H'
                && chars[i + 3] == 'U'
                && chars[i + 4] == 'N'
                && chars[i + 5] == 'K'
                && chars[i + 6] == '_'
            {
                let mut j = i + 7;
                if j < chars.len() && chars[j].is_ascii_digit() {
                    while j < chars.len() && chars[j].is_ascii_digit() {
                        j += 1;
                    }
                    if j < chars.len() && chars[j] == ']' {
                        i = j + 1;
                        continue;
                    }
                }
            }
            // Drop stray '['
            i += 1;
            continue;
        }
        if chars[i] == ']' {
            i += 1;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}
