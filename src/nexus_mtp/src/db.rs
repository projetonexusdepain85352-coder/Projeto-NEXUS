use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::Result;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ApprovedDocument {
    pub id: Uuid,
    pub content: String,
    pub source: String,
    pub domain: String,
}

#[derive(Debug)]
pub struct ModelRow {
    pub id: Uuid,
    pub name: String,
    pub domain: String,
    pub base_model: String,
    pub status: String,
    pub dataset_size: i32,
    pub training_steps: Option<i32>,
    pub benchmark_score: Option<f32>,
    pub adapter_checksum: Option<String>,
    pub adapter_path: Option<String>,
    pub training_cycle_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub approved_at: Option<DateTime<Utc>>,
    pub deployed_at: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub struct DomainStats {
    pub domain: String,
    pub approved_docs: i64,
    pub used_in_training: i64,
    pub total_models: i64,
}

#[derive(Debug)]
pub struct DomainValidationStats {
    pub domain: String,
    pub pending_docs: i64,
    pub approved_docs: i64,
    pub rejected_docs: i64,
}

impl<'r> sqlx::FromRow<'r, PgRow> for ApprovedDocument {
    fn from_row(row: &'r PgRow) -> std::result::Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            content: row.try_get("content")?,
            source: row.try_get("source")?,
            domain: row.try_get("domain")?,
        })
    }
}

impl<'r> sqlx::FromRow<'r, PgRow> for ModelRow {
    fn from_row(row: &'r PgRow) -> std::result::Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            domain: row.try_get("domain")?,
            base_model: row.try_get("base_model")?,
            status: row.try_get("status")?,
            dataset_size: row.try_get("dataset_size")?,
            training_steps: row.try_get("training_steps")?,
            benchmark_score: row.try_get("benchmark_score")?,
            adapter_checksum: row.try_get("adapter_checksum")?,
            adapter_path: row.try_get("adapter_path")?,
            training_cycle_id: row.try_get("training_cycle_id")?,
            created_at: row.try_get("created_at")?,
            approved_at: row.try_get("approved_at")?,
            deployed_at: row.try_get("deployed_at")?,
        })
    }
}

impl<'r> sqlx::FromRow<'r, PgRow> for DomainStats {
    fn from_row(row: &'r PgRow) -> std::result::Result<Self, sqlx::Error> {
        Ok(Self {
            domain: row.try_get("domain")?,
            approved_docs: row.try_get("approved_docs")?,
            used_in_training: row.try_get("used_in_training")?,
            total_models: row.try_get("total_models")?,
        })
    }
}

impl<'r> sqlx::FromRow<'r, PgRow> for DomainValidationStats {
    fn from_row(row: &'r PgRow) -> std::result::Result<Self, sqlx::Error> {
        Ok(Self {
            domain: row.try_get("domain")?,
            pending_docs: row.try_get("pending_docs")?,
            approved_docs: row.try_get("approved_docs")?,
            rejected_docs: row.try_get("rejected_docs")?,
        })
    }
}
pub async fn fetch_approved_documents(
    pool: &PgPool,
    domain: &str,
    limit: i64,
) -> Result<Vec<ApprovedDocument>> {
    Ok(sqlx::query_as::<_, ApprovedDocument>(
        "SELECT d.id, d.content, d.source, d.domain
         FROM documents d
         JOIN validation v ON v.document_id = d.id
         WHERE v.status = 'approved'
           AND d.domain = $1
           AND d.used_in_training = FALSE
         ORDER BY d.content_length DESC
         LIMIT $2",
    )
    .bind(domain)
    .bind(limit)
    .fetch_all(pool)
    .await?)
}

pub async fn mark_training_eligible(pool: &PgPool, ids: &[Uuid]) -> Result<u64> {
    if ids.is_empty() {
        return Ok(0);
    }
    let r = sqlx::query("UPDATE documents SET training_eligible = TRUE WHERE id = ANY($1)")
        .bind(ids)
        .execute(pool)
        .await?;
    Ok(r.rows_affected())
}

pub async fn mark_used_in_training(pool: &PgPool, ids: &[Uuid]) -> Result<u64> {
    if ids.is_empty() {
        return Ok(0);
    }
    let r = sqlx::query("UPDATE documents SET used_in_training = TRUE WHERE id = ANY($1)")
        .bind(ids)
        .execute(pool)
        .await?;
    Ok(r.rows_affected())
}

pub async fn create_training_cycle(
    pool: &PgPool,
    domain: &str,
    base_model: &str,
    config: &JsonValue,
    dataset_size: i32,
) -> Result<Uuid> {
    let row: (Uuid,) = sqlx::query_as(
        "INSERT INTO training_cycles (domain, base_model, status, config, dataset_size)
         VALUES ($1, $2, 'running', $3, $4)
         RETURNING id",
    )
    .bind(domain)
    .bind(base_model)
    .bind(config)
    .bind(dataset_size)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

pub async fn complete_training_cycle(
    pool: &PgPool,
    cycle_id: Uuid,
    final_loss: Option<f32>,
) -> Result<()> {
    sqlx::query(
        "UPDATE training_cycles SET status = 'completed', final_loss = $2, completed_at = NOW() WHERE id = $1",
    )
    .bind(cycle_id).bind(final_loss).execute(pool).await?;
    Ok(())
}

pub async fn fail_training_cycle(pool: &PgPool, cycle_id: Uuid) -> Result<()> {
    sqlx::query("UPDATE training_cycles SET status = 'failed', completed_at = NOW() WHERE id = $1")
        .bind(cycle_id)
        .execute(pool)
        .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)] // Assinatura legada, mantida por compatibilidade.
pub async fn create_model(
    pool: &PgPool,
    name: &str,
    domain: &str,
    base_model: &str,
    dataset_size: i32,
    training_steps: i32,
    adapter_path: &str,
    adapter_checksum: &str,
    cycle_id: Uuid,
) -> Result<Uuid> {
    let row: (Uuid,) = sqlx::query_as(
        "INSERT INTO models
            (name, domain, base_model, status, dataset_size, training_steps,
             adapter_path, adapter_checksum, training_cycle_id)
         VALUES ($1, $2, $3, 'pending_approval', $4, $5, $6, $7, $8)
         RETURNING id",
    )
    .bind(name)
    .bind(domain)
    .bind(base_model)
    .bind(dataset_size)
    .bind(training_steps)
    .bind(adapter_path)
    .bind(adapter_checksum)
    .bind(cycle_id)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

pub async fn get_model(pool: &PgPool, model_id: Uuid) -> Result<ModelRow> {
    sqlx::query_as::<_, ModelRow>("SELECT * FROM models WHERE id = $1")
        .bind(model_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| crate::error::MtpError::ModelNotFound(model_id.to_string()))
}

pub async fn list_models_pending_approval(pool: &PgPool) -> Result<Vec<ModelRow>> {
    Ok(sqlx::query_as::<_, ModelRow>(
        "SELECT * FROM models WHERE status = 'pending_approval' ORDER BY created_at",
    )
    .fetch_all(pool)
    .await?)
}

pub async fn approve_model(pool: &PgPool, model_id: Uuid) -> Result<()> {
    sqlx::query("UPDATE models SET status = 'approved', approved_at = NOW() WHERE id = $1")
        .bind(model_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn reject_model(pool: &PgPool, model_id: Uuid) -> Result<()> {
    sqlx::query("UPDATE models SET status = 'archived' WHERE id = $1")
        .bind(model_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn deploy_model(pool: &PgPool, model_id: Uuid) -> Result<()> {
    sqlx::query("UPDATE models SET status = 'deployed', deployed_at = NOW() WHERE id = $1")
        .bind(model_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn archive_deployed_models(pool: &PgPool, domain: &str) -> Result<u64> {
    let r = sqlx::query(
        "UPDATE models SET status = 'archived' WHERE domain = $1 AND status = 'deployed'",
    )
    .bind(domain)
    .execute(pool)
    .await?;
    Ok(r.rows_affected())
}

pub async fn update_benchmark_score(pool: &PgPool, model_id: Uuid, score: f32) -> Result<()> {
    sqlx::query("UPDATE models SET benchmark_score = $2 WHERE id = $1")
        .bind(model_id)
        .bind(score)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn insert_lineage(pool: &PgPool, doc_ids: &[Uuid], cycle_id: Uuid) -> Result<()> {
    for doc_id in doc_ids {
        sqlx::query(
            "INSERT INTO document_training_lineage (document_id, cycle_id)
             VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(doc_id)
        .bind(cycle_id)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn domain_stats(pool: &PgPool) -> Result<Vec<DomainStats>> {
    Ok(sqlx::query_as::<_, DomainStats>(
        "SELECT
            d.domain,
            COUNT(*) FILTER (WHERE v.status = 'approved')    AS approved_docs,
            COUNT(*) FILTER (WHERE d.used_in_training = TRUE) AS used_in_training,
            (SELECT COUNT(*) FROM models m WHERE m.domain = d.domain) AS total_models
         FROM documents d
         JOIN validation v ON v.document_id = d.id
         GROUP BY d.domain
         ORDER BY d.domain",
    )
    .fetch_all(pool)
    .await?)
}

pub async fn domain_validation_stats(pool: &PgPool) -> Result<Vec<DomainValidationStats>> {
    Ok(sqlx::query_as::<_, DomainValidationStats>(
        "SELECT
            d.domain,
            COUNT(*) FILTER (WHERE v.status = 'pending')  AS pending_docs,
            COUNT(*) FILTER (WHERE v.status = 'approved') AS approved_docs,
            COUNT(*) FILTER (WHERE v.status = 'rejected') AS rejected_docs
         FROM documents d
         JOIN validation v ON v.document_id = d.id
         GROUP BY d.domain
         ORDER BY d.domain",
    )
    .fetch_all(pool)
    .await?)
}
pub async fn active_model_per_domain(pool: &PgPool) -> Result<Vec<(String, Option<String>)>> {
    Ok(
        sqlx::query_as("SELECT domain, name FROM models WHERE status = 'deployed' ORDER BY domain")
            .fetch_all(pool)
            .await?,
    )
}
