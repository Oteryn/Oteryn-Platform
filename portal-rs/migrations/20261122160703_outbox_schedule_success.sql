-- Keep one completion timestamp per scheduled job kind after slot pruning.
CREATE TABLE outbox_schedule_success (
    kind text PRIMARY KEY,
    completed_at timestamptz NOT NULL
);

INSERT INTO outbox_schedule_success (kind, completed_at)
SELECT kind, max(updated_at)
FROM outbox
WHERE state = 'done' AND schedule_slot IS NOT NULL
GROUP BY kind;
