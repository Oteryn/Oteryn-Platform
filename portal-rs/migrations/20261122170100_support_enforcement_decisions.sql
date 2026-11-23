ALTER TABLE sanctions
    ADD COLUMN enforcement_profile text CHECK (enforcement_profile IN ('MUTE', 'JAIL')),
    ADD COLUMN enforcement_character_id uuid,
    ADD CHECK ((enforcement_profile IS NULL AND enforcement_character_id IS NULL)
        OR (enforcement_profile IS NOT NULL AND enforcement_character_id IS NOT NULL
            AND category='restriction' AND expires_at IS NOT NULL));

-- GM decisions publish explicitly with the same revision as their command.
-- Keep the legacy export for all existing sanction categories and rows.
DROP TRIGGER support_sanction_export_legacy ON sanctions;
CREATE TRIGGER support_sanction_export_legacy AFTER INSERT OR UPDATE ON sanctions
    FOR EACH ROW WHEN (NEW.enforcement_profile IS NULL)
    EXECUTE FUNCTION support_sanction_export_legacy();

-- Typed history for character restrictions. The existing sanctions row is the
-- Platform-facing decision and appeal identity. Commands and N009 share revision.
CREATE TABLE support_enforcement_decisions (
    sanction_id uuid NOT NULL REFERENCES sanctions(id),
    decision_revision bigint NOT NULL CHECK (decision_revision > 0),
    operation_id uuid NOT NULL UNIQUE,
    profile text NOT NULL CHECK (profile IN ('MUTE', 'JAIL')),
    character_id uuid NOT NULL,
    intent text NOT NULL CHECK (intent IN ('APPLY', 'REPLACE', 'REVOKE')),
    reason_code integer NOT NULL CHECK (reason_code BETWEEN 1 AND 65535),
    decided_at_ms bigint NOT NULL CHECK (decided_at_ms > 0),
    expires_at_ms bigint CHECK (expires_at_ms > decided_at_ms),
    reconcile_at_ms bigint,
    PRIMARY KEY (sanction_id, decision_revision),
    CHECK ((intent IN ('APPLY', 'REPLACE')) = (expires_at_ms IS NOT NULL)),
    CHECK (reconcile_at_ms IS NULL OR
        (expires_at_ms IS NOT NULL AND reconcile_at_ms = expires_at_ms))
);
CREATE INDEX support_enforcement_expiry_due
    ON support_enforcement_decisions(reconcile_at_ms, sanction_id)
    WHERE reconcile_at_ms IS NOT NULL;
CREATE INDEX support_enforcement_commands_target
    ON support_enforcement_commands(account_id, character_id, profile, position DESC);
CREATE INDEX account_characters_character_owner ON account_characters(character_id, account_id);
