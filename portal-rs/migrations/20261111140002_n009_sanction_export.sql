-- One transactional counter serializes publication until commit. A bare sequence
-- would allow an older allocated number to commit behind a consumer cursor.
CREATE TABLE support_sanction_export_counter (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    position bigint NOT NULL DEFAULT 0 CHECK (position >= 0)
);
INSERT INTO support_sanction_export_counter DEFAULT VALUES;
CREATE TABLE support_sanction_export_reasons (
    sanction_id uuid PRIMARY KEY REFERENCES support_sanction_identities(sanction_id),
    reason_code integer NOT NULL CHECK (reason_code BETWEEN 1 AND 65535),
    reason_text text NOT NULL CHECK (octet_length(reason_text) BETWEEN 1 AND 120
        AND reason_text !~ U&'[[:cntrl:]\0080-\009f]'),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE support_sanction_export_revisions (
    position bigint PRIMARY KEY CHECK (position > 0),
    sanction_stream_id uuid NOT NULL CHECK (uuid_extract_version(sanction_stream_id) = 7),
    decision_revision bigint NOT NULL CHECK (decision_revision > 0),
    account_id uuid NOT NULL CHECK (uuid_extract_version(account_id) = 7),
    character_id uuid CHECK (uuid_extract_version(character_id) = 7),
    kind smallint NOT NULL CHECK (kind BETWEEN 1 AND 6),
    reason_code integer NOT NULL CHECK (reason_code BETWEEN 1 AND 65535),
    reason_text text NOT NULL CHECK (octet_length(reason_text) BETWEEN 1 AND 120
        AND reason_text !~ U&'[[:cntrl:]\0080-\009f]'),
    starts_at_ms bigint NOT NULL CHECK (starts_at_ms > 0),
    ends_at_ms bigint CHECK (ends_at_ms > starts_at_ms),
    revoked boolean NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (sanction_stream_id, decision_revision),
    CHECK ((kind IN (1,2,3,5) AND character_id IS NOT NULL)
        OR (kind = 4 AND character_id IS NULL) OR kind = 6),
    CHECK (kind NOT IN (1,2) OR ends_at_ms IS NOT NULL),
    CHECK (kind NOT IN (5,6) OR ends_at_ms IS NULL)
);

-- Preserve public text, replace controls with spaces, then shorten at a UTF-8
-- boundary. Never select moderator notes, staff identities, evidence or appeals.
CREATE FUNCTION support_sanction_export_text(raw text) RETURNS text
LANGUAGE plpgsql IMMUTABLE STRICT AS $$
DECLARE result text := btrim(regexp_replace(raw, U&'[[:cntrl:]\0080-\009f]', ' ', 'g'));
BEGIN
    WHILE octet_length(result) > 120 LOOP
        result := left(result, char_length(result)-1);
    END LOOP;
    result := btrim(result);
    IF result = '' THEN
        RAISE EXCEPTION 'empty public sanction reason' USING ERRCODE = '23514';
    END IF;
    RETURN result;
END $$;

CREATE FUNCTION support_sanction_export_append(
    stream uuid, account uuid, target_character uuid, wire_kind smallint,
    code integer, reason text, starts bigint, ends bigint, lifted boolean
) RETURNS void LANGUAGE plpgsql AS $$
DECLARE
    previous support_sanction_export_revisions%ROWTYPE;
    next_position bigint;
BEGIN
    UPDATE support_sanction_export_counter SET position = position + 1
        WHERE singleton RETURNING position INTO next_position;
    SELECT * INTO previous FROM support_sanction_export_revisions
        WHERE sanction_stream_id = stream ORDER BY decision_revision DESC LIMIT 1;
    IF FOUND AND (previous.account_id <> account
        OR previous.character_id IS DISTINCT FROM target_character OR previous.kind <> wire_kind) THEN
        RAISE EXCEPTION 'sanction export identity is immutable' USING ERRCODE = '23514';
    END IF;
    INSERT INTO support_sanction_export_revisions
        (position,sanction_stream_id,decision_revision,account_id,character_id,kind,
         reason_code,reason_text,starts_at_ms,ends_at_ms,revoked)
    VALUES (next_position,stream,COALESCE(previous.decision_revision,0)+1,
        account,target_character,wire_kind,code,reason,starts,ends,
        lifted OR COALESCE(previous.revoked,false));
END $$;

CREATE FUNCTION support_sanction_export_legacy() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE wire_kind smallint;
BEGIN
    wire_kind := CASE NEW.category WHEN 'warning' THEN 6 WHEN 'suspension' THEN 4 END;
    IF wire_kind IS NULL OR NEW.status = 'pending' THEN
        IF EXISTS (SELECT 1 FROM support_sanction_export_revisions WHERE sanction_stream_id=NEW.id) THEN
            RAISE EXCEPTION 'confirmed export cannot become private' USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;
    PERFORM support_sanction_export_append(NEW.id,NEW.account_id,NULL,wire_kind,
        CASE NEW.category WHEN 'warning' THEN 1 ELSE 3 END,
        support_sanction_export_text(NEW.public_reason),
        floor(extract(epoch FROM NEW.effective_at)*1000)::bigint,
        CASE WHEN wire_kind = 4 THEN floor(extract(epoch FROM NEW.expires_at)*1000)::bigint END,
        NEW.status = 'revoked' OR EXISTS (SELECT 1 FROM support_appeal_workflows
            WHERE sanction_id=NEW.id AND owner_id=NEW.account_id AND status='accepted'));
    RETURN NEW;
END $$;
CREATE TRIGGER support_sanction_export_legacy AFTER INSERT OR UPDATE ON sanctions
    FOR EACH ROW EXECUTE FUNCTION support_sanction_export_legacy();

CREATE FUNCTION support_sanction_export_typed_record(item support_sanction_revisions) RETURNS void LANGUAGE plpgsql AS $$
DECLARE
    identity_row support_sanction_identities%ROWTYPE;
    public_row support_sanction_export_reasons%ROWTYPE;
    first_row support_sanction_revisions%ROWTYPE;
    last_active support_sanction_revisions%ROWTYPE;
    wire_kind smallint;
BEGIN
    SELECT * INTO identity_row FROM support_sanction_identities WHERE sanction_id=item.sanction_id;
    wire_kind := CASE identity_row.kind WHEN 'ban' THEN 4 WHEN 'namelock' THEN 5 WHEN 'warning' THEN 6 END;
    SELECT * INTO public_row FROM support_sanction_export_reasons WHERE sanction_id=item.sanction_id;
    IF wire_kind IS NULL OR NOT FOUND THEN RETURN; END IF;
    SELECT * INTO first_row FROM support_sanction_revisions WHERE sanction_id=item.sanction_id
        ORDER BY revision LIMIT 1;
    SELECT * INTO last_active FROM support_sanction_revisions WHERE sanction_id=item.sanction_id
        AND active AND revision <= item.revision ORDER BY revision DESC LIMIT 1;
    PERFORM support_sanction_export_append(item.sanction_id,item.account_id,identity_row.character_id,
        wire_kind,public_row.reason_code,public_row.reason_text,
        floor(extract(epoch FROM first_row.decision_at)*1000)::bigint,
        CASE WHEN wire_kind=4 THEN floor(extract(epoch FROM
            CASE WHEN item.active THEN item.expires_at ELSE last_active.expires_at END)*1000)::bigint END,
        NOT item.active OR EXISTS (SELECT 1 FROM support_appeal_workflows
            WHERE sanction_id=item.sanction_id AND owner_id=item.account_id AND status='accepted'));
    RETURN;
END $$;
CREATE FUNCTION support_sanction_export_typed() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.active AND EXISTS (SELECT 1 FROM support_sanction_identities
        WHERE sanction_id=NEW.sanction_id AND kind='namelock') THEN
        -- An in-flight extension may have passed its application-level check
        -- before an expiry poll committed. Fence revival at publication too.
        PERFORM 1 FROM support_sanction_export_counter WHERE singleton FOR UPDATE;
        IF EXISTS (SELECT 1 FROM support_sanction_export_revisions
            WHERE sanction_stream_id=NEW.sanction_id AND revoked) THEN
            RAISE EXCEPTION 'revoked namelock requires a new sanction identity' USING ERRCODE = '23514';
        END IF;
    END IF;
    PERFORM support_sanction_export_typed_record(NEW);
    RETURN NEW;
END $$;
CREATE TRIGGER support_sanction_export_typed AFTER INSERT ON support_sanction_revisions
    FOR EACH ROW EXECUTE FUNCTION support_sanction_export_typed();

-- A reason registered after the decision publishes its existing history in order.
-- Reason corrections publish a fresh current-state revision.
CREATE FUNCTION support_sanction_export_reason() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE item support_sanction_revisions%ROWTYPE;
BEGIN
    -- Match the typed decision writer's account lock so metadata registration
    -- cannot race a decision that observes the metadata as not yet committed.
    PERFORM 1 FROM accounts WHERE id=(SELECT account_id FROM support_sanction_identities
        WHERE sanction_id=NEW.sanction_id) FOR UPDATE;
    FOR item IN SELECT * FROM support_sanction_revisions WHERE sanction_id=NEW.sanction_id
        AND (TG_OP='INSERT' OR revision=(SELECT max(revision) FROM support_sanction_revisions
            WHERE sanction_id=NEW.sanction_id)) ORDER BY revision LOOP
        PERFORM support_sanction_export_typed_record(item);
    END LOOP;
    RETURN NEW;
END $$;
CREATE TRIGGER support_sanction_export_reason AFTER INSERT OR UPDATE ON support_sanction_export_reasons
    FOR EACH ROW EXECUTE FUNCTION support_sanction_export_reason();

CREATE FUNCTION support_sanction_export_appeal() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE previous support_sanction_export_revisions%ROWTYPE;
BEGIN
    IF NEW.status NOT IN ('reviewing','accepted','rejected')
        OR (TG_OP = 'UPDATE' AND NEW.status = OLD.status AND NEW.outcome IS NOT DISTINCT FROM OLD.outcome)
        THEN RETURN NEW; END IF;
    -- Take the publication lock before reading the latest decision.
    PERFORM 1 FROM support_sanction_export_counter WHERE singleton FOR UPDATE;
    SELECT * INTO previous FROM support_sanction_export_revisions
        WHERE sanction_stream_id=NEW.sanction_id AND account_id=NEW.owner_id
        ORDER BY decision_revision DESC LIMIT 1;
    IF FOUND THEN
        PERFORM support_sanction_export_append(previous.sanction_stream_id,previous.account_id,
            previous.character_id,previous.kind,previous.reason_code,previous.reason_text,
            previous.starts_at_ms,previous.ends_at_ms,previous.revoked OR NEW.status='accepted');
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER support_sanction_export_appeal AFTER INSERT OR UPDATE ON support_appeal_workflows
    FOR EACH ROW EXECUTE FUNCTION support_sanction_export_appeal();

-- Existing confirmed T34 records get an initial revision; pending and unmapped
-- records remain private. Row ordering makes the initial cursor deterministic.
DO $$
DECLARE item sanctions%ROWTYPE;
BEGIN
    FOR item IN SELECT * FROM sanctions WHERE category IN ('warning','suspension')
        AND status <> 'pending' ORDER BY created_at,id LOOP
        PERFORM support_sanction_export_append(item.id,item.account_id,NULL,
            (CASE item.category WHEN 'warning' THEN 6 ELSE 4 END)::smallint,
            CASE item.category WHEN 'warning' THEN 1 ELSE 3 END,
            support_sanction_export_text(item.public_reason),
            floor(extract(epoch FROM item.effective_at)*1000)::bigint,
            CASE WHEN item.category='suspension' THEN floor(extract(epoch FROM item.expires_at)*1000)::bigint END,
            item.status='revoked' OR EXISTS (SELECT 1 FROM support_appeal_workflows
                WHERE sanction_id=item.id AND owner_id=item.account_id AND status='accepted'));
    END LOOP;
END $$;

-- Kind 5 has no wire end time. Reconcile elapsed authoritative deadlines before
-- each poll so consumers receive a durable, sticky revocation instead. Only the
-- latest typed decision can expire; an extension made before expiry wins.
CREATE FUNCTION support_sanction_export_expire_namelocks() RETURNS void LANGUAGE plpgsql AS $$
DECLARE previous support_sanction_export_revisions%ROWTYPE;
BEGIN
    -- A reader must still see committed history while a publisher is pending.
    -- Defer expiry to the next poll if publication is busy; never lock accounts
    -- here, because decision writers take account then counter.
    PERFORM 1 FROM support_sanction_export_counter WHERE singleton FOR UPDATE SKIP LOCKED;
    IF NOT FOUND THEN RETURN; END IF;
    FOR previous IN
        SELECT exported.* FROM (
            SELECT DISTINCT ON (sanction_stream_id) *
            FROM support_sanction_export_revisions WHERE kind=5
            ORDER BY sanction_stream_id,decision_revision DESC
        ) exported
        JOIN LATERAL (
            SELECT active,expires_at FROM support_sanction_revisions
            WHERE sanction_id=exported.sanction_stream_id ORDER BY revision DESC LIMIT 1
        ) authoritative ON true
        WHERE NOT exported.revoked AND authoritative.active
            AND authoritative.expires_at<=clock_timestamp()
        ORDER BY exported.position LIMIT 500
    LOOP
        PERFORM support_sanction_export_append(previous.sanction_stream_id,previous.account_id,
            previous.character_id,previous.kind,previous.reason_code,previous.reason_text,
            previous.starts_at_ms,NULL,true);
    END LOOP;
END $$;

CREATE FUNCTION support_sanction_export_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'sanction export journal is append only' USING ERRCODE = '23514';
END $$;
CREATE TRIGGER support_sanction_export_immutable BEFORE UPDATE OR DELETE ON support_sanction_export_revisions
    FOR EACH ROW EXECUTE FUNCTION support_sanction_export_immutable();
