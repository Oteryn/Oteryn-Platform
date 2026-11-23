CREATE INDEX sanctions_account_history ON sanctions (account_id, created_at DESC, id DESC)
    WHERE status <> 'pending';
