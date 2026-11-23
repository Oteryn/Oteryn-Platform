-- Order history resolves delivery operations by fulfilment unit.
CREATE INDEX store_operations_entitlement_revision
    ON store_operations (entitlement_id, source_revision DESC);

-- Staff operation lookups also show the latest restore that reopened the operation.
CREATE INDEX store_reconcile_operation_generation
    ON store_reconcile_state (operation_id, new_recovery_generation DESC);
