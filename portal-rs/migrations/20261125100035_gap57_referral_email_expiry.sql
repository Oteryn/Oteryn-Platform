-- Use the same expiration rule for browser confirmation and session attribution.
-- Legacy invitations without attempt rows retain their original creation time.
CREATE INDEX referral_email_attempts_invitation ON referral_email_attempts(invitation_id);

CREATE FUNCTION referral_email_code_open(code uuid, issued_at timestamptz)
RETURNS boolean LANGUAGE sql STRICT AS $$
    SELECT clock_timestamp() < GREATEST(issued_at, max(created_at), max(delivered_at)) + interval '14 days'
    FROM referral_email_attempts WHERE invitation_id=code
$$;

CREATE OR REPLACE FUNCTION referral_campaign_signup(a uuid,code text) RETURNS boolean
LANGUAGE plpgsql AS $$
DECLARE owner uuid; invitation_time timestamptz; recipient text; token uuid; phase integer; emailed boolean;
BEGIN
    IF NOT referral_deployment_enabled() THEN RETURN false; END IF;
    IF NOT EXISTS(SELECT 1 FROM referral_campaign_settings WHERE enabled) THEN RETURN false; END IF;
    BEGIN token := code::uuid;
    EXCEPTION WHEN invalid_text_representation THEN RETURN false; END;
    IF uuid_extract_version(token) IS DISTINCT FROM 7 OR uuid_extract_version(a) IS DISTINCT FROM 7 THEN RETURN false; END IF;
    SELECT account_id,created_at,NULL::text INTO owner,invitation_time,recipient
        FROM referral_links WHERE referral_links.code=token;
    IF NOT FOUND THEN
        SELECT inviter_id,created_at,referral_email_invitations.recipient INTO owner,invitation_time,recipient
            FROM referral_email_invitations WHERE id=token AND kind='new' AND referral_email_code_open(id,created_at);
    END IF;
    IF owner IS NULL OR owner=a THEN RETURN false; END IF;
    emailed := recipient IS NOT NULL;
    -- Validate before preparing locks, then revalidate under 5600. Invalid or
    -- already-bound session codes must not expand a transaction's account set.
    FOR phase IN 1..2 LOOP
        IF emailed AND NOT referral_email_code_open(token,invitation_time) THEN RETURN false; END IF;
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
