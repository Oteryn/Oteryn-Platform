-- Content-free, bounded (one row per affected family/subject) ambiguity fence.
-- No ingest, replay, epoch raise or retention sweep reconciles contradictory evidence.
-- An operator must reconcile authoritative evidence before removing a fence.
CREATE TABLE public_projection_conflicts (
    family text NOT NULL CHECK (length(family) BETWEEN 1 AND 128),
    subject text NOT NULL CHECK (length(subject) <= 1024),
    epoch numeric(20,0) NOT NULL,
    revision numeric(20,0) NOT NULL,
    PRIMARY KEY (family, subject)
);
CREATE FUNCTION public_projection_conflicted(f text, s text DEFAULT NULL)
RETURNS boolean LANGUAGE sql STABLE AS $$
    SELECT EXISTS (SELECT 1 FROM public_projection_conflicts
                   WHERE family=f AND (s IS NULL OR subject=s))
$$;

-- G-9 admissions are provisioned only after authoritative restore/baseline and
-- catalogue reconciliation. Receiving a random generation never admits it.
CREATE TABLE pd_g9_streams (
 source_authority text NOT NULL,
 account_id uuid NOT NULL,
 stream_generation uuid NOT NULL,
 admitted boolean NOT NULL DEFAULT false,
 recovery_generation numeric(20,0) NOT NULL DEFAULT 1,
 blocked boolean NOT NULL DEFAULT false,
 conflicted boolean NOT NULL DEFAULT false,
 highest_seen numeric(20,0) NOT NULL DEFAULT 0,
 stored_through numeric(20,0) NOT NULL DEFAULT 0,
 applied_through numeric(20,0) NOT NULL DEFAULT 0,
 PRIMARY KEY(source_authority,account_id,stream_generation)
);
CREATE UNIQUE INDEX pd_g9_current_generation ON pd_g9_streams(source_authority,account_id) WHERE admitted;
CREATE TABLE pd_g9_catalogue_keys (
 source_authority text NOT NULL,
 catalog_revision text NOT NULL,
 catalog_key text NOT NULL,
 PRIMARY KEY(source_authority,catalog_revision,catalog_key)
);
CREATE TABLE pd_g9_records (
 source_authority text NOT NULL,
 account_id uuid NOT NULL,
 stream_generation uuid NOT NULL,
 source_revision numeric(20,0) NOT NULL,
 record_kind text NOT NULL CHECK(record_kind IN ('HUNT_SEGMENT','RARE_DROP')),
 session_id uuid NOT NULL,
 segment_id uuid NOT NULL,
 business_id uuid NOT NULL,
 segment_index integer,
 character_id uuid NOT NULL,
 digest bytea NOT NULL CHECK(octet_length(digest)=32),
 canonical_bytes bytea,
 record jsonb,
 resolved boolean NOT NULL DEFAULT false,
 expires_at bigint NOT NULL,
 purged_at bigint,
 PRIMARY KEY(source_authority,account_id,stream_generation,source_revision),
 UNIQUE(source_authority,record_kind,business_id)
);
CREATE UNIQUE INDEX pd_g9_segment_index ON pd_g9_records(source_authority,session_id,segment_index) WHERE record_kind='HUNT_SEGMENT';
CREATE TABLE pd_g9_commands (
 operation_id uuid PRIMARY KEY,
 source_authority text NOT NULL,
 source_revision numeric(20,0) NOT NULL,
 account_id uuid NOT NULL,
 stream_generation uuid NOT NULL,
 intent bytea NOT NULL,
 binding_digest text NOT NULL CHECK(length(binding_digest)=64),
 receipt bytea,
 UNIQUE(source_authority,source_revision)
);
