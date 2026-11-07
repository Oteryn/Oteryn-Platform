ALTER TABLE wallet_operations DROP CONSTRAINT wallet_operations_kind_check;
ALTER TABLE wallet_operations ADD CONSTRAINT wallet_operations_kind_check
    CHECK (kind IN ('transfer', 'adjustment', 'reserve', 'release', 'capture', 'store_purchase', 'top_up'));
