-- Durable promises for coin payouts before an irreversible Game handoff.
-- Wallet row locks serialize credits with reservation creation/consumption.
CREATE TABLE wallet_incoming_capacity (
    id uuid PRIMARY KEY,
    account_id uuid NOT NULL,
    amount bigint NOT NULL CHECK(amount > 0)
);
CREATE INDEX wallet_incoming_capacity_account ON wallet_incoming_capacity(account_id);
-- Preserve obligations already frozen by the previous runtime. Overcommitted
-- historical wallets refuse competing credits until their obligations resolve.
-- The assigned 13-digit migration version precedes the historical 14-digit
-- versions on a fresh database. No FK requires those later tables; upgrades
-- backfill existing promises, while fresh installations have none to backfill.
DO $$ BEGIN
    IF to_regclass('bazaar_coin_dispositions') IS NOT NULL THEN
        INSERT INTO wallet_incoming_capacity(id,account_id,amount)
        SELECT s.id,s.seller,s.seller_net FROM bazaar_settlements s
        JOIN bazaar_coin_dispositions d ON d.settlement_id=s.id
        WHERE NOT d.completed AND s.currency='coins' AND s.seller_net>0;
    END IF;
END $$;
