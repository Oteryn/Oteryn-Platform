use oteryn_db::outbox::{self, PgPool, Uuid};

/// Recheck current delivery consent, account state, and the batch's visible replies.
pub(super) async fn deliverable(pool: &PgPool, email: Uuid) -> Result<bool, outbox::Error> {
    outbox::query_scalar(
        "SELECT EXISTS(
             SELECT 1 FROM forum_mail_batches m
             JOIN forum_subscriptions s USING(thread_id,account_id)
             JOIN accounts a ON a.id=m.account_id AND a.status='active' AND a.email_verified_at IS NOT NULL
             JOIN forum_threads t ON t.id=m.thread_id AND t.deleted_at IS NULL
             JOIN forum_boards b ON b.id=t.board_id AND b.visibility=m.visibility
             WHERE m.email_id=$1
               AND EXISTS (SELECT 1 FROM forum_posts p WHERE p.thread_id=t.id
                 AND p.created_at>=greatest(m.window_starts,s.subscribed_at)
                 AND p.created_at<m.window_starts+interval '1 minute'
                 AND p.deleted_at IS NULL AND p.author_id<>m.account_id)
               AND (b.visibility<>'staff' OR (
                 SELECT count(DISTINCT rp.permission)=2
                 FROM account_roles ar JOIN role_permissions rp USING(role_id)
                 WHERE ar.account_id=m.account_id AND rp.permission IN ('admin.access','forum.moderate')
               ))
         )",
    ).bind(email).fetch_one(pool).await
}
