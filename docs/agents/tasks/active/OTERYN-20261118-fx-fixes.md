---
task_id: OTERYN-20261118-fx-fixes
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261118-fx-fixes

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Flow and class review of portal-rs on the round 15 head (21 flow/class
reviews and 11 slice reviews, read-only). Every finding confirmed P1/P2 by a
second read-only check is fixed with a red-then-green regression test through
the real caller: Bazaar recovery and projection-epoch handling (second epoch
site, tombstone regression, page errors), player-data and character custody,
store/wallet payer holds and closure obligations, referral and league workers,
security revocations for future suspensions, login-history and closure audit
retention, Game account-closure wire aligned byte for byte with the Game
producer, and copied death details purged on schedule even while statistics
is disabled. Also archives the round 15 packet after PR #1948 merged.

## Acceptance criteria

- [x] Each confirmed finding has a fix with a regression test through the real caller
- [x] Each pushed fix was reviewed read-only before integration
- [ ] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261118-fx-fixes.md
  - docs/agents/tasks/active/OTERYN-20261118-r15-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261118-r15-fixes.md
  - portal-rs/bin/oteryn-portal/src/worker.rs
  - portal-rs/bin/oteryn-portal/src/worker_tests.rs
  - portal-rs/crates/bazaar/src/game/authority.rs
  - portal-rs/crates/bazaar/src/game/authority_evidence.rs
  - portal-rs/crates/bazaar/src/game/authority_exchange.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/authority_wire.rs
  - portal-rs/crates/bazaar/src/game/invalidation_tests.rs
  - portal-rs/crates/bazaar/src/game/recovery_tests.rs
  - portal-rs/crates/bazaar/src/game/runtime/characters.rs
  - portal-rs/crates/bazaar/src/game/runtime/gold.rs
  - portal-rs/crates/bazaar/src/game/runtime/invalidation.rs
  - portal-rs/crates/bazaar/src/game/runtime/mod.rs
  - portal-rs/crates/bazaar/src/game/runtime/worker.rs
  - portal-rs/crates/characters/src/restore/storage.rs
  - portal-rs/crates/characters/src/restore/tests.rs
  - portal-rs/crates/characters/src/services/mod.rs
  - portal-rs/crates/characters/src/services/tests.rs
  - portal-rs/crates/community/src/referrals/campaign.rs
  - portal-rs/crates/community/src/referrals/tests.rs
  - portal-rs/crates/identity/src/account_lifecycle_http.rs
  - portal-rs/crates/identity/src/security_revocations_tests.rs
  - portal-rs/crates/identity/src/termination.rs
  - portal-rs/crates/identity/src/termination/README.md
  - portal-rs/crates/identity/src/termination/fixtures/SOURCE.md
  - portal-rs/crates/identity/src/termination/fixtures/pending.json
  - portal-rs/crates/identity/src/termination/fixtures/read.json
  - portal-rs/crates/identity/src/termination/fixtures/watermark.json
  - portal-rs/crates/identity/src/termination/game_closure.rs
  - portal-rs/crates/identity/src/termination/game_closure/http.rs
  - portal-rs/crates/identity/src/termination/game_closure/runtime.rs
  - portal-rs/crates/identity/src/termination/game_closure/tests.rs
  - portal-rs/crates/identity/src/termination/game_closure/wire.rs
  - portal-rs/crates/identity/src/termination_tests.rs
  - portal-rs/crates/minigames/src/league/mod.rs
  - portal-rs/crates/minigames/src/league/runtime.rs
  - portal-rs/crates/minigames/src/league/store.rs
  - portal-rs/crates/minigames/src/league/tests.rs
  - portal-rs/crates/playerdata/src/authority.rs
  - portal-rs/crates/playerdata/src/authority/reads.rs
  - portal-rs/crates/playerdata/src/authority/tests.rs
  - portal-rs/crates/playerdata/src/deaths/storage.rs
  - portal-rs/crates/playerdata/src/logins.rs
  - portal-rs/crates/playerdata/src/logins/tests.rs
  - portal-rs/crates/playerdata/src/retention.rs
  - portal-rs/crates/playerdata/src/store.rs
  - portal-rs/crates/stats/src/tests.rs
  - portal-rs/crates/stats/src/worker.rs
  - portal-rs/crates/store/src/delivery.rs
  - portal-rs/crates/store/src/delivery_tests.rs
  - portal-rs/crates/store/src/purchase_repository.rs
  - portal-rs/crates/store/src/purchase_tests.rs
  - portal-rs/crates/wallet/src/ledger.rs
  - portal-rs/migrations/20261118120000_security_revocations_future_suspension.sql
modules:
  - bazaar
  - characters
  - community
  - identity
  - minigames
  - oteryn-portal
  - playerdata
  - stats
  - store
  - wallet
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
updated_at: 2026-11-18T11:00:00Z
head: pending
branch: claude/happy-davinci-ijz3wy-fx
status: implementing
terminal_pr_policy: archive_pending
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261118-fx-fixes.md
  - docs/agents/tasks/active/OTERYN-20261118-r15-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261118-r15-fixes.md
  - portal-rs/bin/oteryn-portal/src/worker.rs
  - portal-rs/bin/oteryn-portal/src/worker_tests.rs
  - portal-rs/crates/bazaar/src/game/authority.rs
  - portal-rs/crates/bazaar/src/game/authority_evidence.rs
  - portal-rs/crates/bazaar/src/game/authority_exchange.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/authority_wire.rs
  - portal-rs/crates/bazaar/src/game/invalidation_tests.rs
  - portal-rs/crates/bazaar/src/game/recovery_tests.rs
  - portal-rs/crates/bazaar/src/game/runtime/characters.rs
  - portal-rs/crates/bazaar/src/game/runtime/gold.rs
  - portal-rs/crates/bazaar/src/game/runtime/invalidation.rs
  - portal-rs/crates/bazaar/src/game/runtime/mod.rs
  - portal-rs/crates/bazaar/src/game/runtime/worker.rs
  - portal-rs/crates/characters/src/restore/storage.rs
  - portal-rs/crates/characters/src/restore/tests.rs
  - portal-rs/crates/characters/src/services/mod.rs
  - portal-rs/crates/characters/src/services/tests.rs
  - portal-rs/crates/community/src/referrals/campaign.rs
  - portal-rs/crates/community/src/referrals/tests.rs
  - portal-rs/crates/identity/src/account_lifecycle_http.rs
  - portal-rs/crates/identity/src/security_revocations_tests.rs
  - portal-rs/crates/identity/src/termination.rs
  - portal-rs/crates/identity/src/termination/README.md
  - portal-rs/crates/identity/src/termination/fixtures/SOURCE.md
  - portal-rs/crates/identity/src/termination/fixtures/pending.json
  - portal-rs/crates/identity/src/termination/fixtures/read.json
  - portal-rs/crates/identity/src/termination/fixtures/watermark.json
  - portal-rs/crates/identity/src/termination/game_closure.rs
  - portal-rs/crates/identity/src/termination/game_closure/http.rs
  - portal-rs/crates/identity/src/termination/game_closure/runtime.rs
  - portal-rs/crates/identity/src/termination/game_closure/tests.rs
  - portal-rs/crates/identity/src/termination/game_closure/wire.rs
  - portal-rs/crates/identity/src/termination_tests.rs
  - portal-rs/crates/minigames/src/league/mod.rs
  - portal-rs/crates/minigames/src/league/runtime.rs
  - portal-rs/crates/minigames/src/league/store.rs
  - portal-rs/crates/minigames/src/league/tests.rs
  - portal-rs/crates/playerdata/src/authority.rs
  - portal-rs/crates/playerdata/src/authority/reads.rs
  - portal-rs/crates/playerdata/src/authority/tests.rs
  - portal-rs/crates/playerdata/src/deaths/storage.rs
  - portal-rs/crates/playerdata/src/logins.rs
  - portal-rs/crates/playerdata/src/logins/tests.rs
  - portal-rs/crates/playerdata/src/retention.rs
  - portal-rs/crates/playerdata/src/store.rs
  - portal-rs/crates/stats/src/tests.rs
  - portal-rs/crates/stats/src/worker.rs
  - portal-rs/crates/store/src/delivery.rs
  - portal-rs/crates/store/src/delivery_tests.rs
  - portal-rs/crates/store/src/purchase_repository.rs
  - portal-rs/crates/store/src/purchase_tests.rs
  - portal-rs/crates/wallet/src/ledger.rs
  - portal-rs/migrations/20261118120000_security_revocations_future_suspension.sql
proven:
  - Each finding was confirmed by a read-only check with file:line evidence
  - Each pushed fix was reviewed read-only against its packet before integration
  - The closure wire matches the Game producer fixtures pinned at Game 553802fa
derived:
  - none
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - none
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261118-fx-fixes.md
  - docs/agents/tasks/active/OTERYN-20261118-r15-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261118-r15-fixes.md
  - portal-rs/bin/oteryn-portal/src/worker.rs
  - portal-rs/bin/oteryn-portal/src/worker_tests.rs
  - portal-rs/crates/bazaar/src/game/authority.rs
  - portal-rs/crates/bazaar/src/game/authority_evidence.rs
  - portal-rs/crates/bazaar/src/game/authority_exchange.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/authority_wire.rs
  - portal-rs/crates/bazaar/src/game/invalidation_tests.rs
  - portal-rs/crates/bazaar/src/game/recovery_tests.rs
  - portal-rs/crates/bazaar/src/game/runtime/characters.rs
  - portal-rs/crates/bazaar/src/game/runtime/gold.rs
  - portal-rs/crates/bazaar/src/game/runtime/invalidation.rs
  - portal-rs/crates/bazaar/src/game/runtime/mod.rs
  - portal-rs/crates/bazaar/src/game/runtime/worker.rs
  - portal-rs/crates/characters/src/restore/storage.rs
  - portal-rs/crates/characters/src/restore/tests.rs
  - portal-rs/crates/characters/src/services/mod.rs
  - portal-rs/crates/characters/src/services/tests.rs
  - portal-rs/crates/community/src/referrals/campaign.rs
  - portal-rs/crates/community/src/referrals/tests.rs
  - portal-rs/crates/identity/src/account_lifecycle_http.rs
  - portal-rs/crates/identity/src/security_revocations_tests.rs
  - portal-rs/crates/identity/src/termination.rs
  - portal-rs/crates/identity/src/termination/README.md
  - portal-rs/crates/identity/src/termination/fixtures/SOURCE.md
  - portal-rs/crates/identity/src/termination/fixtures/pending.json
  - portal-rs/crates/identity/src/termination/fixtures/read.json
  - portal-rs/crates/identity/src/termination/fixtures/watermark.json
  - portal-rs/crates/identity/src/termination/game_closure.rs
  - portal-rs/crates/identity/src/termination/game_closure/http.rs
  - portal-rs/crates/identity/src/termination/game_closure/runtime.rs
  - portal-rs/crates/identity/src/termination/game_closure/tests.rs
  - portal-rs/crates/identity/src/termination/game_closure/wire.rs
  - portal-rs/crates/identity/src/termination_tests.rs
  - portal-rs/crates/minigames/src/league/mod.rs
  - portal-rs/crates/minigames/src/league/runtime.rs
  - portal-rs/crates/minigames/src/league/store.rs
  - portal-rs/crates/minigames/src/league/tests.rs
  - portal-rs/crates/playerdata/src/authority.rs
  - portal-rs/crates/playerdata/src/authority/reads.rs
  - portal-rs/crates/playerdata/src/authority/tests.rs
  - portal-rs/crates/playerdata/src/deaths/storage.rs
  - portal-rs/crates/playerdata/src/logins.rs
  - portal-rs/crates/playerdata/src/logins/tests.rs
  - portal-rs/crates/playerdata/src/retention.rs
  - portal-rs/crates/playerdata/src/store.rs
  - portal-rs/crates/stats/src/tests.rs
  - portal-rs/crates/stats/src/worker.rs
  - portal-rs/crates/store/src/delivery.rs
  - portal-rs/crates/store/src/delivery_tests.rs
  - portal-rs/crates/store/src/purchase_repository.rs
  - portal-rs/crates/store/src/purchase_tests.rs
  - portal-rs/crates/wallet/src/ledger.rs
  - portal-rs/migrations/20261118120000_security_revocations_future_suspension.sql
validation:
  - command: cargo fmt --all --check; cargo clippy --workspace --all-targets -- -D warnings
    result: PASS
    evidence: integrated tree
  - command: cargo test per touched crate with PostgreSQL 18
    result: PASS
    evidence: local PostgreSQL 18 on the integrated tree
  - command: Portal RS CI on the PR head
    result: NOT_RUN
    evidence: full suite runs in CI shards
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: server-side fixes covered by tests through the real routers and workers; portal-rs E2E dispatched after merge
blockers:
  - none
next_action: Portal RS CI on the PR head, then squash auto-merge
```
