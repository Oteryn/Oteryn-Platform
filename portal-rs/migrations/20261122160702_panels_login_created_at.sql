-- no-transaction
CREATE INDEX CONCURRENTLY security_events_operations_login_created_at ON security_events (created_at)
    WHERE kind = 'login_succeeded';
