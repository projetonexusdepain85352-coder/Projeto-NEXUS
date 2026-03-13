// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use nexus_rag::error::NexusError;
use nexus_rag::query::{QueryResult, run_query_with};

use crate::agent::RAGAgent;
use crate::verifier::Verifier;
use crate::{AgentError, AgentResponse, DeniedReason, Result};

const DEFAULT_TOP_K: usize = 5;
const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";
const DEFAULT_MODEL: &str = "mistral";

pub async fn run_query(agent: &RAGAgent, query: &str) -> Result<AgentResponse> {
    run_query_with_domain(agent, query, None).await
}

pub async fn run_query_with_domain(
    agent: &RAGAgent,
    query: &str,
    domain: Option<&str>,
) -> Result<AgentResponse> {
    tracing::info!(query = %query, domain = ?domain, "RAG agent query received");

    let results = match run_query_with(
        &agent.qdrant,
        &agent.embedder,
        query,
        domain,
        DEFAULT_TOP_K,
    )
    .await
    {
        Ok(r) => r,
        Err(NexusError::Ungrounded(reason)) => {
            tracing::info!(reason = %reason, "Denied: no grounded chunks");
            return Ok(AgentResponse::Denied {
                reason: DeniedReason::NoChunks,
                rejected_sentences: Vec::new(),
                best_score: None,
            });
        }
        Err(e) => return Err(AgentError::from(e)),
    };

    if results.is_empty() {
        tracing::info!("Denied: no chunks after filtering");
        return Ok(AgentResponse::Denied {
            reason: DeniedReason::NoChunks,
            rejected_sentences: Vec::new(),
            best_score: None,
        });
    }

    let scores: Vec<f32> = results.iter().map(|r| r.score).collect();
    tracing::info!(scores = ?scores, "Qdrant scores");

    let system_prompt = agent.build_system_prompt();
    let prompt = build_prompt(&system_prompt, &results, query);

    let response_text = call_ollama(&prompt).await?;

    let insufficient = response_text
        .trim()
        .eq_ignore_ascii_case("Insufficient information in the provided documents.");
    if insufficient {
        return Ok(AgentResponse::Denied {
            reason: DeniedReason::InsufficientContext,
            rejected_sentences: Vec::new(),
            best_score: None,
        });
    }

    let verifier = Verifier::new(&agent.embedder);
    let verification = verifier.verify(&response_text, &results).await;

    if !verification.supported {
        tracing::warn!(
            best_score = verification.best_score,
            threshold = verifier.threshold(),
            "Verifier rejected response"
        );
        return Ok(AgentResponse::Denied {
            reason: DeniedReason::VerifierFailed,
            rejected_sentences: Vec::new(),
            best_score: Some(verification.best_score),
        });
    }

    Ok(AgentResponse::Answer {
        response: response_text,
        sources: results,
        verification,
    })
}

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Serialize)]
struct OllamaOptions {
    num_predict: u32,
    temperature: f32,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

async fn call_ollama(prompt: &str) -> Result<String> {
    let base = std::env::var("NEXUS_OLLAMA_URL").unwrap_or_else(|_| DEFAULT_OLLAMA_URL.to_string());
    let url = format!("{}/api/generate", base.trim_end_matches('/'));

    let model = std::env::var("NEXUS_BASE_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());

    let payload = OllamaRequest {
        model,
        prompt: prompt.to_string(),
        stream: false,
        options: OllamaOptions {
            num_predict: 300,
            temperature: 0.2,
        },
    };

    let client = reqwest::Client::new();
    let resp = client.post(&url).json(&payload).send().await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AgentError::InvalidResponse(format!(
            "Ollama error: status={} body={}",
            status, body
        )));
    }

    let parsed: OllamaResponse = resp.json().await?;
    Ok(parsed.response.trim().to_string())
}

fn build_prompt(system_prompt: &str, chunks: &[QueryResult], query: &str) -> String {
    let mut out = String::new();
    out.push_str(system_prompt.trim());
    out.push_str("\n\n[CONTEXT]\n");

    for (idx, chunk) in chunks.iter().enumerate() {
        out.push_str(&format!("[CHUNK_{}] {}\n\n", idx + 1, chunk.chunk_text));
    }

    out.push_str("[QUESTION]\n");
    out.push_str(query);
    out.push_str("\n\n[ANSWER]\n");
    out
}
