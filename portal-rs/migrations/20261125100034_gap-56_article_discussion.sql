CREATE TABLE news_discussion_replies (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    news_id uuid REFERENCES news(id) ON DELETE SET NULL,
    author_id uuid NOT NULL REFERENCES accounts(id),
    body text NOT NULL CHECK (length(btrim(body)) BETWEEN 1 AND 2000),
    status text NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'published', 'rejected', 'hidden')),
    revision bigint NOT NULL DEFAULT 1 CHECK (revision > 0),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);

CREATE INDEX news_discussion_thread ON news_discussion_replies(news_id, id DESC);
CREATE INDEX news_discussion_quota ON news_discussion_replies(author_id, created_at DESC);
CREATE INDEX news_discussion_queue ON news_discussion_replies(status, id);
CREATE INDEX news_discussion_audit_history ON audit_log(target, id DESC)
    WHERE action = 'news.discussion.moderated';
