-- Keep the original request independently of the current moderation hold.
CREATE TABLE bazaar_suspension_requests (
    id uuid PRIMARY KEY,
    actor uuid NOT NULL,
    listing_id uuid NOT NULL REFERENCES bazaar_listings(id),
    reason text NOT NULL CHECK (char_length(reason) BETWEEN 10 AND 500)
);
