# Platform-originated Canary Account Provisioning Contract

> **LEGACY / COMPATIBILITY** — Canary compatibility path; native target is defined by the Game contracts (LCFA, native login, character authority boundary).

## Status

`IMPLEMENTED — GREENFIELD ACCOUNT PROVISIONING AND IMMUTABLE BINDING`

This contract defines the only approved Phase 5 operation for creating a Canary account for a greenfield Oteryn Platform Identity and establishing the immutable `1 Platform Identity <-> 1 Canary accounts.id` ownership binding selected by ADR 0004.

The implementation is delivered through PR #33. Existing Canary accounts are not imported or claimed.

## Product ownership prerequisite

ADR 0004 is authoritative:

`1 Platform Identity <-> 1 Canary accounts.id`

Rules:

- supported accounts are greenfield and originate only from Oteryn Platform;
- self-service unlink, rebind and transfer are forbidden;
- normal recovery restores the same Platform Identity and therefore the same immutable Canary account binding;
- all later user-scoped Canary authorization is derived from the ready Platform-owned binding.

## Canary account-create surface

The operation writes only:

- `accounts.name`;
- `accounts.password`;
- `accounts.email`;
- `accounts.creation`.

All other account columns use current Canary defaults unless a later separately approved lifecycle/profile contract changes them.

Current Canary account insertion triggers creation of three default non-customizable VIP groups. Those trigger writes are Canary-owned side effects of the account transaction; Oteryn Platform does not receive direct `account_vipgroups` write privileges.

If the trigger fails, the Canary transaction fails and the Platform binding remains not-ready.

## Internal Canary account name

The Canary `accounts.name` for Platform-originated accounts is an internal immutable provisioning identifier, not a user login name or ownership signal.

Format:

`op` + 30 lowercase hexadecimal characters generated from 15 cryptographically secure random bytes.

Properties:

- exact length 32;
- generated server-side before any Canary write;
- persisted in Platform-owned provisioning state;
- globally unique by Platform constraint and Canary `accounts.name` uniqueness;
- immutable;
- never accepted from browser input;
- used as the deterministic forward-recovery key after partial failure.

## Canary password compatibility strategy

The current Canary schema requires `accounts.password`, but Oteryn Platform is the user credential authority and future game login must not depend on a reusable Canary password known by the user.

For each new Platform-originated Canary account the implementation:

1. generates a high-entropy random temporary sink secret;
2. derives the current legacy-compatible digest representation;
3. inserts only the derived value into `accounts.password`;
4. never returns, persists or logs the sink plaintext;
5. erases the temporary material from process memory after use;
6. never reads `accounts.password` back.

This compatibility value is not the Platform Identity password and is not a user authentication mechanism.

The existence of this sink credential does **not** mean the future authoritative game-login bridge is implemented.

## Platform-owned provisioning/binding state

`identity_canary_accounts` provides durable state containing the logical equivalents of:

- unique `identity_id`;
- nullable unique `canary_account_id` while provisioning is pending;
- unique immutable `provisioning_name`;
- immutable `canary_creation_epoch`;
- pending / ready / conflict state;
- readiness/completion timestamp;
- bounded failure metadata and normal timestamps.

The Platform record exists before the first Canary insert attempt.

Database constraints enforce:

- at most one provisioning/binding record per Platform Identity;
- at most one Platform binding for a Canary `accounts.id`;
- globally unique provisioning name.

A non-ready Identity fails closed for user-scoped Canary operations.

## Provisioning saga — IMPLEMENTED

Platform and Canary persistence are separate database boundaries. The implementation does not pretend one local transaction spans both databases.

### Step 1 — durable Platform intent

Inside Platform persistence:

1. create or lock the Identity provisioning/binding record;
2. generate/persist immutable provisioning name and creation marker when absent;
3. commit pending intent before any Canary write.

Registration may create the Identity and pending intent in the same Platform transaction.

### Step 2 — Canary transaction

Using only `canary_provisioning`:

1. begin Canary transaction;
2. attempt `accounts(name, password, email, creation)` insert with persisted provisioning identity and a newly generated sink credential digest;
3. allow Canary account-create trigger side effects;
4. select only `id`, `name`, `creation` for the exact provisioning name;
5. require exact name and creation-marker match;
6. commit only when account row and trigger side effects succeed.

The operation does not read `accounts.password`.

### Step 3 — durable Platform binding

Inside Platform persistence:

1. lock the provisioning/binding row;
2. null `canary_account_id` -> store exact recovered/created ID and mark ready;
3. same existing ID -> idempotent success;
4. different existing ID -> hard ownership conflict;
5. emit successful provisioning security/audit state according to Platform recorder conventions.

## Retry and partial-failure semantics

### Canary unavailable before insert

- no Canary account is assumed to exist;
- Platform record remains pending;
- retry reuses the same provisioning name and creation marker.

### Insert or trigger failure

- Canary transaction rolls back;
- Platform record remains pending;
- retry is allowed.

### Canary commit succeeds, Platform finalization fails

- no destructive automatic Canary deletion is attempted;
- retry selects by the persisted provisioning name;
- recovered `creation` must match the persisted creation marker;
- the exact same binding is finalized.

### Duplicate provisioning name

A duplicate is not automatically a new create. Existing row recovery requires exact provisioning-name and creation-marker match.

Mismatch enters fail-closed conflict state. The implementation must not silently generate a replacement name after an ambiguous committed external state.

### Concurrent workers

For one Identity:

- Platform uniqueness/locking serializes ownership finalization;
- workers reuse the same persisted provisioning identity;
- Canary uniqueness prevents duplicate account rows for one intent;
- finalization to the same ID is idempotent;
- a different ID is a hard conflict.

For different Identities:

- provisioning names are independently random;
- unique bound `canary_account_id` prevents one Canary account from authorizing two Platform Identities.

## Dedicated least-privilege connection — IMPLEMENTED

Connection:

`canary_provisioning`

The generic `canary` / `oteryn_readonly` connection remains unchanged.

Approved privileges are limited to:

- column-level `INSERT` on `accounts(name, password, email, creation)`;
- column-level `SELECT` on `accounts(id, name, creation)`.

Not approved:

- account UPDATE or DELETE;
- reading `accounts.password`;
- session tables;
- player writes;
- direct VIP-group writes;
- guild/ban/coin writes;
- DDL;
- `GRANT OPTION` or administrative privileges.

A reviewed SQL provisioning template defines the exact deployment grants. Production credentials are not stored in Git.

A fail-closed effective-grant verifier rejects missing required or excessive privileges.

Real MariaDB integration coverage proves that the restricted principal can perform the approved account insert/recovery and Canary trigger side effects without receiving direct VIP-group write privileges, while password reads and broader mutations remain denied.

## Authorization

Only an authenticated Platform Identity may initiate provisioning for its own Identity record.

The operation never accepts a target Canary `accounts.id` or provisioning name from the browser.

The created/recovered `accounts.id` is discovered only from the exact server-generated persisted provisioning identity and becomes authoritative only after durable Platform binding finalization.

## Validation evidence

PR #33 validation covers:

- client non-control of account ID and provisioning identity;
- successful provisioning and ready binding;
- dependency-unavailable pending state;
- retry and forward recovery;
- idempotent completed state;
- hard recovery conflict;
- binding uniqueness;
- effective-grant policy;
- real MariaDB restricted-principal execution;
- account-create trigger side effects;
- denial of `accounts.password` reads;
- duplicate-free recovery after committed Canary state.

The immutable binding contract records the final delivered evidence and ownership policy.

## Deployment gate

Before enabling account provisioning in an environment:

1. provision the dedicated `canary_provisioning` principal out-of-band from the reviewed SQL template;
2. supply credentials through approved secret management;
3. run the provisioning privilege verifier;
4. fail closed if grants are missing or broader than approved;
5. revalidate if deployed Canary schema/trigger behavior differs materially from the evidence baseline.

## Game-login follow-up

Provisioning a Canary account is not the same as enabling authoritative Platform-backed game login.

A separately authorized cross-repository task must provide short-lived exact-account Platform authorization with explicit expiry, audience, replay/session-consumption and revocation semantics and no user dependence on the sink credential.

Expected primary external scope is `opentibiabr/login-server`. `blakinio/canary` changes are required only if the final protocol needs direct assertion verification or stronger replay/revocation/fencing behavior.

No Canary/login-server repository was modified by Phase 5.

## Decision

`PLATFORM-ORIGINATED CANARY ACCOUNT PROVISIONING: IMPLEMENTED`

`IMMUTABLE GREENFIELD PLATFORM IDENTITY -> CANARY accounts.id BINDING: IMPLEMENTED`

The operation is production-enableable only after the environment-specific least-privilege principal is provisioned and verified.
