-- Coin debt settlement (owner decision 2026-11-14). A held purchase reversal
-- (outcome 'held', coins - debited) is coin debt. Every later coin credit to
-- the account pays it first, oldest debt first, in the credit's transaction
-- under the wallet account lock; only the remainder stays on the balance.
-- Additive: one widened operation kind, one ledger table, one view and indexes.
-- No existing row changes; the reversal rows stay immutable.

-- The repayment of one credit is its own wallet operation ('debt_settlement',
-- reference_id = the credit operation) with one negative entry, so the
-- account's history shows the full credit and the repayment.
ALTER TABLE wallet_operations DROP CONSTRAINT wallet_operations_kind_check;
ALTER TABLE wallet_operations ADD CONSTRAINT wallet_operations_kind_check
    CHECK (kind IN ('transfer','adjustment','reserve','release','capture','store_purchase','top_up','reward','staff_grant','settlement','debt_settlement'));
CREATE INDEX wallet_operations_debt_settlement ON wallet_operations (reference_id)
    WHERE kind = 'debt_settlement';

-- One row per debt paid by one credit: the debt record, the coins applied and
-- the credit operation. Immutable like the rest of the ledger. reversal_id has
-- no foreign key on purpose: the wallet ledger never references payments
-- tables (plan 3.2), so no cascade from them can reach these rows. The row is
-- written in the credit's transaction while that reversal row is locked.
CREATE TABLE coin_debt_settlements (
    id uuid PRIMARY KEY,
    reversal_id uuid NOT NULL,
    account_id uuid NOT NULL CHECK (account_id <> '00000000-0000-0000-0000-000000000000'),
    credit_operation_id uuid NOT NULL REFERENCES wallet_operations(id),
    settlement_operation_id uuid NOT NULL REFERENCES wallet_operations(id),
    coins bigint NOT NULL CHECK (coins > 0),
    owed_before bigint NOT NULL CHECK (owed_before >= coins),
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (reversal_id, credit_operation_id)
);
CREATE INDEX coin_debt_settlements_account ON coin_debt_settlements (account_id, created_at);
CREATE INDEX coin_debt_settlements_credit ON coin_debt_settlements (credit_operation_id);
CREATE INDEX coin_debt_settlements_settlement ON coin_debt_settlements (settlement_operation_id);
CREATE TRIGGER coin_debt_settlements_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
    ON coin_debt_settlements FOR EACH STATEMENT EXECUTE FUNCTION wallet_reject_rewrite();

-- The one definition of what is still owed per held reversal.
CREATE VIEW coin_outstanding_debts AS
    SELECT r.id AS reversal_id,
           r.order_id,
           r.payer_account_id AS account_id,
           r.created_at,
           r.coins - r.debited - COALESCE(
               (SELECT SUM(s.coins) FROM coin_debt_settlements s WHERE s.reversal_id = r.id), 0
           )::bigint AS owed
    FROM coin_purchase_reversals r
    WHERE r.outcome = 'held';
-- Rollback is application rollback with flags off; do not discard settlement rows.
