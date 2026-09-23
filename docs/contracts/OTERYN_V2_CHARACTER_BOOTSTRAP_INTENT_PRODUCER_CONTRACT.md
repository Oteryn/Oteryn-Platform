# Oteryn v2 Character bootstrap-intent producer contract

## Authority and scope

Platform is the sole issuer of the bounded `OPERATOR_CONTROL_PLANE_BOOTSTRAP` authorization intent. The operator command accepts a Platform Identity database identifier only as lookup input and binds the intent to that Identity's immutable persisted canonical `AccountId`. It never allocates a CharacterId or mutates Game or Canary.

The authority namespace is dedicated to Character bootstrap intents. Its positive `source_revision` sequence and retained high-water file are not account-security evidence or signing-trust state. Issuance requires an explicit technical TTL of 1–3600 seconds and configured issuer authority. Activation of the private read endpoint is separate and defaults off.

## Issuance and reconciliation

`operation_id` is a canonical lower-case UUID and an idempotency/correlation identifier, not a credential. One operation binds forever to the canonical AccountId, target world, variant, operation and all four interpretation revisions. An exact retry returns the original decision and timestamps. Any changed binding fails closed. Disabled, terminated, missing or ambiguous Platform Identity state cannot issue.

A committed issuance contains exactly:

- `contract_version = 1`;
- `variant = OPERATOR_CONTROL_PLANE_BOOTSTRAP`;
- bounded `issuer_authority` and canonical UUID `issuer_decision_id`;
- positive comparable `source_revision`;
- canonical UUID `operation_id` and `operation = INITIAL_CHARACTER_BOOTSTRAP`;
- canonical UUID `account_id` and bounded `target_world_id`;
- `interpretation_context` containing positive decimal `profile_revision`, `ruleset_revision`, `content_revision`, and `starter_template_revision`;
- decimal Unix-source-time `issued_at_source` and later `expires_at_source`;
- `audience = OTERYN_GAME_CHARACTER_AUTHORITY`.

The database authority counter and retained filesystem high-water must agree before issue or read. A lower database revision, missing retained history, or any disagreement is ambiguous and fails closed. Equal operation identity can name only its original immutable decision.

## Private consumer boundary

`POST /internal/v1/game-auth/character-bootstrap-intents/reconcile` accepts only an `application/json` object, at most 512 bytes, with members in canonical order: `contract_version`, `operation_id`, `audience`. Unknown, missing, duplicate, nested, unsupported or unbounded input is rejected. Reads never mint an intent.

The route is inactive unless explicitly enabled and requires the separately configured exact TLS 1.3 verified-client subject. Missing or wrong peer provenance is rejected. Only an existing, non-expired, non-future, ordering-consistent intent is returned. Responses are private/no-store and contain no credential or secret.
