-- no-transaction
CREATE INDEX CONCURRENTLY accounts_operations_created_at ON accounts (created_at);
