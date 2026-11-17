---
task_id: OTERYN-20261117-bazaar-bindcomp
governing_issue: 1928
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-bazaar-bindcomp

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1928.

Bazaar runtime, character sale, money path. When Game terminally refuses
the settlement `BindBazaarCharacterTransfer` (any refusal code except
`CHAR_CMD_INTENT_EXPIRED`, which stays a retry), the listing no longer
strands in `awaiting_delivery`. The worker retires the unpublished
settlement transfer (append-only `bazaar_authority_transfer_retirements`,
evidence = the refused Bind), stages the `BAZAAR_ESCROW_RETURN` transfer
and its Bind, and after Game reports `RETURNED` releases the buyer's funds
once (COINS reservation release, or GOLD `TERMINAL_REFUSAL` release),
records the compensation (return Bind op) and issues one Unlock, following
the contract's refused-flow order. Refusal is read only from a Game
receipt, never inferred from a timeout. No migration.

## Acceptance criteria

- [x] A refused settlement Bind retires the settlement transfer, issues the escrow-return Bind and does not release funds before `RETURNED`
- [x] COINS: after `RETURNED` the listing is cancelled, the buyer is refunded once and one Unlock is issued; extra ticks change nothing
- [x] GOLD: one `TERMINAL_REFUSAL` Release, then cancelled with one Unlock; extra ticks change nothing
- [x] An expired settlement Bind still retries for both currencies
- [ ] Portal RS CI green on the PR head (no PR yet)

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-bazaar-bindcomp.md
  - portal-rs/crates/bazaar/src/game/**
modules:
  - bazaar
dependencies:
  - docs/contracts/OTERYN_V2_BAZAAR_CONTRACT_PROPOSAL.md
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-17T20:00:00Z
head: 149b24b6
branch: claude/happy-davinci-ijz3wy-bazaar-bindcomp
pr: 1931
status: ready
terminal_pr_policy: archive_pending
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-bazaar-bindcomp.md
  - portal-rs/crates/bazaar/src/game/**
proven:
  - Without the production change the COINS and GOLD tests fail (escrow-return Bind never staged); without the listing_batch change they fail (return transfer never published)
  - The expired-Bind test is red when the worker stops excluding CHAR_CMD_INTENT_EXPIRED from refusal
  - A refused Bind that is not a settlement refusal keeps its recorded code without a worker error
  - No migration, contract, fixture or flag change
derived:
  - Retirement plus return Bind matches the contract rule that one competing binding must be terminally refused before the other becomes executable
unknown:
  - Game acceptance of an escrow-return Bind after a refused settlement Bind is unverified against the Game implementation
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Treating a Bind timeout as refusal; refusal must come from a Game receipt
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-bazaar-bindcomp.md
  - portal-rs/crates/bazaar/src/game/authority_exchange.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/runtime/worker.rs
validation:
  - command: cargo test -p oteryn-bazaar --lib character_settlement_bind (PostgreSQL)
    result: PASS
    evidence: 3 passed; COINS and GOLD tests red without the fix
  - command: cargo test -p oteryn-bazaar --lib (PostgreSQL)
    result: PASS
    evidence: 120 passed, 0 failed, no skip line
  - command: cargo fmt --all --check; cargo clippy -p oteryn-bazaar --all-targets -- -D warnings
    result: PASS
    evidence: both exit 0
blockers:
  - none
next_action: Portal RS CI on the PR head; archive this packet once the PR lands.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```

## Notes

`listing_batch` now polls every listing in `awaiting_delivery`, including
characters with a recorded refusal. The binding-digest denial has no
dedicated test. A non-expired refusal of the escrow-return Bind still strands the listing
and needs an operator.
