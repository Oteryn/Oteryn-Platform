-- A public, one-use server challenge; old unbound families must sign in again.
ALTER TABLE device_sessions ADD COLUMN rotation_nonce text
    CHECK (rotation_nonce IS NULL OR rotation_nonce ~ '^[A-Za-z0-9_-]{43}$');
