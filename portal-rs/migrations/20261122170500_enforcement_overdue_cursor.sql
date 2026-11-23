-- Receipt monitoring progress is independent of command and result state.
CREATE TABLE support_enforcement_overdue_cursor (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    position bigint NOT NULL DEFAULT 0 CHECK (position >= 0)
);
INSERT INTO support_enforcement_overdue_cursor DEFAULT VALUES;
