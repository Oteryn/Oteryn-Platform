-- Immutable evidence binds an approval to one verified dry-run result and intent.
CREATE TABLE admin_control_previews (
    id uuid PRIMARY KEY,
    requester uuid NOT NULL,
    operation text NOT NULL,
    permission text NOT NULL,
    intent_digest text NOT NULL CHECK (intent_digest ~ '^[0-9a-f]{64}$'),
    preview_digest text NOT NULL CHECK (preview_digest ~ '^[0-9a-f]{64}$'),
    preview jsonb NOT NULL,
    reason jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE admin_control_approvals (
    id uuid PRIMARY KEY,
    preview_id uuid NOT NULL UNIQUE REFERENCES admin_control_previews(id),
    admin_id uuid NOT NULL,
    step_up_at bigint NOT NULL,
    approved_at bigint NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE admin_control_step_up_attempts (
    account_id uuid PRIMARY KEY,
    window_start bigint NOT NULL,
    attempts integer NOT NULL CHECK (attempts > 0),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE FUNCTION check_admin_control_approval() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.admin_id = (SELECT requester FROM admin_control_previews WHERE id=NEW.preview_id)
       OR NEW.step_up_at > NEW.approved_at
       OR NEW.approved_at - NEW.step_up_at >= 300 THEN
        RAISE EXCEPTION 'invalid control approval' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER admin_control_approval_check BEFORE INSERT ON admin_control_approvals
    FOR EACH ROW EXECUTE FUNCTION check_admin_control_approval();
CREATE TRIGGER admin_control_previews_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON admin_control_previews
    FOR EACH STATEMENT EXECUTE FUNCTION reject_audit_log_mutation();
CREATE TRIGGER admin_control_approvals_immutable BEFORE UPDATE OR DELETE OR TRUNCATE ON admin_control_approvals
    FOR EACH STATEMENT EXECUTE FUNCTION reject_audit_log_mutation();
