CREATE INDEX sanctions_owner_history
    ON sanctions (account_id, created_at DESC, id DESC)
    WHERE status <> 'pending' AND (expires_at IS NOT NULL OR approved_by IS NOT NULL);
