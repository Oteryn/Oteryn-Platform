-- Native game evidence (crates/gameauth) reads a positive per-account security
-- generation and fails closed without it. Additive: existing rows start at 1.
ALTER TABLE accounts
    ADD COLUMN native_security_generation bigint NOT NULL DEFAULT 1
        CHECK (native_security_generation > 0);
