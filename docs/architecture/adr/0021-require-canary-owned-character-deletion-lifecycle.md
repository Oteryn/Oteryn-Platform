# ADR 0021: Require a Canary-owned character deletion lifecycle

- Status: Accepted
- Date: 2026-07-30
- Decision owners: Oteryn Platform architecture and security boundary
- Related: #342, #317, #277, #268

## Context

Oteryn Platform requires an owner-facing character lifecycle with scheduling, a grace period, cancellation/restoration, deterministic finalization, recovery and audit.

Current Canary evidence at `blakinio/canary@3c90d3ada717cd2ed0c5344f1dac210a205355f6` provides a legacy mechanism:

- `players.deletion = 0` is active;
- any non-zero value immediately removes the character from account lists and blocks game preload;
- the value is interpreted as a timestamp by a server-startup Lua cleanup;
- startup submits an asynchronous physical `DELETE FROM players` for expired rows;
- physical deletion cascades through broad player data;
- deleting a guild owner cascades deletion of the guild and its dependent rows;
- a database trigger clears house ownership.

This mechanism has no operation identifier, deterministic request result, continuously scheduled finalizer, recovery receipt or Platform-correlatable audit boundary. Directly granting the Platform `UPDATE(deletion)` or `DELETE(players)` would expose Canary-owned destructive semantics without proving mutual exclusion, rollback or reconciliation.

The existing character-creation and Character Bazaar credentials do not authorize this mutation.

## Decision

Oteryn Platform will **not** implement character deletion, restoration or finalization through direct use of the current Canary timestamp/startup-cleanup mechanism.

Canary remains the semantic owner of character lifecycle and destructive side effects. Before Oteryn Platform implementation, a separately authorized Canary change must expose one explicit, idempotent and testable schedule/cancel/finalize lifecycle or equivalent operation boundary.

The Canary prerequisite must:

1. fence or replace the uncorrelated raw startup delete path;
2. identify every request with a durable operation ID;
3. enforce deterministic ownership, online/session and concurrent-operation checks;
4. define guild, house, market and historical-retention effects;
5. make finalization independently invokable and observable rather than dependent on restart timing;
6. support bounded results and recovery after ambiguous outcomes;
7. provide a least-privilege interface and effective-grant verification;
8. document compatible rollout and rollback ordering.

Oteryn Platform may later own the web workflow, Platform operation state, notifications and privacy-safe audit trail. It may not become the semantic owner of Canary player deletion.

The detailed boundary is `docs/contracts/CHARACTER_DELETION_CONTRACT.md`.

## Consequences

### Positive

- prevents a web endpoint from unintentionally disbanding a guild, releasing a house or erasing gameplay/history rows;
- prevents infrastructure restart timing from masquerading as deterministic product finalization;
- preserves deny-by-default database privileges;
- creates an explicit point for idempotency, locking, recovery and cross-operation exclusion;
- keeps Canary gameplay semantics in the owning repository.

### Negative

- Issue #317 cannot proceed directly to Platform implementation;
- a separately authorized Canary task and coordinated rollout are required;
- the product gap remains open until both repositories have compatible evidence;
- the eventual operation may require a Canary schema/runtime change rather than a Platform-only migration.

## Rejected alternatives

### Grant `UPDATE(players.deletion)` to Platform

Rejected. It can make a character unavailable, but it does not provide deterministic finalization, operation receipts, side-effect policy or recovery, and it remains exposed to the startup cleanup lifecycle.

### Grant `DELETE(players)` to Platform

Rejected. It would expose broad foreign-key cascades and triggers, including guild and house effects, with an unnecessarily powerful destructive privilege.

### Reuse Character Bazaar transfer authority

Rejected. The transfer principal is limited to `UPDATE(players.account_id)` and requires `deletion = 0`. Ownership transfer does not authorize deletion or restoration.

### Treat the Platform operation row as authoritative

Rejected. Platform state cannot prove current Canary ownership, session state, existence or finalization after an ambiguous cross-database outcome.

### Preserve the startup cleanup and infer success later

Rejected. Absence of a player row cannot distinguish expected finalization from external deletion or an uncorrelated failure, and the cleanup provides no request identity or bounded result.

## Follow-up

1. Create a separately authorized Canary lifecycle prerequisite linked to #342/#317.
2. Keep Issue #317 blocked from implementation while that prerequisite is absent.
3. Reinspect exact Canary revision/schema and update the operation contract after the prerequisite merges.
4. Only then create the Platform implementation branch, credential template, migrations, tests and browser acceptance package.

This ADR authorizes no Canary write, runtime endpoint, database credential, production action or `PRODUCTION_PROVEN` claim.
