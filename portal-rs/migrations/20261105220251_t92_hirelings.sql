-- Platform intents only. No game rewards or store debits are executed here.
CREATE TABLE hirelings (
 id uuid PRIMARY KEY,
 account_id uuid NOT NULL,
 house_id uuid NOT NULL,
 entitlement_id uuid NOT NULL UNIQUE,
 profession text NOT NULL CHECK (profession IN ('miner','herbalist','fisher','merchant')),
 level integer NOT NULL DEFAULT 1 CHECK(level > 0),
 slot smallint NOT NULL CHECK(slot BETWEEN 1 AND 3),
 created_at timestamptz NOT NULL DEFAULT now(),
 updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(account_id,slot)
);
CREATE TABLE hireling_expeditions (
 id uuid PRIMARY KEY,
 account_id uuid NOT NULL,
 hireling_id uuid NOT NULL REFERENCES hirelings(id),
 destination text NOT NULL,
 reward_table jsonb NOT NULL,
 duration_hours integer NOT NULL CHECK(duration_hours BETWEEN 1 AND 12),
 state text NOT NULL CHECK(state IN ('pending','running','returned','claimed','refused')),
 receipt_id uuid UNIQUE,
 started_at timestamptz,
 ends_at timestamptz,
 created_at timestamptz NOT NULL DEFAULT now(),
 updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX hireling_one_active ON hireling_expeditions(hireling_id) WHERE state IN ('pending','running','returned');
CREATE TABLE hireling_commands (
 id uuid PRIMARY KEY,
 account_id uuid NOT NULL,
 expedition_id uuid NOT NULL REFERENCES hireling_expeditions(id),
 kind text NOT NULL CHECK(kind IN ('start','claim')),
 created_at timestamptz NOT NULL DEFAULT now(),
 updated_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(expedition_id,kind)
);
CREATE TABLE hireling_weekly_limits (
 account_id uuid NOT NULL,
 week date NOT NULL,
 reserved integer NOT NULL CHECK(reserved >= 0),
 created_at timestamptz NOT NULL DEFAULT now(),
 updated_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(account_id,week)
);
