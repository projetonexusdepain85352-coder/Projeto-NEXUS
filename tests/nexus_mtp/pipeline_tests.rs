use std::fs;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use nexus_mtp::benchmark::benchmark_passed;
use nexus_mtp::dataset::{extract_with_store, DatasetStore};
use nexus_mtp::db::{ApprovedDocument, ModelRow};
use nexus_mtp::error::{MtpError, Result};
use nexus_mtp::main_impl::ensure_model_deployable;
use nexus_mtp::trainer::training_env_overrides;
use tempfile::tempdir;
use uuid::Uuid;

#[derive(Clone)]
struct MockStore {
    docs: Vec<ApprovedDocument>,
    marked: Arc<Mutex<Vec<Uuid>>>,
}

#[async_trait]
impl DatasetStore for MockStore {
    async fn fetch_approved_documents(
        &self,
        _domain: &str,
        _max_samples: i64,
    ) -> Result<Vec<ApprovedDocument>> {
        Ok(self.docs.clone())
    }

    async fn mark_training_eligible(&self, ids: &[Uuid]) -> Result<u64> {
        let mut guard = self.marked.lock().expect("lock marked ids");
        guard.extend(ids.iter().cloned());
        Ok(ids.len() as u64)
    }
}

fn sample_model(status: &str, score: Option<f32>) -> ModelRow {
    ModelRow {
        id: Uuid::new_v4(),
        name: "model".to_string(),
        domain: "rust".to_string(),
        base_model: "base".to_string(),
        status: status.to_string(),
        dataset_size: 10,
        training_steps: Some(100),
        benchmark_score: score,
        adapter_checksum: Some("chk".to_string()),
        adapter_path: Some("adapter".to_string()),
        training_cycle_id: None,
        created_at: Utc::now(),
        approved_at: None,
        deployed_at: None,
    }
}

#[tokio::test]
async fn dataset_extracts_jsonl_from_mock_store() {
    // Scenario: mock store returns approved documents for dataset extraction.
    // Expectation: JSONL file is created and training eligible IDs are recorded.
    let doc1 = ApprovedDocument {
        id: Uuid::new_v4(),
        content: "conteudo tecnico relevante".repeat(50),
        source: "http://example.com/1".to_string(),
        domain: "rust".to_string(),
    };
    let doc2 = ApprovedDocument {
        id: Uuid::new_v4(),
        content: "outro conteudo tecnico".repeat(40),
        source: "http://example.com/2".to_string(),
        domain: "rust".to_string(),
    };

    let marked = Arc::new(Mutex::new(Vec::new()));
    let store = MockStore {
        docs: vec![doc1.clone(), doc2.clone()],
        marked: Arc::clone(&marked),
    };

    let dir = tempdir().expect("temp dir");
    let (path, ids, total) = extract_with_store(
        &store,
        "rust",
        100,
        dir.path().to_string_lossy().as_ref(),
    )
    .await
    .expect("dataset extract should succeed");

    assert!(path.exists());
    assert_eq!(ids.len(), 2);
    assert!(total > 0);

    let contents = fs::read_to_string(&path).expect("read jsonl");
    let lines: Vec<&str> = contents.lines().collect();
    assert!(!lines.is_empty());

    let first: serde_json::Value = serde_json::from_str(lines[0]).expect("valid json");
    assert!(first.get("instruction").is_some());
    assert!(first.get("input").is_some());
    assert!(first.get("output").is_some());
    assert!(first.get("source").is_some());

    let marked_ids = marked.lock().expect("lock");
    assert_eq!(marked_ids.len(), 2);
}

#[test]
fn benchmark_pass_fail_logic() {
    // Scenario: evaluate benchmark scores against a threshold.
    // Expectation: scores above threshold pass; below threshold fail.
    assert!(benchmark_passed(0.8, 0.7));
    assert!(!benchmark_passed(0.6, 0.7));
}

#[test]
fn deploy_requires_approved_benchmark() {
    // Scenario: deploy is requested for models in different benchmark states.
    // Expectation: only approved models above min score can deploy.
    let min_score = 0.7;

    let err = ensure_model_deployable(&sample_model("training", Some(0.9)), min_score)
        .expect_err("non-approved status should fail");
    assert!(matches!(err, MtpError::NotApproved(_)));

    let err = ensure_model_deployable(&sample_model("approved", None), min_score)
        .expect_err("missing benchmark should fail");
    assert!(matches!(err, MtpError::BenchmarkMissing));

    let err = ensure_model_deployable(&sample_model("approved", Some(0.4)), min_score)
        .expect_err("low benchmark should fail");
    assert!(matches!(err, MtpError::BenchmarkBelowThreshold { .. }));

    ensure_model_deployable(&sample_model("approved", Some(0.9)), min_score)
        .expect("approved benchmark should pass");
}

#[test]
fn ice_workaround_env_vars_present() {
    // Scenario: training command is prepared with ICE workaround flags.
    // Expectation: TORCHINDUCTOR_DISABLE and TORCH_COMPILE_DISABLE are set to 1.
    let envs = training_env_overrides();
    assert!(envs.iter().any(|(k, v)| *k == "TORCHINDUCTOR_DISABLE" && *v == "1"));
    assert!(envs.iter().any(|(k, v)| *k == "TORCH_COMPILE_DISABLE" && *v == "1"));
}
