use std::collections::HashMap;

use nexus_rag::embedder::{Embedder, EMBEDDING_DIM};
use nexus_rag::indexer::collection_name;
use nexus_rag::query::run_query_with;
use qdrant_client::qdrant::{
    CreateCollectionBuilder, Distance, PointStruct, UpsertPointsBuilder, Value,
    VectorParamsBuilder, value::Kind,
};
use uuid::Uuid;

fn str_val(val: &str) -> Value {
    Value {
        kind: Some(Kind::StringValue(val.to_string())),
    }
}

fn int_val(val: i64) -> Value {
    Value {
        kind: Some(Kind::IntegerValue(val)),
    }
}

#[tokio::test]
#[ignore]
async fn integration_qdrant_end_to_end() -> Result<(), Box<dyn std::error::Error>> {
    // Scenario: real Qdrant instance with an indexed point.
    // Expectation: run_query_with returns at least one grounded result.
    if std::env::var("NEXUS_INTEGRATION_TESTS").ok().as_deref() != Some("1") {
        eprintln!("NEXUS_INTEGRATION_TESTS=1 not set; skipping.");
        return Ok(());
    }

    let url = std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6333".to_string());
    std::env::set_var("QDRANT_URL", &url);

    let client = nexus_rag::qdrant_builder::build_qdrant_client()?;
    let embedder = Embedder::new()?;

    let domain = format!("itest_{}", Uuid::new_v4());
    let collection = collection_name(&domain);

    let params = VectorParamsBuilder::new(EMBEDDING_DIM as u64, Distance::Cosine);
    client
        .create_collection(CreateCollectionBuilder::new(&collection).vectors_config(params))
        .await?;

    let vector = embedder.embed_one("integration test")?;
    let mut payload = HashMap::new();
    payload.insert("document_id".to_string(), str_val("doc-it"));
    payload.insert("source".to_string(), str_val("http://example.com"));
    payload.insert("domain".to_string(), str_val(&domain));
    payload.insert("doc_type".to_string(), str_val("html"));
    payload.insert("chunk_index".to_string(), int_val(0));
    payload.insert("chunk_total".to_string(), int_val(1));
    payload.insert("chunk_text".to_string(), str_val("integration evidence"));

    let point = PointStruct::new(Uuid::new_v4().to_string(), vector, payload);
    client
        .upsert_points(UpsertPointsBuilder::new(&collection, vec![point]))
        .await?;

    let results = run_query_with(&client, &embedder, "integration test", Some(&domain), 1).await?;
    assert!(!results.is_empty());

    let _ = client.delete_collection(&collection).await;
    Ok(())
}

