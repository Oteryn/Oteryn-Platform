# ADR 0028 — Platform-issued UUIDv7 AccountId for native cross-boundary identity

## Status

Accepted — 2026-08-07

## Context

Oteryn Platform is already the authoritative owner of user Identity, reusable credentials, account-security policy and the Game Login Ticket lifecycle. Historical Platform integration was built around a greenfield compatibility relation to Canary and therefore many current contracts use the local Platform `identities.id` integer together with `canary_account_id` / Canary `accounts.id`.

Oteryn v2 is a separate native game domain. Its accepted foundation keeps `AccountId` owned by Oteryn Platform, `CharacterId` owned by the game-domain Character authority and the canonical gameplay `GameSessionId` owned by the game-domain Game Session / Admission authority. Oteryn v2 explicitly forbids silently re-keying externally owned Platform account identity.

The current Platform storage model uses an integer primary key for `identities.id`. That implementation detail is useful inside the Laravel bounded context but is not a good long-lived native cross-system identity contract: it exposes persistence topology, encourages accidental coupling to local foreign keys and makes later storage migration unnecessarily visible to the game domain.

A separate durable, strongly typed Platform-owned account identity is therefore required for the native Platform <-> Oteryn-v2 boundary while retaining the existing database primary key as an internal persistence surrogate.

## Decision

### 1. Canonical native AccountId

The canonical cross-boundary account identity for native Oteryn integration is:

```text
AccountId = strongly typed UUIDv7, full 128 bits
```

`AccountId` is:

- issued and owned by Oteryn Platform Identity;
- globally unique for Platform account identity;
- immutable for the lifetime of the Platform Identity;
- never reused for a different account;
- semantically opaque and independent from email, account name, Canary account number, database row position, current world, security state or entitlement state;
- preserved losslessly at every authorized Platform <-> Gateway <-> Oteryn-v2 boundary;
- not a credential, bearer capability or proof of authorization by itself.

Nil/zero UUID is invalid. UUIDv7 timestamp ordering is not authority, causality, session freshness or fencing evidence.

### 2. `identities.id` remains local persistence surrogate

The existing integer `identities.id` remains a Platform-local persistence key.

It may continue to support Laravel relations, local foreign keys and existing Platform schema without a broad re-key migration.

It is not the canonical native cross-boundary account identity and must not be introduced into new Oteryn-v2 public/service/protocol contracts as `AccountId`.

The target internal relation is conceptually:

```text
identities.id        = local Platform persistence surrogate
identities.account_id = canonical Platform AccountId (UUIDv7)
```

The exact column name, backfill, uniqueness constraint, rollout order and migration implementation are deliberately deferred to a separately authorized implementation/migration task.

### 3. No silent re-keying outside Platform

Only Platform Identity may mint canonical `AccountId` values.

Oteryn-v2, Game Gateway, game servers, clients, Canary adapters and migration tooling must not:

- derive `AccountId` from `identities.id`;
- derive `AccountId` from `canary_account_id`;
- mint replacement account UUIDs;
- treat an account email/name as account identity;
- infer ownership from equality of non-canonical identifiers.

Where an external consumer requires account identity, Platform emits the canonical `AccountId` under an explicit versioned contract.

### 4. Canary account IDs are legacy compatibility / ACL state

`canary_account_id` / Canary `accounts.id` remains valid only inside the explicit Canary compatibility and anti-corruption layer.

It is not:

- native `AccountId`;
- native character identity;
- native game-session identity;
- a public user identifier;
- proof of account ownership outside the compatibility contract.

Existing delivered Canary provisioning, character-management, Bazaar and legacy Game Session contracts may continue to use the verified Canary identifier while those compatibility paths remain supported. They must be labelled and interpreted as legacy/current implementation boundaries rather than the native Oteryn-v2 identity model.

The mapping conceptually becomes:

```text
Platform Identity
    local identities.id
    canonical AccountId
        |
        +--> native Oteryn-v2 contracts use AccountId
        |
        +--> Legacy Canary ACL mapping may resolve canary_account_id
```

Removal of the Canary mapping is not authorized by this ADR.

### 5. Native Game Login Ticket binding

The native Oteryn-v2 Game Login Ticket binds to canonical `AccountId`, not to `canary_account_id`.

All existing security properties of the approved ticket model remain required unless a later accepted security contract strengthens them, including:

- opaque high-entropy bearer material;
- single-use atomic consumption;
- short server-authoritative expiry (current policy default 60 seconds);
- audience binding to Game Gateway;
- current `game_auth_generation` or equivalent revocation fence;
- no plaintext ticket persistence or logging;
- fail-closed behavior for invalid, expired, reused, revoked or ambiguous state.

Conceptually:

```text
Native Game Login Ticket authorization context
    AccountId
    game_auth_generation
    audience
    issued_at / expires_at
    one-time consume state
```

The ticket is authorization to attempt the bounded game-admission flow. It is not `GameSessionId` and does not prove successful gameplay admission.

### 6. Character relationship remains game-owned

This ADR does not make Platform the issuer or gameplay owner of `CharacterId`.

The native account/character relationship is:

```text
Platform AccountId
    -> game-domain CharacterId[]
```

The authoritative game-domain Character authority owns `CharacterId` issuance and the authoritative current AccountId-to-CharacterId ownership binding. Platform WWW/Gateway may consume an authorized projection but must not manufacture CharacterIds or turn a stale projection into final gameplay authority.

Final game admission revalidates authoritative account/character/session/lease/fencing state according to the Oteryn-v2 admission contract.

### 7. Compatibility and rollout

This is an architecture decision, not an implementation or activation claim.

Until a separately authorized migration is implemented and proven:

- current Platform database schema remains unchanged;
- current Canary-compatible ticket/redeem/session contracts may continue to use `identity_id` and `canary_account_id` where their declared scope is explicitly Canary compatibility;
- native Oteryn-v2 integration must not copy those Canary-specific identifiers into its target contract;
- mixed-version ambiguity fails closed rather than silently translating identities.

The migration must be additive and backward compatible first. It must not require rewriting all existing Platform foreign keys solely to expose native `AccountId`.

## Relationship to earlier decisions

This ADR refines ADR 0004 and ADR 0009 for the native Oteryn-v2 boundary.

Where those earlier ADRs use `1 Platform Identity <-> 1 Canary accounts.id` or bind the Game Login Ticket to `canary_account_id`, that remains valid evidence and policy for the declared Canary compatibility model, but it no longer defines canonical native cross-boundary account identity.

All still-compatible decisions remain in force, especially:

- Platform Identity is the reusable credential authority;
- Game Gateway does not verify user passwords;
- authorization identifiers supplied by clients are never ownership proof;
- Game Login Tickets remain short-lived, one-time and fail closed;
- Canary remains the semantic owner of legacy Canary gameplay data while the compatibility path exists.

This ADR does not supersede those decisions outside the narrower identity-representation/native-boundary conflict resolved here.

## Consequences

### Positive

- the native game domain receives a stable Platform-owned account identity independent from Laravel persistence layout;
- Platform can evolve internal storage without re-keying every external game contract;
- Canary numeric IDs cannot leak into the native domain as accidental permanent identity;
- existing Laravel foreign keys do not require an unnecessary broad migration;
- AccountId, CharacterId and GameSessionId retain separate owners and lifecycle semantics;
- security-sensitive ticket and admission boundaries become easier to type-check, audit and reason about across languages.

### Costs

- Platform needs an additive UUIDv7 account-identity migration before native runtime consumption;
- compatibility adapters must maintain explicit mappings while Canary support exists;
- Gateway/private APIs require a versioned migration from Canary-specific authorization results to native AccountId results;
- cross-repository fixtures and conformance tests must prove lossless 128-bit representation and reject mixed identifier types.

## Rejected alternatives

### Use `identities.id` bigint as canonical AccountId

Rejected as the native cross-boundary contract because it exposes local persistence identity and makes future storage evolution unnecessarily visible outside Platform.

### Replace every Platform foreign key with UUIDv7 immediately

Rejected because canonical external identity does not require rewriting all internal persistence relations. The migration cost and risk would not provide corresponding product value.

### Reuse `canary_account_id` as native AccountId

Rejected because Canary account identity belongs to a legacy compatibility model and would permanently couple the native Rust game domain to Canary schema/history.

### Let Oteryn-v2 mint its own account UUID and map it to Platform

Rejected because it creates a second account identity authority and violates the accepted Platform Identity ownership boundary.

## Required follow-up

A separately authorized implementation/migration task must later define and prove:

1. additive storage of canonical `AccountId` in Platform Identity;
2. UUIDv7 generation and uniqueness/collision behavior;
3. deterministic backfill for existing Platform Identities without changing `identities.id`;
4. versioned native ticket redeem/login-context schema using `AccountId`;
5. explicit Canary ACL mapping and legacy endpoint/version retention during rollout;
6. cross-language UUID representation/canonicalization fixtures;
7. rollback and mixed-version failure behavior;
8. coordinated Oteryn-v2 consumer adoption only after its owning foundation gates permit implementation.

No runtime, database migration, Oteryn-v2 repository write, protocol activation or production deployment is authorized by this ADR alone.
