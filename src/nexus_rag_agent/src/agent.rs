// SPDX-License-Identifier: Apache-2.0

use nexus_rag::embedder::Embedder;
use nexus_rag::qdrant_builder;
use qdrant_client::Qdrant;
use sqlx::PgPool;

use crate::prompts;
use crate::Result;

pub struct RAGAgent {
    pub embedder: Embedder,
    pub qdrant: Qdrant,
    pub pool: PgPool,
}

impl RAGAgent {
    pub async fn new() -> Result<Self> {
        let embedder = Embedder::new()?;
        let qdrant = qdrant_builder::build_qdrant_client()?;
        let pool = nexus_rag::db::connect().await?;
        Ok(Self {
            embedder,
            qdrant,
            pool,
        })
    }

    pub fn build_system_prompt(&self) -> String {
        prompts::system_prompt()
    }
}
