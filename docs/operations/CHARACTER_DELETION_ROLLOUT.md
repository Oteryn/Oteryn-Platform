# Character Deletion Lifecycle Rollout Gate

## Status

`BLOCKED — CANARY PREREQUISITE REQUIRED`

This runbook does not authorize deployment. It records the only safe producer/consumer order after the Canary-owned prerequisite in Issue #344 is explicitly authorized and implemented.

## Current hold

Do not:

- add an Oteryn character-deletion route, controller, job or command;
- provision a deletion database principal;
- grant `UPDATE(players.deletion)` or `DELETE(players)`;
- change the Canary startup cleanup from this repository;
- expose pending-deletion UI as functional;
- claim staging or production readiness.

## Required rollout order

1. **Canary contract and implementation**
   - merge the operation-safe schedule/cancel/finalize lifecycle in `blakinio/canary`;
   - fence or replace the legacy startup raw-delete path;
   - publish exact schema, interface, bounded results, side effects and rollback behavior;
   - pass Canary unit, real-MariaDB concurrency, destructive-side-effect and recovery tests.

2. **Canary deployment compatibility window**
   - deploy the producer while no Platform caller exists;
   - keep the legacy caller surface disabled or read-only;
   - verify the exact deployed revision and effective privileges;
   - prove old server nodes cannot bypass the new finalizer contract.

3. **Oteryn contract revalidation**
   - pin the deployed-compatible Canary revision;
   - update `docs/contracts/CHARACTER_DELETION_CONTRACT.md` from NO-GO to an approved bounded operation only if every invariant is proven;
   - approve the exact Platform schema, credential/interface, saga, reconciliation and audit design in a new task.

4. **Oteryn implementation while hidden**
   - deploy Platform-owned operation state, reconciliation and notifications first;
   - keep routes and navigation disabled;
   - run effective-grant checks and environment-gated integration tests;
   - verify mutual exclusion with Character Bazaar, rename and transfer.

5. **Staging acceptance**
   - exercise schedule, immediate login exclusion, cancel/restore, deadline passage, finalization and every recovery state;
   - verify guild, house, market, mail, session and public-profile policies;
   - pass English/Polish desktop, tablet and mobile zero-retry acceptance.

6. **Production activation**
   - run preflight against the exact Canary and Platform revisions;
   - verify no in-flight incompatible operations;
   - enable the feature flag for a bounded cohort;
   - observe reconciliation and failure classifications before wider activation.

## Rollback order

1. disable new Platform schedule requests;
2. keep reconciliation and cancel/recovery paths available;
3. do not revoke the operation interface while pending operations exist;
4. drain or deterministically classify every in-flight operation;
5. roll back the Platform consumer;
6. roll back the Canary producer only after proving no pending operation depends on it;
7. never restore the legacy raw startup delete path while contracted operation rows remain active.

Rollback must not convert an ambiguous state into success or directly modify player rows outside the approved lifecycle.

## Activation evidence

Production activation requires recorded evidence for:

- exact Canary and Oteryn revisions;
- schema compatibility;
- effective required and forbidden privileges;
- finalizer exclusivity;
- queue/scheduler/reconciliation health;
- backup and rollback readiness;
- test identities and test characters isolated from real users;
- no unresolved recovery operations;
- audit and notification delivery health.

Until those conditions are met, the public capability remains unavailable and Issue #317 remains blocked by Issue #344.
