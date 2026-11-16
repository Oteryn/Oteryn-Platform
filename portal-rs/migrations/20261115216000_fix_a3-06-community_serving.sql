-- A serve-time revision fence covers restrictive privacy/identity changes.
-- Readers sample before resolving permissions, then validate under FOR SHARE.
CREATE TABLE community_privacy_revision (id integer PRIMARY KEY CHECK (id=1), revision bigint NOT NULL DEFAULT 0);
INSERT INTO community_privacy_revision(id) VALUES(1);
CREATE FUNCTION community_privacy_changed() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    UPDATE community_privacy_revision SET revision=revision+1 WHERE id=1;
    RETURN NULL;
END $$;
CREATE TRIGGER community_account_privacy_changed AFTER UPDATE OF privacy_revision,status,email_verified_at ON accounts FOR EACH STATEMENT EXECUTE FUNCTION community_privacy_changed();
CREATE TRIGGER community_character_privacy_changed AFTER INSERT OR UPDATE OR DELETE ON character_profiles FOR EACH STATEMENT EXECUTE FUNCTION community_privacy_changed();
CREATE TRIGGER community_character_identity_changed AFTER INSERT OR UPDATE OR DELETE ON account_characters FOR EACH STATEMENT EXECUTE FUNCTION community_privacy_changed();

-- Indexed parent lookups with the same 256-visited-node work bound as Rust.
CREATE FUNCTION community_referral_ancestry_denied(owner uuid, target uuid) RETURNS boolean LANGUAGE plpgsql AS $$
DECLARE pending uuid[] := ARRAY[owner]; seen uuid[] := ARRAY[]::uuid[]; current_id uuid; parent uuid;
BEGIN
    WHILE cardinality(pending)>0 LOOP
        current_id := pending[cardinality(pending)];
        pending := pending[1:cardinality(pending)-1];
        IF current_id=target OR cardinality(seen)>=256 THEN RETURN true; END IF;
        IF current_id=ANY(seen) THEN CONTINUE; END IF;
        seen := array_append(seen,current_id);
        SELECT inviter_id INTO parent FROM referral_invitations WHERE invitee_id=current_id;
        IF FOUND THEN pending := array_append(pending,parent); END IF;
        SELECT inviter_id INTO parent FROM referral_campaign_invitations WHERE invitee_id=current_id;
        IF FOUND THEN pending := array_append(pending,parent); END IF;
    END LOOP;
    RETURN false;
END $$;
CREATE OR REPLACE FUNCTION referral_campaign_signup(a uuid,code text) RETURNS boolean
LANGUAGE plpgsql AS $$
DECLARE owner uuid; invitation_time timestamptz; recipient text; token uuid; phase integer;
BEGIN
    IF NOT EXISTS(SELECT 1 FROM referral_campaign_settings WHERE enabled) THEN RETURN false; END IF;
    BEGIN token := code::uuid;
    EXCEPTION WHEN invalid_text_representation THEN RETURN false; END;
    IF uuid_extract_version(token) IS DISTINCT FROM 7 OR uuid_extract_version(a) IS DISTINCT FROM 7 THEN RETURN false; END IF;
    SELECT account_id,created_at,NULL::text INTO owner,invitation_time,recipient
        FROM referral_links WHERE referral_links.code=token;
    IF NOT FOUND THEN
        SELECT inviter_id,created_at,referral_email_invitations.recipient INTO owner,invitation_time,recipient
            FROM referral_email_invitations WHERE id=token AND kind='new';
    END IF;
    IF owner IS NULL OR owner=a THEN RETURN false; END IF;
    -- Validate before preparing locks, then revalidate under 5600. Invalid or
    -- already-bound session codes must not expand a transaction's account set.
    FOR phase IN 1..2 LOOP
        IF (SELECT count(*) FROM accounts WHERE id IN (a,owner) AND account_value_eligible(id))<>2 THEN RETURN false; END IF;
        IF EXISTS(SELECT 1 FROM referral_campaign_invitations WHERE invitee_id=a) THEN
            RETURN EXISTS(SELECT 1 FROM referral_campaign_invitations WHERE invitee_id=a AND inviter_id=owner AND kind='new');
        END IF;
        IF NOT EXISTS(SELECT 1 FROM accounts WHERE id=a AND created_at>=invitation_time
            AND created_at>clock_timestamp()-interval '7 days'
            AND (recipient IS NULL OR lower(email)=recipient))
            OR EXISTS(SELECT 1 FROM store_orders WHERE account_id=a) THEN RETURN false; END IF;
        IF community_referral_ancestry_denied(owner,a)
            OR EXISTS(SELECT 1 FROM referral_invitations WHERE invitee_id=a) THEN RETURN false; END IF;
        IF phase=1 THEN
            -- A verified login must bind or abort: deferring to the worker lets
            -- a first purchase commit before attribution and makes it ineligible.
            PERFORM referral_campaign_lock(ARRAY[a,owner]);
        END IF;
    END LOOP;
    INSERT INTO referral_campaign_invitations(inviter_id,invitee_id,kind) VALUES(owner,a,'new');
    PERFORM referral_campaign_reward('starter:'||i.id,a,'starter',3,NULL,b.created_at+interval '7 days')
        FROM referral_campaign_invitations i JOIN accounts b ON b.id=i.invitee_id WHERE i.invitee_id=a;
    RETURN true;
END $$;

-- Delivery authorization is revocable until the send attempt begins. It takes
-- no account/qualification locks across SMTP, which must not stall purchases.
CREATE FUNCTION community_invitation_delivery_eligible(target uuid) RETURNS boolean
LANGUAGE sql AS $$
    SELECT CASE
                   WHEN a.status<>'active' THEN 'account_'||a.status
                   WHEN a.email_verified_at IS NULL THEN 'email_unverified'
                   WHEN a.email_change_blocked_until>clock_timestamp() THEN 'email_change_lock'
                   WHEN EXISTS(SELECT 1 FROM account_terminations t WHERE t.account_id=a.id
                     AND t.cancelled_at IS NULL AND t.finalized_at IS NULL) THEN 'termination_pending'
                   WHEN EXISTS(SELECT 1 FROM sanctions s WHERE s.account_id=a.id AND s.status='active'
                     AND s.category IN ('restriction','suspension') AND s.effective_at<=clock_timestamp()
                     AND (s.expires_at IS NULL OR s.expires_at>clock_timestamp())) THEN 'sanction'
                   WHEN EXISTS(SELECT 1 FROM sanctions s WHERE s.account_id=a.id
                     AND (s.category NOT IN ('warning','restriction','suspension')
                     OR s.status NOT IN ('pending','active','expired','revoked'))) THEN 'sanction_unknown'
                   WHEN EXISTS(SELECT 1 FROM (SELECT DISTINCT ON (r.sanction_id) r.active,r.expires_at
                     FROM support_sanction_revisions r JOIN support_sanction_identities i USING(sanction_id)
                     WHERE r.account_id=a.id AND i.kind='ban' ORDER BY r.sanction_id,r.revision DESC) latest
                     WHERE latest.active AND (latest.expires_at IS NULL OR latest.expires_at>clock_timestamp()))
                     THEN 'ban'
                 END IS NULL FROM accounts a WHERE a.id=target;
$$;
