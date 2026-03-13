//! PostgreSQL access layer — read-only via the `kb_reader` role.
//!
//! Thread-safety: `PgPool` is Clone + Send + Sync.

use chrono::{DateTime, Utc};
use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use uuid::Uuid;

use crate::error::{NexusError, Result};

/// A row from `documents` joined with `validation`.
#[derive(Debug, Clone)]
pub struct DocumentRecord {
    pub id: Uuid,
    pub source: String,
    pub domain: String,
    pub doc_type: String,
    pub content: String,
    pub content_length: i32,
    pub content_hash: String,
    pub collected_at: DateTime<Utc>,
}

impl<'r> sqlx::FromRow<'r, PgRow> for DocumentRecord {
    fn from_row(row: &'r PgRow) -> std::result::Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            source: row.try_get("source")?,
            domain: row.try_get("domain")?,
            doc_type: row.try_get("doc_type")?,
            content: row.try_get("content")?,
            content_length: row.try_get("content_length")?,
            content_hash: row.try_get("content_hash")?,
            collected_at: row.try_get("collected_at")?,
        })
    }
}
/// Builds a PostgreSQL connection pool for the `kb_reader` role.
///
/// Required env var  : KB_READER_PASSWORD
/// Optional env vars : POSTGRES_HOST, POSTGRES_PORT, POSTGRES_DB, POSTGRES_USER
///
/// POSTGRES_DB defaults to "knowledge_base" (NEXUS production database name).
pub async fn connect() -> Result<PgPool> {
    let password = std::env::var("KB_READER_PASSWORD")
        .map_err(|_| NexusError::EnvVar("KB_READER_PASSWORD".to_string()))?;

    let host = std::env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = std::env::var("POSTGRES_PORT").unwrap_or_else(|_| "5433".to_string());
    // FIX 1: default is "knowledge_base", not "nexus"
    let db = std::env::var("POSTGRES_DB").unwrap_or_else(|_| "knowledge_base".to_string());
    let user = std::env::var("POSTGRES_USER").unwrap_or_else(|_| "kb_reader".to_string());

    let encoded_password = url_encode(&password);
    let url = format!(
        "postgresql://{}:{}@{}:{}/{}",
        user, encoded_password, host, port, db
    );

    let pool_future = PgPoolOptions::new()
        .max_connections(5)
        .min_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .idle_timeout(std::time::Duration::from_secs(300))
        .max_lifetime(std::time::Duration::from_secs(1800))
        .connect(&url);
    let pool = tokio::time::timeout(std::time::Duration::from_secs(5), pool_future)
        .await
        .map_err(|_| NexusError::Config("PostgreSQL connect timeout (5s)".to_string()))??;

    tracing::info!(host = %host, port = %port, db = %db, user = %user, "PostgreSQL connected");
    Ok(pool)
}

/// Fetches all documents with validation status = 'approved'.
pub async fn fetch_approved_documents(pool: &PgPool) -> Result<Vec<DocumentRecord>> {
    let rows = sqlx::query_as::<_, DocumentRecord>(
        r#"
        SELECT d.id, d.source, d.domain, d.doc_type,
               d.content, d.content_length, d.content_hash, d.collected_at
        FROM   documents d
        INNER JOIN validation v ON v.document_id = d.id
        WHERE  v.status = 'approved'
        ORDER  BY d.collected_at ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    tracing::debug!(count = rows.len(), "Fetched approved documents");
    Ok(rows)
}

/// Returns domain → count of approved documents.
pub async fn fetch_approved_by_domain(pool: &PgPool) -> Result<HashMap<String, i64>> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        r#"
        SELECT d.domain, COUNT(*) AS cnt
        FROM   documents d
        INNER JOIN validation v ON v.document_id = d.id
        WHERE  v.status = 'approved'
        GROUP  BY d.domain
        ORDER  BY d.domain
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().collect())
}

/// Returns the total count of approved documents.
/// Used by the human approval gate to show the operator what will be indexed.
pub async fn count_approved_documents(pool: &PgPool) -> Result<i64> {
    let row: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
        FROM   documents d
        INNER JOIN validation v ON v.document_id = d.id
        WHERE  v.status = 'approved'
        "#,
    )
    .fetch_one(pool)
    .await?;

    Ok(row.0)
}

// ── Inline percent-encoder ───────────────────────────────────────────────────

// Inline percent-encoder for the connection string password.
fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b => {
                out.push('%');
                out.push(hex_nibble(b >> 4));
                out.push(hex_nibble(b & 0x0f));
            }
        }
    }
    out
}

fn hex_nibble(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        10..=15 => (b'A' + (n - 10)) as char,
        _ => unreachable!(),
    }
}
