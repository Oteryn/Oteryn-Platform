-- Shared acquisition/spending eligibility. Lock identity before monetary rows;
-- safe returns intentionally use their separate, narrower identity gate.
CREATE FUNCTION account_value_restriction(target uuid) RETURNS text LANGUAGE plpgsql AS $$
DECLARE reason text;
BEGIN
    -- Trigger callers can already hold their actor row out of UUID order.
    -- Match referral preparation: retry rather than wait on another identity.
    IF pg_trigger_depth()>0 THEN
        PERFORM id FROM accounts WHERE id=target FOR SHARE NOWAIT;
    ELSE
        PERFORM id FROM accounts WHERE id=target FOR SHARE;
    END IF;
    IF NOT FOUND THEN RETURN 'account_missing'; END IF;
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
                 END INTO reason FROM accounts a WHERE a.id=target;
    RETURN reason;
EXCEPTION WHEN lock_not_available THEN
    RAISE EXCEPTION 'value eligibility conflicts with caller locks; retry transaction' USING ERRCODE='40001';
END $$;
CREATE FUNCTION account_value_eligible(target uuid) RETURNS boolean LANGUAGE sql AS $$
    SELECT account_value_restriction(target) IS NULL
$$;

ALTER TABLE wallet_operations DROP CONSTRAINT wallet_operations_kind_check;
ALTER TABLE wallet_operations ADD CONSTRAINT wallet_operations_kind_check CHECK
(kind IN ('transfer','adjustment','reserve','release','capture','store_purchase','top_up','reward','staff_grant','settlement','debt_settlement','pass_purchase','card_set_purchase'));

CREATE OR REPLACE FUNCTION referral_campaign_reward(k text,a uuid,t text,n integer,c uuid DEFAULT NULL,e timestamptz DEFAULT NULL)
RETURNS void LANGUAGE plpgsql AS $$
DECLARE reward uuid;
BEGIN
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
        IF EXISTS(WITH RECURSIVE edges AS (
            SELECT inviter_id,invitee_id FROM referral_invitations UNION
            SELECT inviter_id,invitee_id FROM referral_campaign_invitations),
            ancestors(id) AS (SELECT owner UNION SELECT e.inviter_id FROM edges e JOIN ancestors p ON e.invitee_id=p.id)
            SELECT 1 FROM ancestors WHERE id=a)
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
