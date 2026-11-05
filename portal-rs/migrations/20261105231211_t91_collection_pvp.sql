CREATE TABLE collection_pvp_tournaments (
 id uuid PRIMARY KEY, organizer uuid NOT NULL REFERENCES accounts(id),
 capacity integer NOT NULL CHECK(capacity BETWEEN 2 AND 16),
 status text NOT NULL DEFAULT 'registration' CHECK(status IN ('registration','active','completed')),
 created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE collection_pvp_entries (
 tournament_id uuid NOT NULL REFERENCES collection_pvp_tournaments(id),
 account_id uuid NOT NULL REFERENCES accounts(id), deck jsonb NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(tournament_id,account_id)
);
CREATE TABLE collection_pvp_battles (
 id uuid PRIMARY KEY, left_account uuid NOT NULL REFERENCES accounts(id),
 right_account uuid NOT NULL REFERENCES accounts(id), left_deck jsonb NOT NULL,
 right_deck jsonb, state jsonb NOT NULL DEFAULT '{}',
 status text NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','active','declined','completed')),
 revision bigint NOT NULL DEFAULT 0 CHECK(revision>=0), winner uuid REFERENCES accounts(id),
 tournament_id uuid REFERENCES collection_pvp_tournaments(id),
 created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now(),
 CHECK(left_account<>right_account), CHECK(winner IS NULL OR winner IN(left_account,right_account)),
 CHECK(status='completed' OR winner IS NULL), CHECK(status IN ('pending','declined') OR right_deck IS NOT NULL),
 UNIQUE(tournament_id,left_account,right_account)
);
CREATE INDEX collection_pvp_battles_left ON collection_pvp_battles(left_account,created_at);
CREATE INDEX collection_pvp_battles_right ON collection_pvp_battles(right_account,created_at);
CREATE TABLE collection_pvp_sets (
 id uuid PRIMARY KEY, version bigint NOT NULL CHECK(version>0), price bigint NOT NULL CHECK(price>0),
 cards jsonb NOT NULL CHECK(jsonb_typeof(cards)='object' AND cards<>'{}'::jsonb),
 published boolean NOT NULL DEFAULT false,
 created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE collection_pvp_purchases (
 id uuid PRIMARY KEY, account_id uuid NOT NULL REFERENCES accounts(id),
 set_id uuid NOT NULL REFERENCES collection_pvp_sets(id), version bigint NOT NULL CHECK(version>0),
 price bigint NOT NULL CHECK(price>0), cards jsonb NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now()
);
-- Contents and price are immutable. A changed offer gets a new identifier.
CREATE FUNCTION collection_pvp_fixed_offer() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF (NEW.id,NEW.version,NEW.price,NEW.cards) IS DISTINCT FROM (OLD.id,OLD.version,OLD.price,OLD.cards) THEN
  RAISE EXCEPTION 'fixed set offers are immutable';
 END IF;
 RETURN NEW;
END;
$$;
CREATE TRIGGER collection_pvp_fixed_offer BEFORE UPDATE ON collection_pvp_sets
 FOR EACH ROW EXECUTE FUNCTION collection_pvp_fixed_offer();
