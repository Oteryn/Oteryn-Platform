---
task_id: OTERYN-20261117-review5-fixes
governing_issue: 1922
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-review5-fixes

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1922
(round 4 follow-ups). Fix the confirmed findings of the fifth wide review
(subagent slices on main afb4a8fd):

1. Identity, email change: while a primary email change can still be
   reverted, its old address stays reserved, so a stranger cannot register it
   and block the owner's 30-day revert (`accounts::address_available`).
2. Characters, V1 receipt path: `publish` refuses V2 service commands
   (rename, deletion scheduling) so they settle only through their own
   receipt path, which also settles or releases the wallet reservation.
3. Identity, recovery key: `generate` and `confirm` require an active,
   verified owner; an account still pending verification may belong to
   whoever registered the address. Recovery of existing keys is unchanged, so
   a later email change (which bumps `email_verified_at`) does not invalidate
   keys made since.

Out of scope, reported to the coordinator: bazaar runtime compensation of a
refused character transfer bind (bazaar contract is a proposal), the players
router permission for moderator-only kick (role-catalog question), and the
chained email change after the 72 h block (partially covered by item 1).

## Acceptance criteria

- [x] Registering the old address during the revert window is refused and the revert succeeds; after the window closes the address is free (`r5_old_address_stays_reserved_through_the_revert_window`)
- [x] A V1 receipt for a V2 service command is refused and the command stays pending (`db_v1_receipt_refuses_v2_service_commands`)
- [x] A pending account cannot generate a recovery key; a key made after verification survives a later verification bump (`r5_recovery_key_requires_a_verified_owner`)
- [ ] Portal RS CI green on the PR head (no PR yet)

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-review5-fixes.md
  - portal-rs/crates/identity/src/accounts.rs
  - portal-rs/crates/identity/src/email_change_tests.rs
  - portal-rs/crates/identity/src/recovery_key.rs
  - portal-rs/crates/identity/src/recovery_key_tests.rs
  - portal-rs/crates/characters/src/commands/storage.rs
  - portal-rs/crates/characters/src/commands/tests.rs
modules:
  - identity
  - characters
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-17T12:00:00Z
head: pending
branch: claude/happy-davinci-ijz3wy-review5-fixes
pr: none
status: ready
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-review5-fixes.md
  - portal-rs/crates/identity/src/accounts.rs
  - portal-rs/crates/identity/src/email_change_tests.rs
  - portal-rs/crates/identity/src/recovery_key.rs
  - portal-rs/crates/identity/src/recovery_key_tests.rs
  - portal-rs/crates/characters/src/commands/storage.rs
  - portal-rs/crates/characters/src/commands/tests.rs
proven:
  - Each new test is red on main afb4a8fd and green with the fix (PostgreSQL 18)
  - The live character_commands CHECK (t46) admits RenameCharacter and the other V2 commands, so the V1 path could receive them
  - No migration, contract, fixture, flag or workflow change
derived:
  - E2E registration helpers verify the address before using recovery keys, so the generate gate does not change those flows
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Requiring key created_at >= email_verified_at at recovery would revoke legitimate keys after every confirmed email change
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-review5-fixes.md
  - portal-rs/crates/identity/src/accounts.rs
  - portal-rs/crates/identity/src/email_change_tests.rs
  - portal-rs/crates/identity/src/recovery_key.rs
  - portal-rs/crates/identity/src/recovery_key_tests.rs
  - portal-rs/crates/characters/src/commands/storage.rs
  - portal-rs/crates/characters/src/commands/tests.rs
validation:
  - command: cargo test -p oteryn-identity --lib; cargo test -p oteryn-characters --lib; cargo test -p oteryn-admin --lib (PostgreSQL 18)
    result: NOT_RUN
    evidence: pending
blockers:
  - none
next_action: Portal RS CI on the PR
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Task branch for the review 5 fixes; retained until merge
source_branch_evidence: The branch records the published head
```
