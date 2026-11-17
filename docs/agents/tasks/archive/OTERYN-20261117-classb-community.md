---
task_id: OTERYN-20261117-classb-community
governing_issue: 530
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-classb-community

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.

Round 4, class B, step 2, community. Referral invitations and the referral
campaign transaction helper opened writer transactions with a plain
`pool.begin()` and waited on the referral serialization lock (5600) without the
shared three-second mutation budget. Both now arm `oteryn_db::mutation_budget`
before the first statement. Other community writers already arm the budget
through the RBAC lock helpers; projection ingest workers keep their own policy.

## Acceptance criteria

- [x] A held referral lock sheds an invitation within the mutation budget (test red without the change, green with it)
- [x] Lock order and refusals unchanged
- [x] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-classb-community.md
  - portal-rs/crates/community/src/referrals.rs
  - portal-rs/crates/community/src/referrals/campaign.rs
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
updated_at: 2026-11-17T04:15:00Z
head: c69b998f8a1d471626e0a993d873ee9c1d6a5acf
branch: claude/happy-davinci-ijz3wy-classb-community
pr: 1904
status: completed
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-classb-community.md
  - portal-rs/crates/community/src/referrals.rs
  - portal-rs/crates/community/src/referrals/campaign.rs
  - portal-rs/crates/community/src/referrals_tests.rs
proven:
  - No migration, contract, fixture, flag or wire byte change
derived:
  - The site list for later crates is the class B inventory on Issue 530; sites whose first statement is an rbac lock helper already arm the budget
unknown:
  - Coordinator Jira synchronization
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-classb-community.md
  - portal-rs/crates/community/src/referrals.rs
  - portal-rs/crates/community/src/referrals/campaign.rs
  - portal-rs/crates/community/src/referrals_tests.rs
validation:
  - command: cargo test --locked -p oteryn-community --lib held_referral (PostgreSQL 18), without and with the change
    result: PASS
    evidence: postgres_held_referral_lock_sheds_invite_within_mutation_budget fails at the 10 s wait without the change and passes with it
  - command: cargo test --locked -p oteryn-community (PostgreSQL 18)
    result: PASS
    evidence: 132+13+1+4+8+16+9 passed, 0 failed
  - command: cargo fmt --all --check; cargo clippy --locked -p oteryn-community --all-targets -- -D warnings
    result: PASS
    evidence: exit 0
blockers:
  - none
next_action: none; delivered by PR 1904 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR 1904 squash-merged through the merge queue; GitHub deletes the head branch after merge
source_branch_evidence: PR 1904 merged as c69b998f8a1d471626e0a993d873ee9c1d6a5acf
```
