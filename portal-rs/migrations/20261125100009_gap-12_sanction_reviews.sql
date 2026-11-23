-- Platform review targets and grants have no authority to change Game state.
CREATE TABLE admin_sanction_review_targets (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    world_id uuid NOT NULL REFERENCES world_status(id),
    account_id uuid NOT NULL REFERENCES accounts(id),
    character_id uuid NOT NULL CHECK (uuid_extract_version(character_id) IS NOT DISTINCT FROM 7),
    UNIQUE (world_id, account_id, character_id)
);

CREATE TABLE admin_sanction_review_grants (
    staff_id uuid NOT NULL REFERENCES accounts(id),
    target_id uuid NOT NULL REFERENCES admin_sanction_review_targets(id),
    PRIMARY KEY (staff_id, target_id)
);

CREATE TABLE admin_sanction_reviews (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    target_id uuid NOT NULL REFERENCES admin_sanction_review_targets(id),
    report_id uuid NOT NULL REFERENCES player_reports(id),
    kind text NOT NULL CHECK (kind IN ('ban', 'mute', 'jail')),
    reason text NOT NULL CHECK (reason IN ('conduct', 'harassment', 'cheating', 'other')),
    expires_at timestamptz,
    requested_by uuid NOT NULL REFERENCES accounts(id),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    apply_approved_by uuid REFERENCES accounts(id),
    apply_approved_at timestamptz,
    lift_requested_by uuid REFERENCES accounts(id),
    lift_requested_at timestamptz,
    lift_reason text CHECK (lift_reason IN ('appeal', 'correction', 'review')),
    lift_approved_by uuid REFERENCES accounts(id),
    lift_approved_at timestamptz,
    UNIQUE (target_id, report_id, kind),
    CHECK (expires_at IS NOT NULL OR kind = 'ban'),
    CHECK ((apply_approved_by IS NULL) = (apply_approved_at IS NULL)),
    CHECK ((lift_requested_by IS NULL) = (lift_requested_at IS NULL)),
    CHECK ((lift_requested_by IS NULL) = (lift_reason IS NULL)),
    CHECK ((lift_approved_by IS NULL) = (lift_approved_at IS NULL)),
    CHECK (apply_approved_by IS DISTINCT FROM requested_by),
    CHECK (lift_requested_by IS NULL OR apply_approved_by IS NOT NULL),
    CHECK (lift_approved_by IS NULL OR
        (lift_requested_by IS NOT NULL AND lift_approved_by <> lift_requested_by))
);

CREATE INDEX admin_sanction_reviews_target_recent
    ON admin_sanction_reviews (target_id, id DESC);
CREATE INDEX admin_sanction_reviews_actor_recent
    ON admin_sanction_reviews (requested_by, created_at DESC);

-- An approved review always refers to the same subject and world.
CREATE FUNCTION admin_sanction_review_target_immutable() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'sanction review target is immutable' USING ERRCODE = '23514';
END;
$$;

CREATE TRIGGER admin_sanction_review_target_immutable
    BEFORE UPDATE ON admin_sanction_review_targets
    FOR EACH ROW EXECUTE FUNCTION admin_sanction_review_target_immutable();
