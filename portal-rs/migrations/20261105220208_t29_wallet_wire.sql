-- Preserve all existing ledger kinds and rows. Existing UUID cursors remain valid.
ALTER TABLE wallet_operations DROP CONSTRAINT wallet_operations_kind_check;
ALTER TABLE wallet_operations ADD CONSTRAINT wallet_operations_kind_check
    CHECK (kind IN ('transfer', 'adjustment', 'reserve', 'release', 'capture', 'store_purchase'));
ALTER TABLE wallet_entries ADD COLUMN entry_sequence bigint GENERATED ALWAYS AS IDENTITY;
CREATE UNIQUE INDEX wallet_entries_sequence_unique ON wallet_entries(entry_sequence);
CREATE INDEX wallet_entries_numeric_history ON wallet_entries(account_id, entry_sequence DESC);
