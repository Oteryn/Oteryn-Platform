---
task_id: OTERYN-20261117-community-fence
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-community-fence

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Round 4, class B, community crate. Public guild, house, ranking and profile
pages ran their read fence (`FOR SHARE` on `public_projection_state`) on a
plain transaction with no lock budget. Ingest holds that row `FOR UPDATE`, so a
page waited for the ingest without bound. The fences now begin through
`privacy::read_fence`, which sets `lock_timeout` 1 s and `statement_timeout`
2.5 s, and shed as unavailable. The referral claim took the invitation row
`FOR UPDATE` before any budget; it now arms `mutation_budget` right after
`begin`. A crate guard requires every `begin()` to arm a budget or be listed
with its reason.

## Acceptance criteria

- [x] A held projection lock sheds the guild page as 503 within the budget (red without the change)
- [x] A held invitation lock sheds a referral claim within the mutation budget (red without the change)
- [x] Crate guard: every begin() is budgeted or listed with a reason (red on main)
- [x] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-community-fence.md
  - portal-rs/crates/community/src/begin_guard_tests.rs
  - portal-rs/crates/community/src/lib.rs
  - portal-rs/crates/community/src/privacy.rs
  - portal-rs/crates/community/src/guilds_pages.rs
  - portal-rs/crates/community/src/guilds_tests.rs
  - portal-rs/crates/community/src/houses.rs
  - portal-rs/crates/community/src/rankings.rs
  - portal-rs/crates/community/src/profiles.rs
  - portal-rs/crates/community/src/referrals.rs
  - portal-rs/crates/community/src/referrals_tests.rs
modules:
  - community
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
updated_at: 2026-11-17T06:50:00Z
head: a6b97122a8ba252d5e876e7493bfbe8920b8f92c
branch: claude/happy-davinci-ijz3wy-community-fence
pr: 1910
status: completed
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-community-fence.md
  - portal-rs/crates/community/src/begin_guard_tests.rs
  - portal-rs/crates/community/src/lib.rs
  - portal-rs/crates/community/src/privacy.rs
  - portal-rs/crates/community/src/guilds_pages.rs
  - portal-rs/crates/community/src/guilds_tests.rs
  - portal-rs/crates/community/src/houses.rs
  - portal-rs/crates/community/src/rankings.rs
  - portal-rs/crates/community/src/profiles.rs
  - portal-rs/crates/community/src/referrals.rs
  - portal-rs/crates/community/src/referrals_tests.rs
proven:
  - No migration, contract, fixture, flag or wire byte change
  - FOR SHARE stays; privacy_cut.validate takes its own FOR SHARE in the same fence, so one transaction budget covers both locks
  - Ingest apply, the referral campaign worker and RBAC or active_in bounded paths are listed in the guard with their reasons
derived:
  - The same unbudgeted fence exists in stats (online/storage.rs, worker.rs); separate PR per the one-crate rule
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - REPEATABLE READ instead of FOR SHARE would not remove the privacy revision wait
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-community-fence.md
  - portal-rs/crates/community/src/begin_guard_tests.rs
  - portal-rs/crates/community/src/lib.rs
  - portal-rs/crates/community/src/privacy.rs
  - portal-rs/crates/community/src/guilds_pages.rs
  - portal-rs/crates/community/src/guilds_tests.rs
  - portal-rs/crates/community/src/houses.rs
  - portal-rs/crates/community/src/rankings.rs
  - portal-rs/crates/community/src/profiles.rs
  - portal-rs/crates/community/src/referrals.rs
  - portal-rs/crates/community/src/referrals_tests.rs
validation:
  - command: cargo test --locked -p oteryn-community --lib postgres_guild_ingest_privacy_replay_and_fence (PostgreSQL 18), without and with the change
    result: PASS
    evidence: without the change the guild page stalls past the 10 s bound at guilds_tests.rs:195; with it the page answers 503 within 3 s
  - command: cargo test --locked -p oteryn-community --lib referral_claim_lock_wait_is_bounded_by_mutation_budget (PostgreSQL 18), without and with the change
    result: PASS
    evidence: without the change the claim stalls past 10 s; with it the claim is refused within 6 s and no reward is paid
  - command: cargo test --locked -p oteryn-community --lib every_transaction_bounds_its_lock_waits, without and with the source change
    result: PASS
    evidence: without it the guard lists guilds_pages, houses, rankings, profiles and referrals as unbudgeted; with it the guard passes
  - command: cargo test --locked -p oteryn-community (PostgreSQL 18); cargo fmt -p oteryn-community --check; cargo clippy --locked -p oteryn-community --all-targets -- -D warnings
    result: PASS
    evidence: lib 134 passed, 0 failed; fmt and clippy exit 0
blockers:
  - none
next_action: none; delivered by PR 1910 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR 1910 squash-merged through the merge queue; GitHub deletes the head branch after merge
source_branch_evidence: PR 1910 merged as a6b97122a8ba252d5e876e7493bfbe8920b8f92c
```
