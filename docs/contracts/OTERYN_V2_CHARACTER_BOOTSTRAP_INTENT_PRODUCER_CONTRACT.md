# Oteryn-v2 Character bootstrap-intent producer contract

## Status and boundary

This contract defines the Platform-owned v1 producer and private reconciliation read for the bounded `OPERATOR_CONTROL_PLANE_BOOTSTRAP` variant. It does not authorize a browser flow, allocate `CharacterId`, mutate Game or Canary, select product naming/quota policy, or replace account-security evidence.

Platform resolves the authoritative subject from a persisted Identity primary-key lookup. The issuer has no `account_id` input: a caller-provided AccountId, operation identity, world identifier or interpretation revision is never authentication or authorization. Issuance rejects a missing, disabled, terminated, null-AccountId or non-canonical Identity.

## Issuance and immutable persistence

`game-auth:character-bootstrap-intent:issue` requires:

- one positive persisted Platform `identity-id` lookup;
- canonical lower-case UUID `operation-id` and `target-world-id` values;
- bounded `profile`, `ruleset`, `content` and `starter-template` revision strings.

The Platform serializes issuance through the singleton row in the dedicated `character_bootstrap_intent_authority` namespace. A new operation receives one positive, globally comparable source revision and one immutable lower-case UUID decision identity. An exact retry returns the stored JSON decision without changing source time, expiry, decision identity or revision. Reuse of the operation UUID with any changed Identity, AccountId, world or interpretation binding fails closed.

The additive intent row persists the complete binding and exact encoded decision. Deleting or changing issued rows is outside this interface. The authority row binds its high-water revision to the decision identity at that revision; a lower-than-row high water or an equal revision with a different decision identity is unavailable. The Game consumer must retain a high-water mark **only for this issuer authority namespace** and reject a lower source revision or an equal revision carrying another decision identity. This namespace is not account-security or signing-trust authority.

Database rollback that removes the authority row, lowers it beneath a retained row, or conflicts at the equal revision is explicitly detected and fails closed. Simultaneous rollback of all Platform database copies below all retained history is not recoverable by this database-owned mechanism and must not be represented as accepted recovery; restored service requires reconciliation against an independently retained consumer high water. No generic witness framework or NativeEvidence witness namespace is introduced.

## Technical expiry configuration

`GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_TTL_SECONDS` is required for issuance and has no default. It is a technical transport/reconciliation interval from 1 through 300 seconds, not product policy. Missing, malformed, zero, negative or larger values fail closed. Source times are non-negative Unix seconds encoded as canonical decimal strings.

## Private reconciliation request

`POST /internal/v1/game-auth/character-bootstrap-intents/read` accepts only this bounded JSON object (object member order is insignificant):

```json
{"contract_version":1,"operation_id":"01890f4e-7c00-7000-8000-000000000001"}
```

The body is at most 256 bytes. Duplicate, unknown, missing, nested, non-scalar, unsupported-version, upper-case/non-canonical UUID or malformed JSON input is rejected. The read never issues an intent. Unknown and expired intents are not served as current.

The endpoint requires a successful TLS 1.3 client-certificate verification provenance supplied by the trusted terminator and an exact subject match to the separately configured `GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_MTLS_CLIENT_IDENTITY`. Missing/malformed configuration is unavailable; missing/wrong provenance is unauthorized. The raw credential and private secret material are neither accepted in JSON nor persisted or emitted. Every result is `no-store` and `private`.

## Exact v1 success wire

A successful response contains exactly these semantic fields:

```text
contract_version = 1
variant = OPERATOR_CONTROL_PLANE_BOOTSTRAP
issuer_authority = OTERYN_PLATFORM_CHARACTER_AUTHORITY
issuer_decision_id
source_revision
operation_id
operation = INITIAL_CHARACTER_BOOTSTRAP
account_id
target_world_id
interpretation_context
  profile_revision
  ruleset_revision
  content_revision
  starter_template_revision
issued_at_source
expires_at_source
audience = OTERYN_GAME_CHARACTER_AUTHORITY
```

The response is at most 4096 bytes. UUIDs are canonical lower-case. Revisions are 1–128 bytes and match `[A-Za-z0-9][A-Za-z0-9._:-]*`. `source_revision`, `issued_at_source` and `expires_at_source` are canonical positive/non-negative decimal strings suitable for integer comparison, not lexical comparison.

## Fail-closed outcomes

- malformed request: empty `400`;
- missing/wrong TLS peer provenance: empty `401`;
- unknown or expired operation: empty `404`;
- unavailable database, invalid durable state, rollback or equal-revision decision conflict: empty `503`.

Transport failure bodies carry no credentials, intent binding or private diagnostics. NativeEvidence remains its separate four-operation contract and is not extended by this endpoint.
