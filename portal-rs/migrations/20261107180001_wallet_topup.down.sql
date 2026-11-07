-- Refuse loss of the classification while immutable financial history exists.
LOCK TABLE wallet_operations IN ACCESS EXCLUSIVE MODE;
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM wallet_operations WHERE kind='top_up') THEN
        RAISE EXCEPTION 'cannot roll back wallet top_up while top_up rows exist';
    END IF;
END $$;
ALTER TABLE wallet_operations DROP CONSTRAINT wallet_operations_kind_check;
ALTER TABLE wallet_operations ADD CONSTRAINT wallet_operations_kind_check
    CHECK (kind IN ('transfer', 'adjustment', 'reserve', 'release', 'capture', 'store_purchase'));
