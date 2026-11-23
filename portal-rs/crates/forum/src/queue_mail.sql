WITH scheduled AS (
    INSERT INTO forum_mail_batches(thread_id,account_id,window_starts,email_id,visibility)
    SELECT s.thread_id,s.account_id,date_trunc('minute',$3::timestamptz),uuidv7(),$4
    FROM forum_subscriptions s
    JOIN accounts a ON a.id=s.account_id AND a.status='active' AND a.email_verified_at IS NOT NULL
    JOIN account_notification_prefs p ON p.account_id=s.account_id AND p.category='forum' AND p.email_enabled
    WHERE s.thread_id=$1 AND s.account_id<>$2 AND s.subscribed_at<=$3
    ON CONFLICT (thread_id,account_id,window_starts) DO NOTHING
    RETURNING account_id,email_id,window_starts
), emails AS (
    INSERT INTO email_outbox(id,account_id,category,recipient,subject,text_body,html_body)
    SELECT s.email_id,s.account_id,'forum',a.email,
           CASE a.locale WHEN 'pl' THEN $5 ELSE $8 END,
           CASE a.locale WHEN 'pl' THEN $6 ELSE $9 END,
           CASE a.locale WHEN 'pl' THEN $7 ELSE $10 END
    FROM scheduled s JOIN accounts a ON a.id=s.account_id
    RETURNING id
)
INSERT INTO outbox(kind,payload,idempotency_key,run_after,max_attempts)
SELECT 'notify.email',to_jsonb(emails.id::text),emails.id::text,s.window_starts+interval '1 minute',5
FROM emails JOIN scheduled s ON s.email_id=emails.id
