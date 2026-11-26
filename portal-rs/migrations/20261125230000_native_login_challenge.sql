-- OTERYN_NATIVE_LOGIN_CHALLENGE_V1 (Oteryn-Game 42496a66): Platform's side,
-- behind PORTAL_NATIVE_LOGIN_CHALLENGE (default off). Additive only.

-- Sub-window counters shared by every replica. Keys are SHA-256 digests of a
-- normalized email or a client address, never the value itself; rows are
-- pruned after the configured retention (default 24 hours) by the worker's
-- cleanup.native-login-challenge job and beside every counter write.
CREATE TABLE native_login_challenge_counters (
    kind text NOT NULL CHECK (kind IN ('identity_failure', 'source_attempt')),
    key_hash bytea NOT NULL CHECK (octet_length(key_hash) = 32),
    bucket_start timestamptz NOT NULL,
    hits integer NOT NULL CHECK (hits > 0),
    PRIMARY KEY (kind, key_hash, bucket_start)
);
CREATE INDEX native_login_challenge_counters_bucket
    ON native_login_challenge_counters(bucket_start);

-- One challenge per pending authorization code, bound to the whole flow. The
-- code row itself is never changed by issuing a challenge.
CREATE TABLE native_login_challenges (
    id uuid PRIMARY KEY,
    code_hash bytea NOT NULL UNIQUE REFERENCES oauth_authorization_codes(code_hash),
    account_id uuid NOT NULL REFERENCES accounts(id),
    client_id text NOT NULL,
    code_challenge text NOT NULL,
    redirect_uri text NOT NULL,
    credential_hash bytea NOT NULL,
    policy_version integer NOT NULL CHECK (policy_version > 0),
    reason text NOT NULL CHECK (reason IN ('identity', 'source')),
    expires_at timestamptz NOT NULL,
    -- The six digits the page shows; a digest of six digits hides nothing.
    answer text CHECK (answer IS NULL OR answer ~ '^[0-9]{6}$'),
    answer_attempts integer NOT NULL DEFAULT 0 CHECK (answer_attempts >= 0),
    return_uri text,
    return_state_hash bytea CHECK (return_state_hash IS NULL OR octet_length(return_state_hash) = 32),
    completed_at timestamptz,
    proof_nonce_hash bytea UNIQUE CHECK (proof_nonce_hash IS NULL OR octet_length(proof_nonce_hash) = 32),
    consumed_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK ((return_uri IS NULL) = (return_state_hash IS NULL)),
    CHECK (consumed_at IS NULL OR (completed_at IS NOT NULL AND proof_nonce_hash IS NOT NULL))
);
CREATE INDEX native_login_challenges_account ON native_login_challenges(account_id);
CREATE INDEX native_login_challenges_expires ON native_login_challenges(expires_at);
