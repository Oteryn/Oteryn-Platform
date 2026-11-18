---
task_id: OTERYN-20261118-r14-fixes
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261118-r14-fixes

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Review round 14: small-slice Codex reviews of portal-rs on the round 13
head. Confirmed P0-P2 findings are fixed by Codex Cloud packets with
red-then-green regression tests: Bazaar replay during recovery resync
(historical Character Lock, EXCESS release of the leader's hold, Character
Payout intent after restore), G-11 superseded-epoch retention, delayed records
of an earlier ownership period, login promotion feed-freshness ceiling,
zero-duration G-9 segments, and delayed CANCEL rejections after a
recovery-generation increase. Implements the owner decisions of 2026-11-18:
PREMIUM_TIME store purchase granted by the Platform premium ledger, BANK first
publication only under the announced recovery generation, expired Bazaar
Deliver bodies served during an unfinished resync, and the G-11 account
partition and stream identity. Also archives the round 13 packet after PR #1946
merged.

## Acceptance criteria

- [x] Each confirmed finding has a fix with a regression test through the real caller
- [x] Each pushed fix was reviewed read-only before integration
- [x] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261118-r13-fixes.md
  - docs/agents/tasks/active/OTERYN-20261118-r14-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261118-r13-fixes.md
  - docs/contracts/OTERYN_V2_BAZAAR_CONTRACT_PROPOSAL.md
  - docs/contracts/OTERYN_V2_GAME_JOURNAL_ITEMS_CONTRACT_PROPOSAL.md
  - docs/contracts/OTERYN_V2_GAME_JOURNAL_PROGRESS_CONTRACT_PROPOSAL.md
  - docs/contracts/OTERYN_V2_PREMIUM_TIME_STORE_PURCHASE_AMENDMENT_2026-11-18.md
  - portal-rs/bin/oteryn-portal/src/game_routes.rs
  - portal-rs/crates/bazaar/src/game/authority.rs
  - portal-rs/crates/bazaar/src/game/authority_exchange.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/recovery_tests.rs
  - portal-rs/crates/bazaar/src/game/runtime/mod.rs
  - portal-rs/crates/community/src/referrals/tests.rs
  - portal-rs/crates/identity/src/termination/game_closure.rs
  - portal-rs/crates/identity/src/termination/game_closure/tests.rs
  - portal-rs/crates/internal-api/src/playerdata/sessions.rs
  - portal-rs/crates/internal-api/tests/playerdata_sessions.rs
  - portal-rs/crates/playerdata/src/authority.rs
  - portal-rs/crates/playerdata/src/authority/reads.rs
  - portal-rs/crates/playerdata/src/authority/tests.rs
  - portal-rs/crates/playerdata/src/journal.rs
  - portal-rs/crates/playerdata/src/logins.rs
  - portal-rs/crates/playerdata/src/store.rs
  - portal-rs/crates/playerdata/src/store/tests.rs
  - portal-rs/crates/store/src/premium.rs
  - portal-rs/crates/store/src/premium/repository.rs
  - portal-rs/crates/store/src/purchase_accounts.rs
  - portal-rs/crates/store/src/purchase_repository.rs
  - portal-rs/crates/store/src/purchase_tests.rs
  - portal-rs/crates/store/src/web_tests.rs
modules:
  - bazaar
  - identity
  - internal-api
  - store
  - community
  - oteryn-portal
  - playerdata
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
updated_at: 2026-11-18T06:45:00Z
head: ca5a5fdda62662a6e153dc2aa6c33d5f44a206b1
branch: claude/happy-davinci-ijz3wy-r14-fixes
pr: 1947
status: completed
terminal_pr_policy: archive_pending
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261118-r13-fixes.md
  - docs/agents/tasks/active/OTERYN-20261118-r14-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261118-r13-fixes.md
  - docs/contracts/OTERYN_V2_BAZAAR_CONTRACT_PROPOSAL.md
  - docs/contracts/OTERYN_V2_GAME_JOURNAL_ITEMS_CONTRACT_PROPOSAL.md
  - docs/contracts/OTERYN_V2_GAME_JOURNAL_PROGRESS_CONTRACT_PROPOSAL.md
  - docs/contracts/OTERYN_V2_PREMIUM_TIME_STORE_PURCHASE_AMENDMENT_2026-11-18.md
  - portal-rs/bin/oteryn-portal/src/game_routes.rs
  - portal-rs/crates/bazaar/src/game/authority.rs
  - portal-rs/crates/bazaar/src/game/authority_exchange.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/recovery_tests.rs
  - portal-rs/crates/bazaar/src/game/runtime/mod.rs
  - portal-rs/crates/community/src/referrals/tests.rs
  - portal-rs/crates/identity/src/termination/game_closure.rs
  - portal-rs/crates/identity/src/termination/game_closure/tests.rs
  - portal-rs/crates/internal-api/src/playerdata/sessions.rs
  - portal-rs/crates/internal-api/tests/playerdata_sessions.rs
  - portal-rs/crates/playerdata/src/authority.rs
  - portal-rs/crates/playerdata/src/authority/reads.rs
  - portal-rs/crates/playerdata/src/authority/tests.rs
  - portal-rs/crates/playerdata/src/journal.rs
  - portal-rs/crates/playerdata/src/logins.rs
  - portal-rs/crates/playerdata/src/store.rs
  - portal-rs/crates/playerdata/src/store/tests.rs
  - portal-rs/crates/store/src/premium.rs
  - portal-rs/crates/store/src/premium/repository.rs
  - portal-rs/crates/store/src/purchase_accounts.rs
  - portal-rs/crates/store/src/purchase_repository.rs
  - portal-rs/crates/store/src/purchase_tests.rs
  - portal-rs/crates/store/src/web_tests.rs
proven:
  - Each round-14 finding was confirmed by a read-only check with file:line evidence
  - Each pushed fix was reviewed read-only against its packet before integration
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
  - docs/agents/tasks/active/OTERYN-20261118-r13-fixes.md
  - docs/agents/tasks/active/OTERYN-20261118-r14-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261118-r13-fixes.md
  - docs/contracts/OTERYN_V2_BAZAAR_CONTRACT_PROPOSAL.md
  - docs/contracts/OTERYN_V2_GAME_JOURNAL_ITEMS_CONTRACT_PROPOSAL.md
  - docs/contracts/OTERYN_V2_GAME_JOURNAL_PROGRESS_CONTRACT_PROPOSAL.md
  - docs/contracts/OTERYN_V2_PREMIUM_TIME_STORE_PURCHASE_AMENDMENT_2026-11-18.md
  - portal-rs/bin/oteryn-portal/src/game_routes.rs
  - portal-rs/crates/bazaar/src/game/authority.rs
  - portal-rs/crates/bazaar/src/game/authority_exchange.rs
  - portal-rs/crates/bazaar/src/game/authority_tests.rs
  - portal-rs/crates/bazaar/src/game/recovery_tests.rs
  - portal-rs/crates/bazaar/src/game/runtime/mod.rs
  - portal-rs/crates/community/src/referrals/tests.rs
  - portal-rs/crates/identity/src/termination/game_closure.rs
  - portal-rs/crates/identity/src/termination/game_closure/tests.rs
  - portal-rs/crates/internal-api/src/playerdata/sessions.rs
  - portal-rs/crates/internal-api/tests/playerdata_sessions.rs
  - portal-rs/crates/playerdata/src/authority.rs
  - portal-rs/crates/playerdata/src/authority/reads.rs
  - portal-rs/crates/playerdata/src/authority/tests.rs
  - portal-rs/crates/playerdata/src/journal.rs
  - portal-rs/crates/playerdata/src/logins.rs
  - portal-rs/crates/playerdata/src/store.rs
  - portal-rs/crates/playerdata/src/store/tests.rs
  - portal-rs/crates/store/src/premium.rs
  - portal-rs/crates/store/src/premium/repository.rs
  - portal-rs/crates/store/src/purchase_accounts.rs
  - portal-rs/crates/store/src/purchase_repository.rs
  - portal-rs/crates/store/src/purchase_tests.rs
  - portal-rs/crates/store/src/web_tests.rs
validation:
  - command: cargo fmt --all --check; cargo clippy --workspace --all-targets -- -D warnings
    result: PASS
    evidence: integrated tree
  - command: cargo test per crate with PostgreSQL 18
    result: PASS
    evidence: local PostgreSQL 18 - bazaar 189, identity 198, internal-api 19, playerdata 199 (Codex run after BANK change), store 141, community 194, oteryn-portal 249; 0 failed, 0 ignored
  - command: Portal RS CI on the PR head
    result: PASS
    evidence: all Portal RS CI jobs green on PR head 665b5e32; merged through the merge queue
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: server-side fixes covered by tests through the real routers and workers
blockers:
  - none
next_action: none; delivered by PR #1947 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR #1947 squash-merged through the merge queue; GitHub deletes the head branch after merge
source_branch_evidence: PR #1947 merged as ca5a5fdda62662a6e153dc2aa6c33d5f44a206b1
```
