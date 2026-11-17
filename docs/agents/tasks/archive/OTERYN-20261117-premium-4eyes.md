---
task_id: OTERYN-20261117-premium-4eyes
governing_issue: 1928
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261117-premium-4eyes

## Goal

Governing GitHub Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1928
(task D2). Plan §5.4 four eyes, second part: a staff premium grant needs a
second staff member's approval.

1. Premium: `Ledger::mutate_digest_in` takes the reason as its SHA-256
   digest; `mutate_in` and the referral grant hash the trimmed reason and
   delegate, so both paths record the same ledger digest.
2. Commerce: `Command::Grant` requires approval (Revoke stays direct). Submit
   refuses a target that is not value-eligible and audits the denial; apply
   checks eligibility again. Approval applies the grant from the stored
   reason hash, decoded once (bad hex or length is `Invalid`).
3. Staff page: the approval queue lists pending grants with their days.
4. E2E: `staff-commerce-premium` approves the grant as a second staff
   identity seeded like `admin.spec.mjs`.

## Acceptance criteria

- [x] Grant submit is pending with no entitlement (`postgres_grant_submit_is_pending_without_entitlement`)
- [x] The proposer cannot approve its own grant (`postgres_grant_proposer_cannot_approve_own_grant`)
- [x] A second staff member's approval creates the entitlement with ledger digest sha256(reason) (`postgres_grant_second_staff_approval_records_reason_digest`)
- [x] A replayed approval is idempotent (`postgres_grant_approval_replay_is_idempotent`)
- [x] A grant to a non-value-eligible account is refused and audited at submit (`postgres_grant_to_value_ineligible_account_refused_at_submit`)
- [x] Stored reason hash decoding refuses bad hex and wrong length (`reason_digest_decodes_only_a_full_hex_sha256`)
- [x] Portal RS CI green on the PR head

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-premium-4eyes.md
  - portal-rs/crates/store/src/premium/repository.rs
  - portal-rs/crates/admin/src/commerce/README.md
  - portal-rs/crates/admin/src/commerce/effects.rs
  - portal-rs/crates/admin/src/commerce/http.rs
  - portal-rs/crates/admin/src/commerce/mod.rs
  - portal-rs/crates/admin/src/commerce/tests.rs
  - portal-rs/e2e/specs/staff-commerce-premium.spec.mjs
  - docs/architecture/OTERYN_PORTAL_RUST_PLAN.md
modules:
  - admin
  - store
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
updated_at: 2026-11-17T13:57:19Z
head: 5510ab0a840a009eecdab01fd988823a258d1f30
branch: claude/happy-davinci-ijz3wy-premium-4eyes
pr: 1933
status: completed
terminal_pr_policy: archive_pending
context_routes: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20261117-premium-4eyes.md
  - portal-rs/crates/store/src/premium/repository.rs
  - portal-rs/crates/admin/src/commerce/README.md
  - portal-rs/crates/admin/src/commerce/effects.rs
  - portal-rs/crates/admin/src/commerce/http.rs
  - portal-rs/crates/admin/src/commerce/mod.rs
  - portal-rs/crates/admin/src/commerce/tests.rs
  - portal-rs/e2e/specs/staff-commerce-premium.spec.mjs
proven:
  - The five new PostgreSQL grant tests fail on the base tree (b2400627) and pass with the change (PostgreSQL 18)
  - No migration, contract, fixture, flag or workflow change
derived:
  - Commerce trims the reason before hashing and premium hashes the trimmed reason, so digest and plaintext entries record identical ledger digests
  - The cancellation route has no command-kind filter; only the queue query needed a grant arm
unknown:
  - Coordinator Jira synchronization
  - E2E staff-commerce-premium run against a live portal
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Keeping plaintext reasons for approval; the commerce table stores only the reason hash by design
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261117-premium-4eyes.md
  - portal-rs/crates/store/src/premium/repository.rs
  - portal-rs/crates/admin/src/commerce/README.md
  - portal-rs/crates/admin/src/commerce/effects.rs
  - portal-rs/crates/admin/src/commerce/http.rs
  - portal-rs/crates/admin/src/commerce/mod.rs
  - portal-rs/crates/admin/src/commerce/tests.rs
  - portal-rs/e2e/specs/staff-commerce-premium.spec.mjs
validation:
  - command: cargo test -p oteryn-admin --lib commerce (PostgreSQL 18)
    result: PASS
    evidence: 33 passed, 0 failed, no skip lines in 2 of 3 full runs; 1 run hit a 57014 statement timeout in a3_commerce_queue_pagination_and_terminal_disposition (no global lock), which passed alone and in both reruns
  - command: cargo test -p oteryn-store --lib premium; --lib codes; cargo test -p oteryn-community --lib referrals (PostgreSQL 18)
    result: PASS
    evidence: 27, 15 and 57 passed, 0 failed
  - command: cargo fmt --all --check; cargo clippy -p oteryn-admin -p oteryn-store --all-targets -- -D warnings
    result: PASS
    evidence: both exit 0
  - command: Playwright staff-commerce-premium.spec.mjs
    result: NOT_RUN
    evidence: no live portal in this session; node --check passes
blockers:
  - none
next_action: none; delivered by PR 1933 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR 1933 squash-merged through the merge queue; GitHub deletes the head branch after merge
source_branch_evidence: PR 1933 merged as 5510ab0a840a009eecdab01fd988823a258d1f30
```
