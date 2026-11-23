-- Character enforcement is scoped to Game and does not restrict the account.
CREATE OR REPLACE FUNCTION account_value_restriction(target uuid) RETURNS text LANGUAGE plpgsql AS $$
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
                   WHEN EXISTS(SELECT 1 FROM sanctions s WHERE s.account_id=a.id AND s.enforcement_profile IS NULL AND s.status='active'
                     AND s.category IN ('restriction','suspension') AND s.effective_at<=clock_timestamp()
                     AND (s.expires_at IS NULL OR s.expires_at>clock_timestamp())) THEN 'sanction'
                   WHEN EXISTS(SELECT 1 FROM sanctions s WHERE s.account_id=a.id AND s.enforcement_profile IS NULL
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

CREATE OR REPLACE FUNCTION community_invitation_delivery_eligible(target uuid) RETURNS boolean
LANGUAGE sql AS $$
    SELECT CASE
                   WHEN a.status<>'active' THEN 'account_'||a.status
                   WHEN a.email_verified_at IS NULL THEN 'email_unverified'
                   WHEN a.email_change_blocked_until>clock_timestamp() THEN 'email_change_lock'
                   WHEN EXISTS(SELECT 1 FROM account_terminations t WHERE t.account_id=a.id
                     AND t.cancelled_at IS NULL AND t.finalized_at IS NULL) THEN 'termination_pending'
                   WHEN EXISTS(SELECT 1 FROM sanctions s WHERE s.account_id=a.id AND s.enforcement_profile IS NULL AND s.status='active'
                     AND s.category IN ('restriction','suspension') AND s.effective_at<=clock_timestamp()
                     AND (s.expires_at IS NULL OR s.expires_at>clock_timestamp())) THEN 'sanction'
                   WHEN EXISTS(SELECT 1 FROM sanctions s WHERE s.account_id=a.id AND s.enforcement_profile IS NULL
                     AND (s.category NOT IN ('warning','restriction','suspension')
                     OR s.status NOT IN ('pending','active','expired','revoked'))) THEN 'sanction_unknown'
                   WHEN EXISTS(SELECT 1 FROM (SELECT DISTINCT ON (r.sanction_id) r.active,r.expires_at
                     FROM support_sanction_revisions r JOIN support_sanction_identities i USING(sanction_id)
                     WHERE r.account_id=a.id AND i.kind='ban' ORDER BY r.sanction_id,r.revision DESC) latest
                     WHERE latest.active AND (latest.expires_at IS NULL OR latest.expires_at>clock_timestamp()))
                     THEN 'ban'
                 END IS NULL FROM accounts a WHERE a.id=target;
$$;
