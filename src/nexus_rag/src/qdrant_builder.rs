//! Centralised Qdrant client construction.
//!
//! Reads QDRANT_URL (required) and QDRANT_API_KEY (optional).
//! In NEXUS_ENV=production, rejects plain http:// URLs.
//!
//! Thread-safety: `Qdrant` is Clone + Send + Sync.

use crate::error::{NexusError, Result};
use qdrant_client::Qdrant;

/// Builds and returns an authenticated Qdrant client.
pub fn build_qdrant_client() -> Result<Qdrant> {
    let url =
        std::env::var("QDRANT_URL").map_err(|_| NexusError::EnvVar("QDRANT_URL".to_string()))?;

    let is_production = std::env::var("NEXUS_ENV").as_deref() == Ok("production");
    if is_production && url.starts_with("http://") {
        return Err(NexusError::Config(
            "QDRANT_URL deve usar https:// ou grpcs:// em produção (NEXUS_ENV=production)"
                .to_string(),
        ));
    }

    let mut builder = Qdrant::from_url(&url);

    if let Ok(key) = std::env::var("QDRANT_API_KEY") && !key.is_empty() {
        builder = builder.api_key(key);
    }

    let client = builder
        .build()
        .map_err(|e| NexusError::Qdrant(format!("Failed to build Qdrant client: {e}")))?;

    tracing::info!(url = %url, "Qdrant client initialised");
    Ok(client)
}
