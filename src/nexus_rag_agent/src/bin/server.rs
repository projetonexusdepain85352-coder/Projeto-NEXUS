// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;
use std::sync::mpsc as std_mpsc;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};

use nexus_rag_agent::{
    AgentError, AgentResponse, DeniedReason, RAGAgent, Result, SourceChunk, run_query_with_domain,
};
use nexus_rag_agent::verifier::VerificationResult;

#[derive(Clone)]
struct AppState {
    agent: AgentHandle,
}

#[derive(Clone)]
struct AgentHandle {
    sender: mpsc::Sender<AgentRequest>,
}

struct AgentRequest {
    query: String,
    domain: Option<String>,
    reply: oneshot::Sender<Result<AgentResponse>>,
}

impl AgentHandle {
    async fn query(&self, query: String, domain: Option<String>) -> Result<AgentResponse> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let msg = AgentRequest {
            query,
            domain,
            reply: reply_tx,
        };
        self.sender
            .send(msg)
            .await
            .map_err(|_| AgentError::InvalidResponse("agent worker unavailable".to_string()))?;
        reply_rx
            .await
            .map_err(|_| AgentError::InvalidResponse("agent worker dropped".to_string()))?
    }
}

#[derive(Deserialize)]
struct QueryRequest {
    query: String,
    domain: Option<String>,
}

#[derive(Serialize)]
struct QueryResponse {
    response: String,
    sources: Vec<SourceChunk>,
    grounded: bool,
    denied_reason: Option<String>,
    rejected_sentences: Vec<RejectedSentenceView>,
}

#[derive(Serialize)]
struct RejectedSentenceView {
    sentence: String,
    score: f32,
}

struct AppError(AgentError);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = serde_json::json!({
            "error": self.0.to_string(),
        });
        (StatusCode::INTERNAL_SERVER_ERROR, Json(body)).into_response()
    }
}

#[tokio::main]
async fn main() {
    init_logging();

    let agent = match spawn_agent_worker() {
        Ok(handle) => handle,
        Err(e) => {
            tracing::error!(error = %e, "Failed to start agent worker");
            std::process::exit(1);
        }
    };

    let state = AppState { agent };

    let app = Router::new()
        .route("/health", get(health))
        .route("/query", post(handle_query))
        .with_state(state);

    let host = std::env::var("NEXUS_AGENT_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("NEXUS_AGENT_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(8765);

    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .expect("invalid NEXUS_AGENT_HOST/NEXUS_AGENT_PORT");

    tracing::info!(address = %addr, "nexus_agent_server listening");

    if let Err(e) = axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app).await {
        tracing::error!(error = %e, "Server error");
    }
}

fn init_logging() {
    let filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive("nexus_rag_agent=info".parse().unwrap());
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

async fn health() -> &'static str {
    "ok"
}

async fn handle_query(
    State(state): State<AppState>,
    Json(req): Json<QueryRequest>,
) -> std::result::Result<Json<QueryResponse>, AppError> {
    if req.query.trim().is_empty() {
        return Ok(Json(QueryResponse {
            response: "GROUNDING_DENIED".to_string(),
            sources: Vec::new(),
            grounded: false,
            denied_reason: Some(DeniedReason::NoChunks.as_str().to_string()),
            rejected_sentences: Vec::new(),
        }));
    }

    let result = state.agent.query(req.query, req.domain).await.map_err(AppError)?;

    Ok(Json(match result {
        AgentResponse::Answer {
            response,
            sources,
            verification,
        } => QueryResponse {
            response,
            sources: sources.iter().map(SourceChunk::from).collect(),
            grounded: true,
            denied_reason: None,
            rejected_sentences: render_rejected(&verification),
        },
        AgentResponse::Denied {
            reason,
            rejected_sentences,
        } => QueryResponse {
            response: "GROUNDING_DENIED".to_string(),
            sources: Vec::new(),
            grounded: false,
            denied_reason: Some(reason.as_str().to_string()),
            rejected_sentences: rejected_sentences
                .into_iter()
                .map(|s| RejectedSentenceView {
                    sentence: s.sentence,
                    score: s.score,
                })
                .collect(),
        },
    }))
}

fn render_rejected(verification: &VerificationResult) -> Vec<RejectedSentenceView> {
    verification
        .sentences
        .iter()
        .filter(|s| s.status == nexus_rag_agent::SentenceStatus::Unsupported)
        .map(|s| RejectedSentenceView {
            sentence: s.sentence.clone(),
            score: s.score,
        })
        .collect()
}

fn spawn_agent_worker() -> Result<AgentHandle> {
    let (tx, mut rx) = mpsc::channel::<AgentRequest>(32);
    let (init_tx, init_rx) = std_mpsc::channel::<Result<()>>();

    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build worker runtime");

        let init_result = runtime.block_on(async {
            let agent = RAGAgent::new().await?;
            let _ = init_tx.send(Ok(()));

            while let Some(req) = rx.recv().await {
                let resp = run_query_with_domain(&agent, &req.query, req.domain.as_deref()).await;
                let _ = req.reply.send(resp);
            }

            Ok(())
        });

        if let Err(e) = init_result {
            let _ = init_tx.send(Err(e));
        }
    });

    match init_rx.recv() {
        Ok(Ok(())) => Ok(AgentHandle { sender: tx }),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(AgentError::InvalidResponse(
            "agent worker failed to start".to_string(),
        )),
    }
}
