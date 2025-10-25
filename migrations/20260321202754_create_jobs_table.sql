-- Add migration script here

CREATE TYPE job_status AS ENUM (
    'pending',
    'running',
    'succedded',
    'failed',
    'dead'
);

CREATE TABLE jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_type TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}',
    status job_status NOT NULL DEFAULT 'pending',
    priority SMALLINT NOT NULL DEFAULT 0,
    attempts SMALLINT NOT NULL DEFAULT 0,
    max_attempts SMALLINT NOT NULL DEFAULT 0,
    error TEXT,
    run_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_job_status_run_at ON jobs (status, run_at) WHERE status = 'pending'