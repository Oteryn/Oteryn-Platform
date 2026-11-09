-- Existing entries remain transferable. Preserve all immutable financial history.
ALTER TABLE wallet_entries ADD COLUMN coin_class text NOT NULL DEFAULT 'transferable'
    CHECK (coin_class IN ('transferable','non_transferable'));
ALTER TABLE wallet_entries DROP CONSTRAINT wallet_entries_operation_id_account_id_key;
ALTER TABLE wallet_entries ADD CONSTRAINT wallet_entries_operation_account_class_key
    UNIQUE (operation_id,account_id,coin_class);
ALTER TABLE wallet_entries ADD CONSTRAINT wallet_non_transferable_not_reserved
    CHECK (coin_class='transferable' OR reserved_delta=0);
ALTER TABLE wallet_reservations ADD COLUMN consumed bigint NOT NULL DEFAULT 0
    CHECK (consumed>=0 AND consumed<=amount);
ALTER TABLE wallet_operations DROP CONSTRAINT wallet_operations_kind_check;
ALTER TABLE wallet_operations ADD CONSTRAINT wallet_operations_kind_check
    CHECK (kind IN ('transfer','adjustment','reserve','release','capture','store_purchase','top_up','reward','staff_grant','settlement'));
-- Rollback is application rollback with flags off; do not discard ledger classes.
