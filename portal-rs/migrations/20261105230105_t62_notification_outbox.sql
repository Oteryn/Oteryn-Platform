-- Platform transition notifications only. No Game command or network dispatch.
CREATE TABLE bazaar_notification_outbox (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    transition_id uuid NOT NULL CHECK (transition_id <> '00000000-0000-0000-0000-000000000000'),
    listing_id uuid NOT NULL CHECK (listing_id <> '00000000-0000-0000-0000-000000000000'),
    recipient_id uuid NOT NULL CHECK (recipient_id <> '00000000-0000-0000-0000-000000000000'),
    evidence_id uuid NOT NULL CHECK (evidence_id <> '00000000-0000-0000-0000-000000000000'),
    kind text NOT NULL CHECK (kind IN ('outbid','new-offer','ending-soon','won','matching-search')),
    state text NOT NULL DEFAULT 'pending' CHECK (state IN ('pending','delivered','suppressed')),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (transition_id,recipient_id,kind)
);
CREATE INDEX bazaar_notification_pending ON bazaar_notification_outbox(created_at,id)
    WHERE state='pending';
