-- Existing request_id and actor/created_at indexes remain authoritative.
-- Preserve the legacy required text target for existing audit writers.
ALTER TABLE audit_log
    ADD COLUMN admin_operation_id uuid,
    ADD COLUMN operation text,
    ADD COLUMN risk_class text CHECK (risk_class IN ('R0', 'R1', 'R2', 'R3', 'R4')),
    ADD COLUMN admin_control_target jsonb,
    ADD COLUMN reason jsonb,
    ADD COLUMN approvals jsonb,
    ADD COLUMN ip text,
    ADD COLUMN user_agent text,
    ADD COLUMN correlation_id uuid,
    ADD COLUMN outcome text,
    ADD COLUMN game_ledger_id text;
COMMENT ON COLUMN audit_log.admin_control_target IS
    'ADM-01 target object; target retains its legacy text contract.';
