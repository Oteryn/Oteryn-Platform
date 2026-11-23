ALTER TABLE player_reports ADD COLUMN verified_target_account_id uuid
    REFERENCES accounts(id) ON DELETE SET NULL;

-- Existing links were imported only after matching the reported UUID to the sanction.
UPDATE player_reports r
SET verified_target_account_id = s.account_id
FROM sanctions s
WHERE r.sanction_id = s.id;

CREATE INDEX player_reports_verified_target_idx
    ON player_reports (verified_target_account_id) WHERE verified_target_account_id IS NOT NULL;
