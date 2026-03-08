-- Migration 001: MTP Schema
BEGIN;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS training_eligible BOOLEAN DEFAULT FALSE;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS used_in_training  BOOLEAN DEFAULT FALSE;
CREATE TABLE IF NOT EXISTS training_cycles (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    domain       TEXT    NOT NULL,
    base_model   TEXT    NOT NULL,
    status       TEXT    NOT NULL DEFAULT 'running',
    config       JSONB   NOT NULL,
    dataset_size INTEGER,
    final_loss   REAL,
    started_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);
CREATE TABLE IF NOT EXISTS models (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name              TEXT NOT NULL,
    domain            TEXT NOT NULL,
    base_model        TEXT NOT NULL,
    status            TEXT NOT NULL DEFAULT 'training',
    dataset_size      INTEGER NOT NULL,
    training_steps    INTEGER,
    benchmark_score   REAL,
    adapter_checksum  TEXT,
    adapter_path      TEXT,
    training_cycle_id UUID REFERENCES training_cycles(id),
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    approved_at       TIMESTAMPTZ,
    deployed_at       TIMESTAMPTZ
);
CREATE TABLE IF NOT EXISTS document_training_lineage (
    document_id UUID NOT NULL REFERENCES documents(id),
    cycle_id    UUID NOT NULL REFERENCES training_cycles(id),
    PRIMARY KEY (document_id, cycle_id)
);
CREATE INDEX IF NOT EXISTS idx_documents_training_eligible ON documents(training_eligible);
CREATE INDEX IF NOT EXISTS idx_documents_domain            ON documents(domain);
CREATE INDEX IF NOT EXISTS idx_models_status               ON models(status);
CREATE INDEX IF NOT EXISTS idx_models_domain               ON models(domain);
CREATE INDEX IF NOT EXISTS idx_training_cycles_domain      ON training_cycles(domain);
COMMIT;
