# Identity to Canary Account Binding Contract

> **LEGACY / COMPATIBILITY** — Canary compatibility path; native target is defined by the Game contracts (LCFA, native login, character authority boundary).

## Status

`IMPLEMENTED FOR GREENFIELD ACCOUNTS — IMMUTABLE 1:1 PLATFORM-OWNED BINDING`

This contract defines the authorization boundary between an authenticated Oteryn Platform Identity and Canary `accounts.id`.

Oteryn Platform is the authoritative owner of user Identity, supported account lifecycle policy and user credential policy. Existing Canary accounts are outside the supported Phase 5 ownership model and are not imported or claimed.

The greenfield ownership-binding implementation is delivered by the Platform-originated account provisioning flow governed by `docs/contracts/PLATFORM_CANARY_ACCOUNT_PROVISIONING_CONTRACT.md`.

## Authoritative ownership model

Canonical cardinality:

`1 Platform Identity <-> 1 Canary accounts.id`

Rules:

- every supported product account originates as an Oteryn Platform Identity;
- each Platform Identity has at most one durable Platform-owned provisioning/binding record;
- each ready binding references exactly one Canary `accounts.id`;
- one Canary `accounts.id` cannot be actively bound to two Platform Identities;
- ownership is established only from the exact Canary account created/recovered by the server-generated provisioning intent;
- email equality, account name, client-supplied `account_id` or other browser-controlled attributes never establish ownership;
- supported existing-account claim/import is not part of Phase 5.

ADR 0004 remains the durable product decision.

## Implemented trusted state

Platform-owned `identity_canary_accounts` persistence provides:

- `identity_id` as the one-row-per-Identity key;
- nullable unique `canary_account_id` while provisioning is pending;
- unique immutable server-generated `provisioning_name`;
- immutable `canary_creation_epoch` recovery marker;
- pending / ready / conflict lifecycle state;
- bounded failure metadata and timestamps.

A binding authorizes user-scoped Canary operations only when it is ready and contains a non-null exact `canary_account_id`.

Pending or conflict state fails closed.

## Required authorization answer

For every user-scoped Canary operation the Platform must answer server-side:

> Does the currently authenticated Platform Identity have the ready immutable binding to this exact Canary `accounts.id`?

The answer is resolved from Platform-owned trusted persistence.

A requested account ID may be used only as a target after comparison with the authenticated Identity's stored ready binding. It is never ownership evidence by itself.

## Binding establishment

The only supported Phase 5 establishment path is Platform-originated Canary account provisioning.

The implementation:

1. creates the Platform Identity and durable pending provisioning intent before any Canary write;
2. uses a server-generated immutable provisioning name and creation marker;
3. creates or deterministically recovers the exact Canary account through the dedicated `canary_provisioning` connection;
4. stores the exact created/recovered `accounts.id` in Platform-owned binding state;
5. marks the binding ready only after durable Platform finalization;
6. treats retry of the same completed binding as idempotent;
7. enters fail-closed conflict state if recovered ownership evidence conflicts.

The implementation never accepts a client-selected Canary account ID for binding.

## Lifecycle policy

### Unlink

Self-service unlink is forbidden.

### Rebind / transfer

Self-service rebind and transfer are forbidden in normal operation.

Any future exceptional transfer requires a separately approved privileged recovery contract with Admin/RBAC, confirmed MFA, independent evidence and audit.

### Recovery

Normal account recovery restores access to the same Platform Identity and therefore retains the same immutable Canary binding.

Missing/corrupted binding or Canary-account state must fail closed. Automatic replacement-account creation or silent rebind is forbidden.

### Disablement / deletion

Disabling a Platform Identity must disable Platform authorization for user-scoped operations without transferring ownership.

Exact irreversible Canary account deletion semantics require a separate account-lifecycle contract.

## Security boundary

- Oteryn Platform remains authoritative for user credentials and ownership policy.
- Canary remains semantic owner of Canary-owned schema/game state.
- Existing `canary` / `oteryn_readonly` remains SELECT-only and unchanged.
- Account provisioning uses separate `canary_provisioning` credentials restricted to the approved column-level account-create/recovery surface.
- Character creation uses separate `canary_character_create` credentials restricted to the approved column-level character-create surface.
- Platform does not read `accounts.password` and does not verify Canary passwords.
- Canary `accounts.password` receives only the non-user random sink-credential compatibility digest defined by the provisioning contract.
- Sink plaintext is never stored in Platform binding/audit state or exposed to users.

## Failure and concurrency invariants

The implemented provisioning saga ensures:

- a Platform Identity is never treated as ready without a durable exact `canary_account_id` binding;
- Canary dependency failure leaves durable pending state for retry;
- a committed Canary account followed by Platform-finalization failure is recovered forward by immutable provisioning name + creation epoch;
- retries cannot authorize multiple accounts for one Identity;
- database uniqueness prevents one Canary account ID from being bound to two Identities;
- mismatched recovery evidence fails closed as conflict;
- no destructive automatic account deletion is used as partial-failure compensation.

The implemented character-create operation additionally ensures:

- only the authenticated Identity's ready exact binding supplies the target `account_id`;
- same-account character creates serialize through the account-row lock before quota evaluation;
- the active-character limit cannot be exceeded by concurrent Platform create requests for one account;
- global character-name uniqueness remains protected by the Canary database unique constraint;
- same-account active same-name retries recover idempotently without ownership reassignment or generic player UPDATE.

## Validation evidence

PR #33 account-provisioning/binding validation includes:

- registration tests proving client-supplied `account_id` and `provisioning_name` cannot control ownership;
- saga tests for success, pending dependency failure, retry, idempotent completed state, hard conflict and binding uniqueness;
- privilege-policy tests rejecting table-level/excessive grants and password-read capability;
- real MariaDB 11.8 integration coverage proving effective column-level provisioning grants, Canary-compatible account-trigger side effects, denial of `accounts.password` reads, duplicate-free retry and forward recovery.

PR #41 character-create validation includes:

- authenticated feature tests using the real Platform login/session establishment path;
- ready-binding authorization and request non-control tests;
- exact name/vocation/sex policy tests;
- real MariaDB exact-grant and forbidden-privilege tests;
- real MariaDB account-lock/quota and global same-name race tests;
- starter/default persisted-row and committed-row recovery tests.

PR #41 merged to `main` as `9839822b8e445c0e9828e73d2d7767bb237e587f` after final CI #568 and Agent Governance #489 passed.

## Existing-account proof

A side-effect-free authoritative existing-account control proof remains unimplemented, but it is not required for the greenfield product because existing Canary accounts are out of scope.

Adding import/claim later requires a new explicitly approved contract and likely separately authorized auth-side work.

## Game-login follow-up

Ownership binding and game-login availability are separate boundaries.

The immutable ownership binding and character creation are implemented, but Platform-originated users still require a separately authorized authoritative game-login integration before they can authenticate to the game under Platform credential authority.

Required future direction:

- `opentibiabr/login-server`: Platform-authorized short-lived cryptographic assertion/session exchange bound to exact `accounts.id`, with explicit audience, expiry, replay/session-consumption and revocation semantics and no sink-password dependency;
- `blakinio/canary`: only if the final login contract requires direct assertion verification or stronger replay/revocation/fencing semantics beyond the selected login-server integration.

No Canary/login-server repository was modified by Phase 5.

## Decision

`PLATFORM IDENTITY -> AUTHORIZED CANARY accounts.id OWNERSHIP BINDING: IMPLEMENTED FOR GREENFIELD ACCOUNTS.`

`GREENFIELD CHARACTER CREATION THROUGH THE READY BINDING: IMPLEMENTED.`

The original Phase 5 ownership-binding blocker is resolved for supported greenfield accounts.

Character deletion, rename and account deletion/rebind/transfer are not authorized by this contract and require separate future operation contracts.
