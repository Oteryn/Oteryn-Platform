CREATE TABLE store_price_list_imports (
    catalog_revision text PRIMARY KEY CHECK (catalog_revision ~ '^[0-9a-f]{64}$'),
    canonical_bytes bytea NOT NULL CHECK (octet_length(canonical_bytes) BETWEEN 1 AND 16777216),
    imported_by uuid NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE store_offers (
    catalog_revision text NOT NULL REFERENCES store_price_list_imports,
    offer_key text NOT NULL,
    currency text NOT NULL CHECK (currency IN ('COINS', 'TRANSFERABLE_COINS')),
    unit_price bigint NOT NULL CHECK (unit_price > 0),
    max_quantity integer NOT NULL CHECK (max_quantity BETWEEN 1 AND 100),
    purchasable boolean NOT NULL,
    canonical_bytes bytea NOT NULL,
    PRIMARY KEY (catalog_revision, offer_key)
);
CREATE TABLE store_active_catalog (
    id integer PRIMARY KEY CHECK (id = 1),
    catalog_revision text REFERENCES store_price_list_imports,
    activated_by uuid,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
INSERT INTO store_active_catalog (id) VALUES (1);
CREATE TABLE store_catalog_activations (
    activation_id uuid PRIMARY KEY,
    activation_sequence bigint GENERATED ALWAYS AS IDENTITY UNIQUE,
    previous_revision text REFERENCES store_price_list_imports,
    catalog_revision text NOT NULL REFERENCES store_price_list_imports,
    actor_id uuid NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE FUNCTION store_catalog_reject_mutation() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'Store catalog history is immutable';
END;
$$;
CREATE TRIGGER store_imports_immutable BEFORE UPDATE OR DELETE ON store_price_list_imports
    FOR EACH ROW EXECUTE FUNCTION store_catalog_reject_mutation();
CREATE TRIGGER store_offers_immutable BEFORE UPDATE OR DELETE ON store_offers
    FOR EACH ROW EXECUTE FUNCTION store_catalog_reject_mutation();
CREATE TRIGGER store_activations_immutable BEFORE UPDATE OR DELETE ON store_catalog_activations
    FOR EACH ROW EXECUTE FUNCTION store_catalog_reject_mutation();
