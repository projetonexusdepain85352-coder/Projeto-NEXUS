use std::path::PathBuf;

use nexus_mtp::trainer::{run_training, TrainJob};
use tempfile::tempdir;

#[test]
#[ignore]
fn integration_training_pipeline() {
    // Scenario: full training pipeline with real model and dataset.
    // Expectation: training completes successfully when integration env is configured.
    if std::env::var("NEXUS_INTEGRATION_TESTS").ok().as_deref() != Some("1") {
        eprintln!("NEXUS_INTEGRATION_TESTS=1 not set; skipping.");
        return;
    }

    let dataset_path = match std::env::var("NEXUS_TRAIN_DATASET") {
        Ok(v) => PathBuf::from(v),
        Err(_) => {
            eprintln!("NEXUS_TRAIN_DATASET not set; skipping.");
            return;
        }
    };

    let base_model = std::env::var("NEXUS_TRAIN_BASE_MODEL")
        .unwrap_or_else(|_| "mistralai/Mistral-7B-Instruct-v0.3".to_string());

    let output_dir = tempdir().expect("temp output");
    let adapter_dir = tempdir().expect("temp adapter");
    let models_dir = tempdir().expect("temp models");

    let job = TrainJob {
        base_model,
        dataset_path,
        domain: "rust".to_string(),
        epochs: 1,
        lora_r: 8,
        lora_alpha: 16,
        max_seq_len: 512,
        learning_rate: 2e-4,
        output_dir: output_dir.path().to_path_buf(),
        adapter_path: adapter_dir.path().to_path_buf(),
        models_dir: models_dir.path().to_path_buf(),
    };

    let result = run_training(&job);
    assert!(result.is_ok());
}
