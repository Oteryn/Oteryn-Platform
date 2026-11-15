-- Each hold instance survives as an immutable release receipt, including retries.
ALTER TABLE coin_purchase_holds ADD COLUMN hold_id uuid NOT NULL DEFAULT uuidv7();
CREATE UNIQUE INDEX coin_purchase_holds_identity ON coin_purchase_holds(hold_id);
ALTER TABLE coin_purchase_hold_releases ADD COLUMN hold_id uuid NOT NULL DEFAULT uuidv7();
CREATE UNIQUE INDEX coin_purchase_hold_releases_identity ON coin_purchase_hold_releases(hold_id);

-- Support bounded holder scans for last-administrator protection.
CREATE INDEX account_roles_role_account ON account_roles(role_id,account_id);
