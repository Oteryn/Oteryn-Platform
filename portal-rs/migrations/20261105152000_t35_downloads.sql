-- Signed publications are immutable; the channel pointer is the only mutable publication state.
CREATE TABLE client_channels (
    id uuid PRIMARY KEY,
    name text NOT NULL UNIQUE CHECK (name IN ('stable', 'beta')),
    active_publication_id uuid,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE client_releases (
    id uuid PRIMARY KEY,
    channel_id uuid NOT NULL REFERENCES client_channels(id),
    policy_revision bigint NOT NULL CHECK (policy_revision > 0),
    release_id text NOT NULL,
    release_sequence bigint NOT NULL CHECK (release_sequence > 0),
    version_display text NOT NULL,
    policy bytea NOT NULL CHECK (octet_length(policy) BETWEEN 1 AND 262144),
    policy_file text NOT NULL,
    metadata jsonb NOT NULL CHECK (jsonb_typeof(metadata) = 'object'),
    versions jsonb NOT NULL CHECK (jsonb_typeof(versions) = 'object'),
    artifacts jsonb NOT NULL CHECK (jsonb_typeof(artifacts) = 'object'),
    expires_at timestamptz NOT NULL,
    -- Account ownership is supplied by an adapter; there is deliberately no accounts foreign key.
    published_by uuid NOT NULL,
    previous_publication_id uuid,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (channel_id, policy_revision),
    UNIQUE (channel_id, id),
    FOREIGN KEY (channel_id, previous_publication_id) REFERENCES client_releases(channel_id, id)
);
ALTER TABLE client_channels ADD CONSTRAINT client_channels_active_publication_fk
    FOREIGN KEY (id, active_publication_id) REFERENCES client_releases(channel_id, id);

CREATE FUNCTION reject_client_release_mutation() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'client release publications are immutable';
END;
$$;
CREATE TRIGGER client_releases_immutable BEFORE UPDATE OR DELETE ON client_releases
    FOR EACH ROW EXECUTE FUNCTION reject_client_release_mutation();
