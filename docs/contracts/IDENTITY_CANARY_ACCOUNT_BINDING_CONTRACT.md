# Identity to legacy engine Account Binding Contract

> **LEGACY / COMPATIBILITY (RETIRING)** — legacy engine compatibility path; the native target is split by owner: Platform owns `AccountId`, Game Login Ticket issuance/redemption, routing policy and Gateway pre-admission (`docs/contracts/OTERYN_V2_ACCOUNT_IDENTITY_CONTRACT.md`, `docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md`); the Game contracts (LCFA, native login, character authority boundary) own final admission, session and character authority. Platform Identity authority for `AccountId`, credential security, login-ticket issuance and legacy engine mapping is defined by `docs/contracts/OTERYN_V2_ACCOUNT_IDENTITY_CONTRACT.md`.

## Status

`READ-ONLY BINDING STATE — IMMUTABLE 1:1 PLATFORM-OWNED BINDING; PROVISIONING AND CHARACTER CREATION RETIRED`

This contract defines the authorization boundary between an authenticated Oteryn Platform Identity and legacy engine `accounts.id`.

Oteryn Platform is the authoritative owner of user Identity, supported account lifecycle policy and user credential policy. Existing legacy engine accounts are outside the supported Phase 5 ownership model and are not imported or claimed.

Platform-originated account provisioning was removed; registration creates no binding and a binding can only arrive from a native source.

## Authoritative ownership model

Canonical cardinality:

`1 Platform Identity <-> 1 legacy engine accounts.id`

Rules:

- every supported product account originates as an Oteryn Platform Identity;
- each Platform Identity has at most one durable Platform-owned provisioning/binding record;
- each ready binding references exactly one legacy engine `accounts.id`;
- one legacy engine `accounts.id` cannot be actively bound to two Platform Identities;
- ownership is established only from the exact legacy engine account created/recovered by the server-generated provisioning intent;
- email equality, account name, client-supplied `account_id` or other browser-controlled attributes never establish ownership;
- supported existing-account claim/import is not part of Phase 5.

ADR 0004 remains the durable product decision.

## Persisted state

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

For every user-scoped legacy engine operation the Platform must answer server-side:

> Does the currently authenticated Platform Identity have the ready immutable binding to this exact legacy engine `accounts.id`?

The answer is resolved from Platform-owned trusted persistence.

A requested account ID may be used only as a target after comparison with the authenticated Identity's stored ready binding. It is never ownership evidence by itself.

## Binding establishment

Platform-originated legacy engine account provisioning, formerly the only Phase 5 establishment path, was removed. A binding can only arrive from a native source; the description below is historical.

The implementation:

Retired flow (removed from Platform): pending intent, dedicated-connection account create, durable finalization and fail-closed conflict handling. Existing binding rows are read only.

The implementation never accepts a client-selected legacy engine account ID for binding.

## Lifecycle policy

### Unlink

Self-service unlink is forbidden.

### Rebind / transfer

Self-service rebind and transfer are forbidden in normal operation.

Any future exceptional transfer requires a separately approved privileged recovery contract with Admin/RBAC, confirmed MFA, independent evidence and audit.

### Recovery

Normal account recovery restores access to the same Platform Identity and therefore retains the same immutable legacy engine binding.

Missing/corrupted binding or Canary-account state must fail closed. Automatic replacement-account creation or silent rebind is forbidden.

### Disablement / deletion

Disabling a Platform Identity must disable Platform authorization for user-scoped operations without transferring ownership.

Exact irreversible legacy engine account deletion semantics require a separate account-lifecycle contract.

## Security boundary

- Oteryn Platform remains authoritative for user credentials and ownership policy.
- Canary remains semantic owner of Canary-owned schema/game state.
- Existing `canary` / `oteryn_readonly` remains SELECT-only and unchanged.
- Platform does not read `accounts.password` and does not verify legacy engine passwords.
- Sink plaintext is never stored in Platform binding/audit state or exposed to users.

## Failure and concurrency invariants

The retired provisioning saga ensured:

- a Platform Identity is never treated as ready without a durable exact `canary_account_id` binding;
- legacy engine dependency failure leaves durable pending state for retry;
- a committed legacy engine account followed by Platform-finalization failure is recovered forward by immutable provisioning name + creation epoch;
- retries cannot authorize multiple accounts for one Identity;
- database uniqueness prevents one legacy engine account ID from being bound to two Identities;
- mismatched recovery evidence fails closed as conflict;
- no destructive automatic account deletion is used as partial-failure compensation.

The retired character-create operation additionally ensured:

- only the authenticated Identity's ready exact binding supplies the target `account_id`;
- same-account character creates serialize through the account-row lock before quota evaluation;
- the active-character limit cannot be exceeded by concurrent Platform create requests for one account;
- global character-name uniqueness remains protected by the legacy engine database unique constraint;
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

A side-effect-free authoritative existing-account control proof remains unimplemented, but it is not required for the greenfield product because existing legacy engine accounts are out of scope.

Adding import/claim later requires a new explicitly approved contract and likely separately authorized auth-side work.

## Game-login follow-up

Ownership binding and game-login availability are separate boundaries.

The immutable ownership binding is read-only state in Platform (character creation was removed), and Platform-originated users still require a separately authorized authoritative game-login integration before they can authenticate to the game under Platform credential authority.

Required future direction:

- `opentibiabr/login-server`: Platform-authorized short-lived cryptographic assertion/session exchange bound to exact `accounts.id`, with explicit audience, expiry, replay/session-consumption and revocation semantics and no sink-password dependency;
- `blakinio/canary`: only if the final login contract requires direct assertion verification or stronger replay/revocation/fencing semantics beyond the selected login-server integration.

No legacy engine/login-server repository was modified by Phase 5.

## Decision

`PLATFORM IDENTITY -> AUTHORIZED CANARY accounts.id OWNERSHIP BINDING: READ-ONLY STATE; ESTABLISHMENT PENDING A NATIVE SOURCE.`

`GREENFIELD CHARACTER CREATION THROUGH THE READY BINDING: RETIRED FROM PLATFORM.`

The original Phase 5 ownership-binding blocker is not resolved in Platform until a native source establishes bindings.

Character deletion, rename and account deletion/rebind/transfer are not authorized by this contract and require separate future operation contracts.
