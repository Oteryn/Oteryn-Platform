# Oteryn-v2 Character bootstrap-intent producer contract

## Status and boundary

This contract defines the Platform-owned producer (contract version 4 since 2026-11-17; versions 1, 2 and 3 are retired) and private reconciliation read for the bounded `OPERATOR_CONTROL_PLANE_BOOTSTRAP` variant. It does not authorize a browser flow, allocate `CharacterId`, mutate Game or legacy engine, select product naming/quota policy, or replace account-security evidence.

Platform resolves the authoritative subject from a persisted Identity primary-key lookup. The issuer has no `account_id` input: a caller-provided AccountId, operation identity, world identifier or interpretation revision is never authentication or authorization. Issuance rejects a missing, disabled, terminated, null-AccountId or non-canonical Identity.

## Issuance and immutable persistence

`game-auth:character-bootstrap-intent:issue` requires:

- one positive persisted Platform `identity-id` lookup;
- canonical lower-case UUID `operation-id` and `target-world-id` values;
- one `requested-name` under Game naming policy revision 1: 2..29 ASCII letters in words joined by single spaces (Game `CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md` §6.1). Platform checks syntax only; Game Character Authority alone decides uniqueness and reserves the name, and a taken name is refused at Game bootstrap, never by Platform;
- explicit `requested-sex`, exactly the case-sensitive JSON string `Female` or `Male` (Game APP-3); no numeric encoding, default, inference or case normalization;
- explicit `requested-vocation`, exactly one of the case-sensitive JSON strings `Knight`, `Paladin`, `Sorcerer`, `Druid` or `Monk` (version 4); no numeric encoding, `None`, default, inference or case normalization;
- bounded `profile`, `ruleset`, `content` and `starter-template` revision strings.

The Platform serializes issuance through the singleton row in the dedicated `character_bootstrap_intent_authority` namespace. A new operation receives one positive, globally comparable source revision and one immutable lower-case UUID decision identity. An exact retry returns the stored JSON decision without changing source time, expiry, decision identity or revision. Reuse of the operation UUID with any changed Identity, AccountId, world, requested name, requested sex, requested vocation or interpretation binding fails closed. Stored version 1, 2 and 3 intents are no longer current: their reads and retries fail closed. The additive nullable `requested_sex` and `requested_vocation` columns leave historical rows unchanged; no sex or vocation is backfilled. New issuance validates sex and vocation before any transaction or source-revision allocation, persists each in both its binding column and the hashed JSON, and reads verify their exact equality.

The additive intent row persists the complete binding and exact encoded decision. Deleting or changing issued rows is outside this interface. The authority row binds its high-water revision to the decision identity at that revision; a lower-than-row high water or an equal revision with a different decision identity is unavailable. The Game consumer must retain a high-water mark **only for this issuer authority namespace** and reject a lower source revision or an equal revision carrying another decision identity. This namespace is not account-security or signing-trust authority.

Database rollback that removes the authority row, lowers it beneath a retained row, or conflicts at the equal revision is explicitly detected and fails closed. Simultaneous rollback of all Platform database copies below all retained history is not recoverable by this database-owned mechanism and must not be represented as accepted recovery; restored service requires reconciliation against an independently retained consumer high water. No generic witness framework or NativeEvidence witness namespace is introduced.

## Operator command

`oteryn-portal bootstrap-intent issue --actor <uuidv7> [--input <file>]` is the portal-rs invocation of `game-auth:character-bootstrap-intent:issue`. It reads one flat JSON object (at most 16 KiB, duplicate or unknown members rejected) from the file or stdin with exactly these string members: `identityId` (the persisted Identity primary key, a lower-case UUIDv7), `operationId`, `targetWorldId`, `requestedName`, `requestedSex`, `requestedVocation`, `profileRevision`, `rulesetRevision`, `contentRevision` and `starterTemplateRevision`. It runs only with `GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_ENABLED=true` (default off) and the TTL below. On success it prints the stored canonical intent on stdout; an exact retry prints the same bytes. Failures print one JSON error object on stderr without configuration values or intent binding and exit `2` (bad input, including an Identity that is not active or a missing, malformed or unauthorized actor), `3` (operation conflict) or `4` (disabled, misconfigured, database or authority unavailable).

`--actor` is required whenever issuance is enabled: the lower-case UUIDv7 of an existing active staff account holding `game.player.manage`, checked inside the issuance transaction. A missing, malformed, unknown, inactive or unauthorized actor exits `2` and issues nothing. A new issuance writes exactly one Platform `audit_log` row (`character.bootstrap_intent.issued`: actor, operation id, Identity id, source revision, TTL seconds and the intent SHA-256, never the intent JSON or the requested name) in the same transaction as the intent, so neither exists without the other; an exact retry returns the stored bytes and writes no further row, and a conflict or failure writes none. The actor and the audit row are Platform-local: they change nothing in the canonical intent, the stored intent row or the private read, so there is no wire change for Game. The testing seed `seed bootstrap-intent` takes the same `requestedSex` and `requestedVocation` members, issues without an actor and writes no audit row.

## Technical expiry configuration

`GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_TTL_SECONDS` is required for issuance and has no default. It is a technical transport/reconciliation interval from 1 through 300 seconds, not product policy. Missing, malformed, zero, negative or larger values fail closed. Source times are non-negative Unix seconds encoded as canonical decimal strings.

## Private reconciliation request

`POST /internal/v1/game-auth/character-bootstrap-intents/read` accepts only this bounded JSON object (object member order is insignificant):

```json
{"contract_version":4,"operation_id":"01890f4e-7c00-7000-8000-000000000001"}
```

The body is at most 256 bytes. Duplicate, unknown, missing, nested, non-scalar, unsupported-version (including a retired version 1, 2 or 3 request), upper-case/non-canonical UUID or malformed JSON input is rejected. The read never issues an intent. Unknown and expired intents are not served as current.

The endpoint requires a successful TLS 1.3 client-certificate verification provenance supplied by the trusted terminator and an exact subject match to the separately configured `GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_MTLS_CLIENT_IDENTITY`. Missing/malformed configuration is unavailable; missing/wrong provenance is unauthorized. The raw credential and private secret material are neither accepted in JSON nor persisted or emitted. Every result is `no-store` and `private`.

## Exact version 4 success wire

A successful response contains exactly these semantic fields:

```text
contract_version = 4
variant = OPERATOR_CONTROL_PLANE_BOOTSTRAP
issuer_authority = OTERYN_PLATFORM_CHARACTER_AUTHORITY
issuer_decision_id
source_revision
operation_id
operation = INITIAL_CHARACTER_BOOTSTRAP
account_id
target_world_id
requested_name
requested_sex = Female | Male
requested_vocation = Knight | Paladin | Sorcerer | Druid | Monk
interpretation_context
  profile_revision
  ruleset_revision
  content_revision
  starter_template_revision
issued_at_source
expires_at_source
audience = OTERYN_GAME_CHARACTER_AUTHORITY
```

The response is at most 4096 bytes. UUIDs are canonical lower-case. Revisions are 1–128 bytes and match `[A-Za-z0-9][A-Za-z0-9._:-]*`. `source_revision`, `issued_at_source` and `expires_at_source` are canonical positive/non-negative decimal strings suitable for integer comparison, not lexical comparison. `requested_vocation` follows `requested_sex` directly in the canonical member order.

Byte-exact vectors, one per vocation plus rejected `knight`, `None` and missing-member bodies, are in `docs/contracts/fixtures/character-bootstrap-intent-v4/` (Newhaven is the existing `starter_template_revision` value `newhaven-starter-r1`; there is no new member).

## Version 4 consumer agreement

Version 4 was agreed with the Game consumer lane: the Game session confirmed it on 2026-11-17. At `Oteryn/Oteryn-Game@e2152d050038cecdb3728c222af2ac184e1cac5a` the consumer pins `CONTRACT_VERSION = 3` (`crates/gameplay/durability/src/character_bootstrap_intent.rs:22`) and moves to 4. Until it does, Platform answers a version 3 read request with an empty `400` and serves no intent, so a consumer still pinned to 3 fails closed rather than creating a character without its requested vocation. The producer stays behind `GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_ENABLED` (default off).

## Fail-closed outcomes

- malformed request: empty `400`;
- missing/wrong TLS peer provenance: empty `401`;
- unknown or expired operation: empty `404`;
- unavailable database, invalid durable state, rollback or equal-revision decision conflict: empty `503`.

Transport failure bodies carry no credentials, intent binding or private diagnostics. NativeEvidence remains its separate four-operation contract and is not extended by this endpoint.
