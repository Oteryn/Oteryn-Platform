# Oteryn v2 Account Identity Boundary Contract

## Status

`ACCEPTED ARCHITECTURE CONTRACT — IMPLEMENTATION NOT YET AUTHORIZED`

This contract defines the native Oteryn-v2 account identity boundary owned by Oteryn Platform.

It is intentionally separate from current Canary-compatible account/session contracts. Existing Canary paths remain valid only within their declared legacy/current implementation scope until a separately authorized migration retires them.

## Canonical owner

Oteryn Platform Identity owns:

- canonical `AccountId` issuance and lifecycle;
- reusable credential verification and account-security policy;
- `game_auth_generation` or equivalent game-authorization revocation fence;
- Game Login Ticket issuance and atomic consumption;
- the Platform-internal mapping from local persistence identity to canonical `AccountId`;
- any legacy compatibility mapping from `AccountId`/Identity to Canary `accounts.id`.

Oteryn-v2 consumes `AccountId` as an externally owned strong identity and must not mint a competing account identifier.

## Canonical AccountId

```text
AccountId = strongly typed UUIDv7, full 128 bits
```

Required semantics:

- globally unique for one Platform Identity;
- immutable for the Identity lifetime;
- never reused for another account;
- nil/zero UUID invalid;
- all 128 bits preserved across authorized boundaries;
- canonical comparison is exact UUID equality under the `AccountId` semantic type;
- UUIDv7 ordering is not authorization, freshness, causality or fencing;
- `AccountId` is not a bearer credential and possession does not authorize reads or mutations.

## Platform-local persistence identity

Current Laravel persistence uses integer `identities.id`.

For the native target:

```text
identities.id = Platform-local persistence surrogate
AccountId     = canonical cross-boundary account identity
```

`identities.id` may continue to back internal Platform foreign keys and ORM relations.

It must not be exported as native `AccountId` merely because it is the current primary key.

Exact additive schema/backfill implementation remains a separate migration task.

## Legacy Canary anti-corruption boundary

`canary_account_id` / Canary `accounts.id` is classified as:

```text
LEGACY COMPATIBILITY / ACL IDENTIFIER
```

It may be used only by explicit Canary-facing adapters/contracts that require it.

It must not become:

- canonical native `AccountId`;
- native `CharacterId`;
- canonical gameplay `GameSessionId`;
- native protocol identity;
- public account identity;
- independent ownership evidence.

Native code and contracts must not require Canary numeric account identity when canonical `AccountId` is available.

## Native Game Login Ticket context

A native Oteryn-v2 Game Login Ticket is Platform-owned pre-admission authorization material.

Its authoritative account binding is:

```text
AccountId
```

not:

```text
canary_account_id
identities.id
email
account name
```

The ticket retains the approved security invariants from the existing Game Gateway/Identity design:

- opaque high-entropy bearer material;
- at least 256 bits CSPRNG entropy before transport encoding;
- plaintext returned only to the intended client path and never stored/logged;
- cryptographic lookup representation server-side;
- single-use atomic consume;
- short server-authoritative expiry, current policy default 60 seconds;
- audience binding to `oteryn-game-gateway`;
- binding to current `game_auth_generation` or equivalent revocation fence;
- fail closed for invalid, expired, reused, revoked, ambiguous or unavailable authoritative state.

The native ticket is authorization to attempt admission. It is not the canonical logical gameplay session and never acts as `GameSessionId`.

## Native redeem result

The target native private redeem/login context must expose the minimum account authority required by Gateway using canonical identity.

Semantic minimum only:

```text
NativeRedeemAuthorization
    AccountId
    security_generation
    redeemed_at
```

This section does not freeze a protocol version number, JSON field layout, endpoint path, transport or implementation rollout. Those remain future implementation-contract work.

A native response must not require `canary_account_id`.

A legacy Canary response may continue returning `canary_account_id` under its existing versioned compatibility contract.

## Account to character boundary

`CharacterId` remains game-domain owned.

The native cross-boundary relationship is conceptually:

```text
Platform AccountId
    -> game-domain CharacterId[]
```

Platform may consume an authorized character projection for WWW/Gateway product surfaces, but:

- Platform does not mint canonical `CharacterId`;
- Oteryn-v2 does not mint canonical `AccountId`;
- a client-supplied AccountId or CharacterId is only a claim to validate;
- projection freshness does not replace authoritative game-domain admission checks;
- final gameplay admission validates current account-character ownership and current session/lease/fencing state in the game domain.

## Trust and authorization rules

At every boundary:

- identity and authorization are separate;
- service authentication is separate from user authorization material;
- account, character, world and session identifiers are strongly typed and not interchangeable;
- no identifier is accepted as authority merely because it is hard to guess;
- mixed legacy/native identifier ambiguity fails closed;
- no password, password hash, MFA secret, recovery material or OAuth refresh credential crosses into the game runtime.

## Compatibility relationship

Current documents such as `GAME_GATEWAY_IDENTITY_CONTRACT.md`, `GAME_SESSION_CANARY_CONTRACT.md`, Canary provisioning contracts and Canary character-operation contracts remain authoritative for the legacy/current Canary compatibility slices they explicitly govern.

For native Oteryn-v2 account identity, this contract together with ADR 0028 is narrower and authoritative where historical contracts bind account authority to `canary_account_id` or expose Platform persistence IDs as cross-boundary identity.

This contract does not silently change current runtime payloads or database schema.

## Versioning and migration requirements

A later implementation must provide an explicit compatibility sequence:

1. add canonical `AccountId` storage/generation inside Platform Identity;
2. backfill existing identities additively while preserving `identities.id`;
3. expose a versioned native redeem/login context using `AccountId`;
4. retain legacy Canary-compatible versions while required;
5. update Gateway native orchestration to consume canonical `AccountId`;
6. coordinate Oteryn-v2 consumer adoption only after its foundation gates authorize implementation;
7. reject unsupported mixed identifier/version combinations;
8. prove rollback without reusing or changing issued AccountIds.

Breaking semantic changes to `AccountId` ownership, uniqueness, representation or issuer require a new accepted ADR/contract revision.

## Non-authorization

This contract authorizes no:

- Laravel migration;
- runtime code change;
- ticket/redeem API activation;
- Canary removal;
- Oteryn-v2 repository write;
- protocol-oteryn implementation;
- production rollout.

Those require separately authorized implementation tasks and their own validation/rollback evidence.
