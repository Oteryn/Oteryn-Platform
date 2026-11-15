-- Bounded rolling windows shared by all portal instances.
CREATE TABLE community_ingest_budgets (
 publisher bytea NOT NULL,
 route text NOT NULL,
 admitted_at double precision[] NOT NULL DEFAULT '{}',
 PRIMARY KEY (publisher, route)
);
