# ADR 0004 — Oteryn Platform is the authoritative account and Identity owner

## Status

Accepted — 2026-07-20

## Context

Phase 5 requires an unambiguous and testable authorization path from an authenticated Oteryn Platform Identity to one exact legacy engine `accounts.id` before any user-scoped legacy engine account or character mutation can be enabled.

The current system has separate Platform Identity credentials and legacy engine/login-server game credentials. Existing-account claim was investigated, but current legacy engine/login-server source does not expose a purpose-built, side-effect-free account-control proof API suitable for binding an existing account without either duplicating credential verification or creating a game session.

This project is greenfield for account ownership. Existing legacy engine accounts are not migration inputs and do not need a self-service claim path.

## Decision

Oteryn Platform is the authoritative owner of user account identity, account lifecycle and user credential policy.

For the product model established by this ADR:

- every user account originates as an Oteryn Platform Identity;
- each Platform Identity owns exactly one legacy engine account;
- each legacy engine account created for the product is owned by exactly one Platform Identity;
- the binding is created only as part of the approved Platform-originated legacy engine account creation operation;
- existing pre-product legacy engine accounts are outside the supported ownership model and are not claimable or imported by Phase 5;
- browser/client supplied `accounts.id`, email equality or other attribute matching never establishes ownership;
- the durable binding is Platform-owned state keyed by immutable legacy engine `accounts.id`;
- binding ownership is immutable in normal product operation.

Canonical cardinality:

`1 Platform Identity <-> 1 legacy engine accounts.id`

## Lifecycle policy

### Unlink

Self-service unlink is not supported. Removing the binding would leave the Identity in an invalid state for game-account operations and is therefore forbidden as a normal operation.

### Rebind / transfer

Self-service rebind or transfer to another legacy engine account or another Platform Identity is not supported.

Any future exceptional ownership transfer requires a new explicitly approved privileged recovery contract, Admin/RBAC authorization, confirmed MFA, independent account-control evidence and audit. It is not part of Phase 5.

### Recovery

Normal account recovery is performed through the authoritative Platform Identity recovery lifecycle. Recovery restores access to the same Platform Identity and therefore to the same immutable legacy engine account binding; it does not create or select a new legacy engine account.

If the legacy engine account or binding is corrupted or lost, automatic recreation/rebinding is forbidden. That condition is a fail-closed operational incident requiring a separately approved privileged recovery procedure.

### Deletion / disablement

Disabling or deleting a Platform Identity must not silently transfer its legacy engine account to another Identity. Exact deletion semantics for legacy engine account data require a separate account-lifecycle operation contract before implementation.

## Security consequences

- Oteryn Platform becomes the authoritative source for user credential policy and account ownership.
- Legacy engine remains the semantic owner of game state and game-server-owned schema.
- The existing `legacy` SQL connection remains read-only and must never be expanded for account creation.
- Platform-originated legacy engine account creation requires a separate least-privilege write credential/connection scoped only to the approved operation.
- Platform must not implement legacy engine password verification merely to establish ownership.
- Current native legacy engine/external login-server password paths are legacy/current implementation dependencies, not the target authority model.
- A future game-login integration must consume Platform-authorized authentication/session material or otherwise delegate to the Platform authority; that cross-repository change requires separate authorization and rollout planning.

## Transactional consequence for account creation

The authoritative product operation is conceptually:

`create Platform Identity -> create legacy engine account -> persist immutable binding`

The implementation must define failure semantics that prevent an active product Identity from being considered game-account-ready unless its exact legacy engine account binding is durably established. The exact cross-database transaction/saga/compensation design and legacy engine account credential representation must be approved in an operation-level account-creation contract before any shared write is implemented.

## Rejected alternatives

### Claim existing legacy engine accounts as the primary model

Rejected for the greenfield product. Existing legacy engine accounts are not part of the supported migration scope, and current auth-side source lacks the required side-effect-free universal account-control proof capability.

### Infer ownership from email

Rejected because legacy engine account email is not unique and equality does not prove control.

### Trust client-supplied account IDs

Rejected because an identifier is not authorization evidence.

### Verify legacy engine credentials inside Oteryn Platform

Rejected because it duplicates credential authority, broadens access to credential-sensitive state and conflicts with the selected authoritative Platform model.

## Follow-up

The next dependency is a bounded Platform-originated legacy engine account creation operation contract. It must define:

- exact legacy engine account fields and defaults;
- credential/game-login representation compatible with the selected authoritative Platform model;
- separate least-privilege write connection and grants;
- cross-database transaction or compensating failure semantics;
- duplicate/concurrency/idempotency behavior;
- audit events and readiness state;
- rollout dependency on future legacy engine/login-server authentication integration.

Character creation remains blocked until account creation plus immutable ownership binding are actually implemented and tested.