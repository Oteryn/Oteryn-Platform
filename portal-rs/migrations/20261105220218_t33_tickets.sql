CREATE TABLE tickets (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    account_id uuid NOT NULL REFERENCES accounts(id),
    request_key uuid NOT NULL,
    category text NOT NULL CHECK (category IN ('account','character','technical','payment','other')),
    subject text NOT NULL CHECK (length(trim(subject)) BETWEEN 1 AND 160),
    status text NOT NULL DEFAULT 'open' CHECK (status IN ('open','waiting_user','waiting_staff','resolved','closed')),
    lock_version bigint NOT NULL DEFAULT 1 CHECK (lock_version > 0),
    unread_reply boolean NOT NULL DEFAULT false,
    last_message_at timestamptz NOT NULL DEFAULT now(),
    closed_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (account_id, request_key)
);
CREATE INDEX tickets_account_status ON tickets(account_id,status);
CREATE INDEX tickets_queue ON tickets(last_message_at DESC,id DESC);
CREATE TABLE ticket_messages (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    ticket_id uuid NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    author_account_id uuid NOT NULL REFERENCES accounts(id),
    author_kind text NOT NULL CHECK (author_kind IN ('user','staff')),
    internal boolean NOT NULL DEFAULT false,
    body text NOT NULL CHECK (length(trim(body)) BETWEEN 1 AND 8000),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (NOT internal OR author_kind='staff')
);
CREATE INDEX ticket_messages_ticket ON ticket_messages(ticket_id,id);
