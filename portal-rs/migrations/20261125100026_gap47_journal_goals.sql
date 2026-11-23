CREATE TABLE pd_journal_goals (
    id uuid NOT NULL,
    account_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    kind text NOT NULL CHECK (kind IN ('level','skill','bestiary','quest','boss','stamina')),
    title text NOT NULL CHECK (char_length(title) BETWEEN 1 AND 120),
    remind_on date,
    completed_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, id)
);

CREATE INDEX pd_journal_goals_account ON pd_journal_goals(account_id, created_at, id);

CREATE FUNCTION pd_remove_closed_account_goals() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    DELETE FROM pd_journal_goals WHERE account_id = NEW.id;
    RETURN NEW;
END;
$$;

CREATE TRIGGER pd_closed_account_goals AFTER UPDATE OF status ON accounts
    FOR EACH ROW WHEN (OLD.status IS DISTINCT FROM NEW.status AND NEW.status = 'closed')
    EXECUTE FUNCTION pd_remove_closed_account_goals();
