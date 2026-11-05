-- Local listing intents are staged, never game wire commands and never worker-dispatched.
CREATE TABLE bazaar_character_intents (
    listing_id uuid PRIMARY KEY REFERENCES bazaar_listings(id),
    owner uuid NOT NULL,
    character_id uuid NOT NULL,
    request_id uuid NOT NULL,
    currency text NOT NULL CHECK (currency IN ('coins','gold')),
    starting_bid bigint NOT NULL CHECK (starting_bid>0 AND (currency<>'gold' OR starting_bid<=999999999999)),
    duration_days smallint NOT NULL CHECK (duration_days BETWEEN 1 AND 14),
    publication_consent boolean NOT NULL CHECK (publication_consent),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    UNIQUE(owner,request_id)
);
CREATE TABLE bazaar_character_publications (
    listing_id uuid PRIMARY KEY REFERENCES bazaar_character_intents(listing_id),
    projection text NOT NULL CHECK (octet_length(projection) <= 524288),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TRIGGER bazaar_character_intent_immutable BEFORE UPDATE OR DELETE ON bazaar_character_intents FOR EACH ROW EXECUTE FUNCTION bazaar_append_only();
CREATE TRIGGER bazaar_character_publication_immutable BEFORE UPDATE OR DELETE ON bazaar_character_publications FOR EACH ROW EXECUTE FUNCTION bazaar_append_only();
