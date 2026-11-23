CREATE TABLE ticket_screenshots (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    ticket_id uuid NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    mime_type text NOT NULL CHECK (mime_type IN ('image/png', 'image/jpeg', 'image/webp')),
    content bytea NOT NULL CHECK (octet_length(content) BETWEEN 1 AND 8388608)
);

CREATE INDEX ticket_screenshots_ticket ON ticket_screenshots(ticket_id, id);
