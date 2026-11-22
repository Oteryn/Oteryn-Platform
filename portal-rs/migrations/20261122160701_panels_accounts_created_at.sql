-- no-transaction
CREATE INDEX CONCURRENTLY IF NOT EXISTS accounts_operations_created_at ON accounts (created_at);
