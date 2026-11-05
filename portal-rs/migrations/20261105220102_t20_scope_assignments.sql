-- Epoch lock serializes ownership changes with runtime admission readers.
CREATE TABLE native_assignment_epoch_locks (id smallint PRIMARY KEY CHECK (id = 1));
INSERT INTO native_assignment_epoch_locks VALUES (1);
CREATE TABLE native_scope_assignments (
    world_id uuid NOT NULL,
    channel_id uuid NOT NULL CHECK (channel_id <> world_id),
    assignment_epoch numeric(20,0) NOT NULL CHECK (assignment_epoch BETWEEN 1 AND 18446744073709551615),
    ownership_generation numeric(20,0) NOT NULL CHECK (ownership_generation BETWEEN 1 AND 18446744073709551615),
    node_identity text,
    assigned_at bigint CHECK (assigned_at >= 0),
    revoked_at bigint CHECK (revoked_at >= 0),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY(world_id, channel_id),
    CHECK ((node_identity IS NOT NULL AND assigned_at IS NOT NULL AND revoked_at IS NULL)
        OR (node_identity IS NULL AND assigned_at IS NULL AND revoked_at IS NOT NULL))
);
