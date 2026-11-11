-- Preserve legal acceptance while allowing repeated optional consent decisions.
ALTER TABLE account_consents DROP CONSTRAINT account_consents_pkey;
ALTER TABLE account_consents DROP CONSTRAINT account_consents_document_kind_check;
ALTER TABLE account_consents
    ADD COLUMN id uuid NOT NULL DEFAULT uuidv7(),
    ADD COLUMN granted boolean NOT NULL DEFAULT true,
    ADD COLUMN actor uuid,
    ADD COLUMN source text NOT NULL DEFAULT 'web' CHECK (source = 'web'),
    ADD PRIMARY KEY (id),
    ADD CHECK (document_kind IN ('terms', 'privacy', 'play_session', 'personalised_ads'));
UPDATE account_consents SET actor = account_id;
ALTER TABLE account_consents ALTER COLUMN granted SET DEFAULT false;
ALTER TABLE account_consents ALTER COLUMN actor SET NOT NULL;
ALTER TABLE account_consents ADD CHECK (actor = account_id);
CREATE UNIQUE INDEX account_consents_legal_version ON account_consents(account_id, document_kind, document_version)
    WHERE document_kind IN ('terms', 'privacy');
CREATE INDEX account_consents_latest ON account_consents(account_id, document_kind, accepted_at DESC, id DESC);

CREATE FUNCTION account_consent_actor() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    NEW.actor := COALESCE(NEW.actor, NEW.account_id);
    IF NEW.document_kind IN ('terms', 'privacy') THEN NEW.granted := true; END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER account_consent_actor BEFORE INSERT ON account_consents
    FOR EACH ROW EXECUTE FUNCTION account_consent_actor();

CREATE FUNCTION account_consent_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF OLD.document_kind IN ('play_session', 'personalised_ads') THEN
        RAISE EXCEPTION 'Consent decisions are immutable' USING ERRCODE = '23514';
    END IF;
    IF TG_OP = 'DELETE' THEN RETURN OLD; END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER account_consent_immutable BEFORE UPDATE OR DELETE ON account_consents
    FOR EACH ROW EXECUTE FUNCTION account_consent_immutable();
