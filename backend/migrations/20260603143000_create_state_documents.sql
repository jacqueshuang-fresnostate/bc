CREATE TABLE IF NOT EXISTS state_documents (
    namespace text PRIMARY KEY,
    payload jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
