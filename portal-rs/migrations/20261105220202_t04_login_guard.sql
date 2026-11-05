-- Importers and MFA enrollment must preserve/set this guard before enabling login.
-- T04 refuses protected accounts until the MFA challenge flow is available.
ALTER TABLE accounts ADD COLUMN login_mfa_required boolean NOT NULL DEFAULT false;
