-- Browser activity is recorded by the explicitly enabled session code.
DROP TRIGGER referral_activity_web ON web_sessions;

-- Configured portal pools always pass an explicit setting. SQL callers outside
-- those pools retain the original behavior, including the campaign setting.
CREATE FUNCTION referral_deployment_enabled() RETURNS boolean LANGUAGE sql STABLE AS $$
    SELECT current_setting('oteryn.referrals_enabled',true) IS DISTINCT FROM 'false'
$$;

CREATE FUNCTION referral_record_activity(a uuid, stamp timestamptz)
RETURNS void LANGUAGE sql STRICT AS $$
    INSERT INTO referral_activity(account_id,last_activity)
    SELECT id,stamp FROM accounts WHERE id=a AND referral_deployment_enabled()
    ON CONFLICT(account_id) DO UPDATE
    SET last_activity=GREATEST(referral_activity.last_activity,EXCLUDED.last_activity),
        updated_at=clock_timestamp()
$$;

-- The game-login projector also upserts activity for expired presentation
-- records. Keep the fence on referral-owned data for that direct write.
CREATE FUNCTION referral_skip_disabled_activity() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RETURN NULL;
END $$;

CREATE TRIGGER referral_activity_deployment_gate BEFORE INSERT ON referral_activity
    FOR EACH ROW WHEN (NOT referral_deployment_enabled())
    EXECUTE FUNCTION referral_skip_disabled_activity();

CREATE OR REPLACE FUNCTION referral_activity_capture() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE a uuid; stamp timestamptz;
BEGIN
    IF NOT referral_deployment_enabled() THEN RETURN NEW; END IF;
    IF TG_TABLE_NAME='accounts' THEN a:=NEW.id; stamp:=NEW.created_at;
    ELSIF TG_TABLE_NAME='pd_game_logins' THEN a:=NEW.account_id; stamp:=to_timestamp(GREATEST(NEW.entered_at,NEW.left_at)/1000.0);
    ELSIF TG_TABLE_NAME='security_events' THEN
        IF NEW.kind<>'login_succeeded' THEN RETURN NEW; END IF;
        a:=NEW.account_id; stamp:=NEW.created_at;
    END IF;
    PERFORM referral_record_activity(a,stamp);
    RETURN NEW;
END $$;

-- Signup, game-login and store callbacks have shared callers. Check the
-- deployment switch before touching campaign state, even if the database
-- campaign switch remains on.
CREATE OR REPLACE FUNCTION referral_campaign_reward(k text,a uuid,t text,n integer,c uuid DEFAULT NULL,e timestamptz DEFAULT NULL)
RETURNS void LANGUAGE plpgsql AS $$
DECLARE reward uuid;
BEGIN
    IF NOT referral_deployment_enabled() THEN RETURN; END IF;
    IF NOT account_value_eligible(a) THEN RETURN; END IF;
    INSERT INTO referral_campaign_rewards(operation_key,account_id,kind,amount,character_id,expires_at)
    VALUES(k,a,t,n,c,e) ON CONFLICT(operation_key) DO NOTHING RETURNING id INTO reward;
    IF reward IS NOT NULL AND t <> 'premium' THEN
        INSERT INTO outbox(kind,idempotency_key,payload,run_after)
        VALUES('referral.entitlement',k,jsonb_build_object('entitlement_id',reward,'account_id',a,'kind',t,'quantity',n,'character_scope',CASE WHEN t='starter' THEN 'FIRST_THREE_CREATED' WHEN t='welcome' THEN 'EXACT_CHARACTER' ELSE 'ACCOUNT' END,'character_id',c,'expires_at',e),'infinity');
    END IF;
END $$;

CREATE OR REPLACE FUNCTION referral_campaign_purchase(a uuid,days integer,paid_at timestamptz)
RETURNS void LANGUAGE plpgsql AS $$
DECLARE invitation referral_campaign_invitations; paying bigint; threshold integer;
BEGIN
    IF NOT referral_deployment_enabled() THEN RETURN; END IF;
    IF NOT EXISTS(SELECT 1 FROM referral_campaign_settings WHERE enabled) THEN RETURN; END IF;
    PERFORM referral_campaign_lock(ARRAY[a] || ARRAY(SELECT inviter_id FROM referral_campaign_invitations WHERE invitee_id=a));
    SELECT i.* INTO invitation FROM referral_campaign_invitations i
      JOIN accounts buyer ON buyer.id=i.invitee_id AND account_value_eligible(buyer.id)
      JOIN accounts owner ON owner.id=i.inviter_id AND account_value_eligible(owner.id)
      WHERE i.invitee_id=a AND i.created_at<=paid_at FOR UPDATE OF i;
    IF NOT FOUND THEN RETURN; END IF;
    IF invitation.kind='new' THEN
        UPDATE referral_campaign_invitations SET first_paid_at=COALESCE(first_paid_at,paid_at),updated_at=clock_timestamp() WHERE id=invitation.id;
        SELECT count(*) INTO paying FROM referral_campaign_invitations WHERE inviter_id=invitation.inviter_id AND kind='new' AND first_paid_at IS NOT NULL;
        FOREACH threshold IN ARRAY ARRAY[1,3,10] LOOP
            IF paying>=threshold THEN
                PERFORM referral_campaign_reward('recruiter:'||invitation.inviter_id||':'||threshold,invitation.inviter_id,CASE threshold WHEN 1 THEN 'outfit' WHEN 3 THEN 'addon1' ELSE 'addon2' END,1);
            END IF;
        END LOOP;
    ELSIF paid_at<=invitation.created_at+interval '90 days' AND days>=30 THEN
        -- The first qualifying purchase fixes the tier; later purchases replay it.
        PERFORM referral_campaign_reward('buddy-premium:'||invitation.id,invitation.inviter_id,'premium',CASE WHEN days>=90 THEN 10 ELSE 5 END);
    END IF;
END $$;

CREATE OR REPLACE FUNCTION referral_campaign_loyalty(a uuid,earned integer)
RETURNS void LANGUAGE plpgsql AS $$
DECLARE invitation referral_campaign_invitations; delta integer;
BEGIN
    IF NOT referral_deployment_enabled() THEN RETURN; END IF;
    IF NOT EXISTS(SELECT 1 FROM referral_campaign_settings WHERE enabled) THEN RETURN; END IF;
    PERFORM referral_campaign_lock(ARRAY[a] || ARRAY(SELECT inviter_id FROM referral_campaign_invitations WHERE invitee_id=a AND kind='new'));
    SELECT i.* INTO invitation FROM referral_campaign_invitations i
      JOIN accounts buyer ON buyer.id=i.invitee_id AND account_value_eligible(buyer.id)
      JOIN accounts owner ON owner.id=i.inviter_id AND account_value_eligible(owner.id)
      WHERE i.invitee_id=a AND i.kind='new' FOR UPDATE OF i;
    IF NOT FOUND OR earned<=invitation.premium_points THEN RETURN; END IF;
    delta := earned/6-invitation.premium_points/6;
    IF delta>0 THEN
        INSERT INTO referral_loyalty_credits(operation_key,account_id,points)
        VALUES('loyalty:'||invitation.id||':'||(earned/6),invitation.inviter_id,delta) ON CONFLICT DO NOTHING;
    END IF;
    UPDATE referral_campaign_invitations SET premium_points=earned,updated_at=clock_timestamp() WHERE id=invitation.id;
END $$;

CREATE OR REPLACE FUNCTION referral_campaign_login() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE invitation referral_campaign_invitations;
BEGIN
    IF NOT referral_deployment_enabled() THEN RETURN NEW; END IF;
    IF NEW.character_id IS NULL OR NOT EXISTS(SELECT 1 FROM referral_campaign_settings WHERE enabled) THEN RETURN NEW; END IF;
    IF TG_OP='UPDATE' AND OLD.character_id IS NOT NULL THEN RETURN NEW; END IF;
    SELECT i.* INTO invitation FROM referral_campaign_invitations i JOIN accounts a ON a.id=i.invitee_id AND account_value_eligible(a.id)
    JOIN accounts owner ON owner.id=i.inviter_id AND account_value_eligible(owner.id)
    WHERE i.invitee_id=NEW.account_id AND i.kind='returning' AND i.created_at<=to_timestamp(NEW.entered_at/1000.0) FOR UPDATE OF i;
    IF FOUND THEN
        PERFORM referral_campaign_reward('welcome:'||invitation.id,NEW.account_id,'welcome',1,NEW.character_id);
    END IF;
    RETURN NEW;
END $$;

CREATE OR REPLACE FUNCTION referral_campaign_signup(a uuid,code text) RETURNS boolean
LANGUAGE plpgsql AS $$
DECLARE owner uuid; invitation_time timestamptz; recipient text; token uuid; phase integer;
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
