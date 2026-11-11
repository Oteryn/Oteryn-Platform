-- Activation requires both the deployment flag and this explicit database switch.
CREATE TABLE referral_campaign_settings (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    enabled boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
INSERT INTO referral_campaign_settings DEFAULT VALUES;
CREATE TABLE referral_links (
    account_id uuid PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
    code uuid NOT NULL UNIQUE DEFAULT uuidv7(),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE referral_email_invitations (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    inviter_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    recipient text NOT NULL,
    kind text NOT NULL CHECK (kind IN ('new','returning')),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE(inviter_id,recipient,kind)
);
CREATE TABLE referral_email_attempts (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    invitation_id uuid NOT NULL REFERENCES referral_email_invitations(id),
    inviter_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    delivered_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX referral_email_budget ON referral_email_attempts(inviter_id,created_at);
CREATE TABLE referral_campaign_invitations (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    inviter_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    invitee_id uuid NOT NULL UNIQUE REFERENCES accounts(id) ON DELETE CASCADE,
    kind text NOT NULL CHECK (kind IN ('new','returning')),
    first_paid_at timestamptz,
    premium_points integer NOT NULL DEFAULT 0 CHECK (premium_points >= 0),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (inviter_id <> invitee_id)
);
CREATE INDEX referral_campaign_inviter ON referral_campaign_invitations(inviter_id,created_at);
CREATE TABLE referral_campaign_rewards (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    operation_key text NOT NULL UNIQUE,
    account_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    kind text NOT NULL CHECK (kind IN ('starter','outfit','addon1','addon2','welcome','premium')),
    amount integer NOT NULL CHECK (amount > 0),
    character_id uuid,
    expires_at timestamptz,
    applied_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE referral_loyalty_credits (
    operation_key text PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    points integer NOT NULL CHECK (points > 0),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
-- Game entitlements are held in the existing outbox until the proposed game
-- delivery contract is approved. They cannot be mistaken for delivered value.
CREATE FUNCTION referral_campaign_reward(k text,a uuid,t text,n integer,c uuid DEFAULT NULL,e timestamptz DEFAULT NULL)
RETURNS void LANGUAGE plpgsql AS $$
DECLARE reward uuid;
BEGIN
    INSERT INTO referral_campaign_rewards(operation_key,account_id,kind,amount,character_id,expires_at)
    VALUES(k,a,t,n,c,e) ON CONFLICT(operation_key) DO NOTHING RETURNING id INTO reward;
    IF reward IS NOT NULL AND t <> 'premium' THEN
        INSERT INTO outbox(kind,idempotency_key,payload,run_after)
        VALUES('referral.entitlement',k,jsonb_build_object('entitlement_id',reward,'account_id',a,'kind',t,'quantity',n,'character_scope',CASE WHEN t='starter' THEN 'FIRST_THREE_CREATED' WHEN t='welcome' THEN 'EXACT_CHARACTER' ELSE 'ACCOUNT' END,'character_id',c,'expires_at',e),'infinity');
    END IF;
END $$;
-- Called only by the committed store purchase path, in its transaction.
CREATE FUNCTION referral_campaign_purchase(a uuid,days integer,paid_at timestamptz)
RETURNS void LANGUAGE plpgsql AS $$
DECLARE invitation referral_campaign_invitations; paying bigint; threshold integer;
BEGIN
    IF NOT EXISTS(SELECT 1 FROM referral_campaign_settings WHERE enabled) THEN RETURN; END IF;
    PERFORM pg_advisory_xact_lock(5600);
    SELECT i.* INTO invitation FROM referral_campaign_invitations i
      JOIN accounts buyer ON buyer.id=i.invitee_id AND buyer.status='active' AND buyer.email_verified_at IS NOT NULL AND buyer.email_change_blocked_until<=clock_timestamp()
      JOIN accounts owner ON owner.id=i.inviter_id AND owner.status='active' AND owner.email_verified_at IS NOT NULL AND owner.email_change_blocked_until<=clock_timestamp()
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
-- Premium-only points are supplied before referral credits are added, so credits
-- cannot recursively earn further credits. High-water marks retain remainders.
CREATE FUNCTION referral_campaign_loyalty(a uuid,earned integer)
RETURNS void LANGUAGE plpgsql AS $$
DECLARE invitation referral_campaign_invitations; delta integer;
BEGIN
    IF NOT EXISTS(SELECT 1 FROM referral_campaign_settings WHERE enabled) THEN RETURN; END IF;
    PERFORM pg_advisory_xact_lock(5600);
    SELECT i.* INTO invitation FROM referral_campaign_invitations i
      JOIN accounts buyer ON buyer.id=i.invitee_id AND buyer.status='active' AND buyer.email_verified_at IS NOT NULL AND buyer.email_change_blocked_until<=clock_timestamp()
      JOIN accounts owner ON owner.id=i.inviter_id AND owner.status='active' AND owner.email_verified_at IS NOT NULL AND owner.email_change_blocked_until<=clock_timestamp()
      WHERE i.invitee_id=a AND i.kind='new' FOR UPDATE OF i;
    IF NOT FOUND OR earned<=invitation.premium_points THEN RETURN; END IF;
    delta := earned/6-invitation.premium_points/6;
    IF delta>0 THEN
        INSERT INTO referral_loyalty_credits(operation_key,account_id,points)
        VALUES('loyalty:'||invitation.id||':'||(earned/6),invitation.inviter_id,delta) ON CONFLICT DO NOTHING;
    END IF;
    UPDATE referral_campaign_invitations SET premium_points=earned,updated_at=clock_timestamp() WHERE id=invitation.id;
END $$;
CREATE FUNCTION referral_campaign_login() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE invitation referral_campaign_invitations;
BEGIN
    IF NEW.character_id IS NULL OR NOT EXISTS(SELECT 1 FROM referral_campaign_settings WHERE enabled) THEN RETURN NEW; END IF;
    IF TG_OP='UPDATE' AND OLD.character_id IS NOT NULL THEN RETURN NEW; END IF;
    SELECT i.* INTO invitation FROM referral_campaign_invitations i JOIN accounts a ON a.id=i.invitee_id AND a.status='active' AND a.email_verified_at IS NOT NULL AND a.email_change_blocked_until<=clock_timestamp()
    JOIN accounts owner ON owner.id=i.inviter_id AND owner.status='active' AND owner.email_verified_at IS NOT NULL AND owner.email_change_blocked_until<=clock_timestamp()
    WHERE i.invitee_id=NEW.account_id AND i.kind='returning' AND i.created_at<=to_timestamp(NEW.entered_at/1000.0) FOR UPDATE OF i;
    IF FOUND THEN
        PERFORM referral_campaign_reward('welcome:'||invitation.id,NEW.account_id,'welcome',1,NEW.character_id);
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER referral_campaign_login AFTER INSERT OR UPDATE OF character_id ON pd_game_logins FOR EACH ROW EXECUTE FUNCTION referral_campaign_login();
-- Preserve activity high-water marks after ordinary session/history retention.
CREATE TABLE referral_activity (
    account_id uuid PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
    last_activity timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
INSERT INTO referral_activity(account_id,last_activity)
SELECT a.id,GREATEST(a.created_at,
    COALESCE((SELECT max(created_at) FROM security_events WHERE account_id=a.id AND kind='login_succeeded'),a.created_at),
    COALESCE((SELECT max(last_seen_at) FROM web_sessions WHERE account_id=a.id),a.created_at),
    COALESCE((SELECT to_timestamp(max(entered_at)/1000.0) FROM pd_game_logins WHERE account_id=a.id),a.created_at)) FROM accounts a;
CREATE FUNCTION referral_activity_capture() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE a uuid; stamp timestamptz;
BEGIN
    IF TG_TABLE_NAME='accounts' THEN a:=NEW.id; stamp:=NEW.created_at;
    ELSIF TG_TABLE_NAME='pd_game_logins' THEN a:=NEW.account_id; stamp:=to_timestamp(NEW.entered_at/1000.0);
    ELSIF TG_TABLE_NAME='security_events' THEN
        IF NEW.kind<>'login_succeeded' THEN RETURN NEW; END IF;
        a:=NEW.account_id; stamp:=NEW.created_at;
    ELSE a:=NEW.account_id; stamp:=NEW.last_seen_at;
    END IF;
    IF a IS NOT NULL AND EXISTS(SELECT 1 FROM accounts WHERE id=a) THEN
        INSERT INTO referral_activity(account_id,last_activity) VALUES(a,stamp)
        ON CONFLICT(account_id) DO UPDATE SET last_activity=GREATEST(referral_activity.last_activity,EXCLUDED.last_activity),updated_at=clock_timestamp();
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER referral_activity_account AFTER INSERT ON accounts FOR EACH ROW EXECUTE FUNCTION referral_activity_capture();
CREATE TRIGGER referral_activity_web AFTER INSERT OR UPDATE ON web_sessions FOR EACH ROW EXECUTE FUNCTION referral_activity_capture();
CREATE TRIGGER referral_activity_security AFTER INSERT ON security_events FOR EACH ROW EXECUTE FUNCTION referral_activity_capture();
CREATE TRIGGER referral_activity_game AFTER INSERT ON pd_game_logins FOR EACH ROW EXECUTE FUNCTION referral_activity_capture();
