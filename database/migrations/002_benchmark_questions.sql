-- Migration 002: benchmark questions for nexus_mtp benchmark subcommand
BEGIN;

CREATE TABLE IF NOT EXISTS benchmark_questions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    domain TEXT NOT NULL CHECK (domain IN ('infra', 'rust', 'mlops', 'security')),
    question TEXT NOT NULL,
    expected_answer TEXT NOT NULL,
    expected_keywords TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[],
    source_document_id UUID REFERENCES documents(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by TEXT NOT NULL DEFAULT 'nexus_mtp'
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_benchmark_questions_domain_question
    ON benchmark_questions (domain, question);

CREATE INDEX IF NOT EXISTS idx_benchmark_questions_domain
    ON benchmark_questions (domain);

COMMIT;