-- Lets the worker prune expired ticket budget windows in bounded batches.
CREATE INDEX game_ticket_budgets_expiry ON game_ticket_budgets (window_start);
