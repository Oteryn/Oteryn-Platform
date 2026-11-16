-- Retain the same acquisition deadline across application and trigger takers.
-- Shorter caller timeouts always win; transaction-local settings cannot leak.
CREATE FUNCTION oteryn_mutation_budget() RETURNS void LANGUAGE plpgsql AS $$
DECLARE remaining bigint; lock_ms bigint; statement_ms bigint;
BEGIN
    PERFORM set_config('oteryn.mutation_deadline',coalesce(nullif(current_setting('oteryn.mutation_deadline',true),''),(extract(epoch FROM clock_timestamp())+3)::text),true);
    remaining := greatest(1,ceil((current_setting('oteryn.mutation_deadline')::numeric-extract(epoch FROM clock_timestamp()))*1000))::bigint;
    lock_ms := (extract(epoch FROM current_setting('lock_timeout')::interval)*1000)::bigint;
    statement_ms := (extract(epoch FROM current_setting('statement_timeout')::interval)*1000)::bigint;
    PERFORM set_config('lock_timeout',least(remaining,coalesce(nullif(lock_ms,0),remaining))::text || 'ms',true);
    PERFORM set_config('statement_timeout',least(remaining,coalesce(nullif(statement_ms,0),remaining))::text || 'ms',true);
END $$;

CREATE OR REPLACE FUNCTION bazaar_moderation_statement_gate() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM oteryn_mutation_budget();
    PERFORM pg_advisory_xact_lock_shared(64002);
    RETURN NULL;
END;
$$;

CREATE OR REPLACE FUNCTION bazaar_moderation_fence() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE listing uuid; item_active boolean;
BEGIN
    PERFORM oteryn_mutation_budget();
    PERFORM pg_advisory_xact_lock_shared(64002);
    IF TG_TABLE_NAME = 'bazaar_listings' THEN
        listing := NEW.id;
        item_active := NEW.kind='item' AND (NEW.state='active' OR (TG_OP='UPDATE' AND OLD.state='active'));
    ELSE
        listing := NEW.listing_id;
        SELECT kind='item' AND state='active' INTO item_active FROM bazaar_listings WHERE id=listing FOR UPDATE;
    END IF;
    IF item_active AND EXISTS(SELECT 1 FROM bazaar_item_restrictions) THEN
        RAISE EXCEPTION 'item policy requires verified binding' USING ERRCODE='23514';
    END IF;
    IF EXISTS(SELECT 1 FROM bazaar_moderation_holds WHERE listing_id=listing) THEN
        RAISE EXCEPTION 'listing suspended' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION bazaar_character_publication_barrier() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    PERFORM oteryn_mutation_budget();
    -- The same fence protects every Character issuer, including service commands.
    PERFORM pg_advisory_xact_lock(22005160000);
    IF EXISTS (
        SELECT 1 FROM bazaar_current_transfers t
        WHERE t.source_revision < NEW.source_revision
          AND (t.intent::jsonb->>'expires_at_source')::bigint > extract(epoch FROM clock_timestamp())
          AND NOT EXISTS(SELECT 1 FROM character_commands c WHERE c.id=t.operation_id)
    ) THEN
        RAISE EXCEPTION 'private Character transfer awaits publication' USING ERRCODE='55P03';
    END IF;
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION pd_reminder_legacy_canonical() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE canonical pd_scoped_reminders%ROWTYPE;
BEGIN
    PERFORM oteryn_mutation_budget();
    IF pg_trigger_depth() > 1 THEN RETURN NEW; END IF;
    IF TG_OP='INSERT' THEN
        -- Older schedulers refuse a different character/cut at their coarser
        -- conflict target. Serialize legacy inserts so a refused attempt cannot
        -- leave a new scoped intent behind before ON CONFLICT checks its WHERE.
        PERFORM pg_advisory_xact_lock(hashtextextended(jsonb_build_array(
            NEW.account_id,NEW.kind,NEW.entity_key,NEW.channel,NEW.due_at_ms)::text,15120016));
        IF EXISTS (SELECT 1 FROM pd_reminders WHERE account_id=NEW.account_id
            AND kind=NEW.kind AND entity_key=NEW.entity_key AND channel=NEW.channel
            AND due_at_ms=NEW.due_at_ms AND (character_id IS DISTINCT FROM NEW.character_id
                OR cut IS DISTINCT FROM NEW.cut)) THEN
            RETURN NEW;
        END IF;
    END IF;
    INSERT INTO pd_scoped_reminders SELECT NEW.*
    ON CONFLICT ON CONSTRAINT pd_scoped_reminders_identity DO UPDATE
        SET updated_at=pd_scoped_reminders.updated_at
    RETURNING * INTO canonical;
    NEW.id := canonical.id;
    IF canonical.state <> 'pending' THEN
        NEW.state := canonical.state;
        NEW.updated_at := canonical.updated_at;
    END IF;
    RETURN NEW;
END $$;

CREATE OR REPLACE FUNCTION referral_campaign_prepare(ids uuid[],exclusive_account uuid DEFAULT NULL) RETURNS void LANGUAGE plpgsql AS $$
DECLARE held uuid[]; account uuid;
BEGIN
    PERFORM oteryn_mutation_budget();
    held := NULLIF(current_setting('oteryn.referral_prepared_accounts',true),'')::uuid[];
    IF held IS NOT NULL THEN
        IF NOT ids <@ held OR exclusive_account IS NOT NULL THEN
            RAISE EXCEPTION 'referral account set changed; retry transaction' USING ERRCODE='40001';
        END IF;
        RETURN;
    END IF;
    FOR account IN SELECT id FROM accounts WHERE id=ANY(ids) ORDER BY id LOOP
        -- Authentication and account adapters may later need UPDATE/SHARE.
        -- Take the strongest lock now, so no path upgrades after 5600.
        IF pg_trigger_depth()>0 THEN
            PERFORM id FROM accounts WHERE id=account FOR UPDATE NOWAIT;
        ELSE
            PERFORM id FROM accounts WHERE id=account FOR UPDATE;
        END IF;
    END LOOP;
    IF pg_trigger_depth()>0 THEN
        PERFORM account_id FROM premium_authority WHERE account_id=ANY(ids)
            ORDER BY account_id FOR UPDATE NOWAIT;
    ELSE
        PERFORM account_id FROM premium_authority WHERE account_id=ANY(ids)
            ORDER BY account_id FOR UPDATE;
    END IF;
    PERFORM set_config('oteryn.referral_prepared_accounts',ids::text,true);
EXCEPTION WHEN lock_not_available THEN
    RAISE EXCEPTION 'referral preparation conflicts with caller locks; retry transaction' USING ERRCODE='40001';
END $$;

CREATE OR REPLACE FUNCTION referral_campaign_lock(ids uuid[]) RETURNS void LANGUAGE plpgsql AS $$
BEGIN
    PERFORM oteryn_mutation_budget();
    -- Preparation is idempotent and refuses expansion, including discovery that
    -- changed while a caller acquired its premium_authority rows.
    PERFORM referral_campaign_prepare(ids);
    PERFORM pg_advisory_xact_lock(5600);
END $$;

