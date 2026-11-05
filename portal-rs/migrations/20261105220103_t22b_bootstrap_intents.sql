-- Dedicated issuer namespace, independent of LCFA and account-security revisions.
-- This read interface does not allocate revisions or issue intents.
CREATE TABLE character_bootstrap_intent_authority (
    id smallint PRIMARY KEY CHECK (id = 1),
    last_source_revision text NOT NULL,
    last_issuer_decision_id text
);
CREATE TABLE character_bootstrap_intents (
    operation_id text PRIMARY KEY,
    issuer_decision_id text NOT NULL,
    source_revision text NOT NULL,
    account_id text NOT NULL,
    target_world_id text NOT NULL,
    requested_name text NOT NULL,
    requested_sex text,
    profile_revision text NOT NULL,
    ruleset_revision text NOT NULL,
    content_revision text NOT NULL,
    starter_template_revision text NOT NULL,
    issued_at_source text NOT NULL,
    expires_at_source text NOT NULL,
    intent_json text NOT NULL CHECK (octet_length(intent_json) <= 4096),
    intent_sha256 text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
