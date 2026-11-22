CREATE TABLE operations_daily_statistics (
    day date PRIMARY KEY,
    registrations bigint NOT NULL CHECK (registrations >= 0),
    logins bigint NOT NULL CHECK (logins >= 0),
    refreshed_at timestamptz NOT NULL
);
