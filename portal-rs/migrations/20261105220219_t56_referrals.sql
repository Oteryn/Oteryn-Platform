-- Each invited account can participate once, across new and returning campaigns.
CREATE TABLE referral_invitations (
    id uuid PRIMARY KEY,
    inviter_id uuid NOT NULL,
    invitee_id uuid NOT NULL UNIQUE,
    kind text NOT NULL CHECK (kind IN ('new', 'returning')),
    expires_at timestamptz NOT NULL,
    reward_amount bigint NOT NULL CHECK (reward_amount > 0),
    reward_actor uuid NOT NULL,
    reward_key uuid NOT NULL UNIQUE,
    claimed_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (inviter_id <> invitee_id),
    CHECK (expires_at > created_at)
);
CREATE INDEX referral_inviter_statistics ON referral_invitations(inviter_id);
