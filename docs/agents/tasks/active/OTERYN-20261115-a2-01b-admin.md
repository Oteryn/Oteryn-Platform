---
task_id: OTERYN-20261115-a2-01b-admin
governing_issue: 536
required_reads: []
search_first: []
optional_reads: []
---

# Audit wave 2 01b-admin

Governing workstream: https://github.com/Oteryn/Oteryn-Platform/issues/536 (open,
verified 2026-11-15). The owner's task defines A01–A05, exact base, branch,
validation and push. Single writer, no subagents; no PR requested. Jira sync is
pending coordinator. Existing active Bazaar ownership is outside these paths.

Base: c6fdc5438a4afa91f32d547b7cfc61f22bf21139.
Branch: claude/happy-davinci-ijz3wy-fix-a2-01b-admin.
Owned paths: portal-rs/crates/{admin,content,payments,support,wiki,world},
portal-rs/templates/admin/roles.html and the task's additive migration.

All findings confirmed on the base. A01 prepares immutable account IDs in stable
order before RBAC. A02 composes control registration/results on the ingress
transaction. A03 binds forms and release receipts to immutable hold instances.
A04 wires an explicit audited staff attestation into the configured router and
retains evidence expiry, target binding and single-use enforcement. A05 uses
50-row keyset pages and verifies a prepared survivor under the fence.

Validation: all 418 tests passed with zero failures on Rust 1.97; one unchanged
browser test is ignored on the base. All six PostgreSQL regressions also passed
after the pre-fix behavior probes. Each probe failed as expected, and the fixed
sources were restored byte-for-byte. Formatting and Clippy pass for all seven
touched crates. No contract files, feature flags or workflow files changed.
Source branch disposition: retain for coordinator review of the owner-requested
fix branch. No PR, merge or deployment is authorized or claimed.


## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-15T17:33:39Z
head: c6fdc5438a4afa91f32d547b7cfc61f22bf21139
branch: claude/happy-davinci-ijz3wy-fix-a2-01b-admin
pr: none
status: ready
context_routes: []
owned_paths:
  - portal-rs/crates/admin/
  - portal-rs/crates/content/
  - portal-rs/crates/payments/
  - portal-rs/crates/support/
  - portal-rs/crates/wiki/
  - portal-rs/crates/world/
  - portal-rs/crates/wallet/src/ledger.rs
  - portal-rs/templates/admin/roles.html
  - portal-rs/migrations/20261115120002_fix_a2-01b-admin_hold_identity.sql
proven:
  - All A01-A05 findings were confirmed against the admitted base and repaired
  - All listed A01 mutation paths prepare their required account locks before RBAC
  - Commerce balance review uses the caller transaction through a small wallet helper
  - Hello and result ingress use one transaction connection including persistence
  - Hold release identity and durable retry receipts preserve replacement holds
  - Configured staff routers issue audited target-bound evidence and complete two-person reset
  - Role catalogue and survivor candidates use bounded keyset pages of 50
  - All six regression probes fail with the corresponding pre-fix behaviors restored
  - Fixed source snapshots were restored byte-for-byte and all six regressions pass
  - PostgreSQL validation ran against the isolated local test service with no production access
  - Repository identity Oteryn/Oteryn-Platform and open issue 536 were verified live
  - Contract bytes, default-off flags and GitHub workflows are unchanged
  - No PR was requested or created
  - No tests were weakened, removed or newly ignored and no new allow attributes were added
derived: []
unknown:
  - Coordinator Jira mapping and synchronization
conflicts: []
first_failure:
  marker: none
  evidence: pre-fix behavior probe failures are expected regression evidence
rejected_hypotheses:
  - The findings were already fixed on the admitted integration base
changed_paths:
  - portal-rs/crates/admin/src/commerce/effects.rs
  - portal-rs/crates/admin/src/commerce/mod.rs
  - portal-rs/crates/admin/src/control/http.rs
  - portal-rs/crates/admin/src/control/http_tests.rs
  - portal-rs/crates/admin/src/control/store.rs
  - portal-rs/crates/admin/src/players/http.rs
  - portal-rs/crates/admin/src/players/mod.rs
  - portal-rs/crates/admin/src/players/page.html
  - portal-rs/crates/admin/src/players/tests.rs
  - portal-rs/crates/admin/src/rbac/mod.rs
  - portal-rs/crates/admin/src/rbac/panel_tests.rs
  - portal-rs/crates/admin/src/rbac/storage.rs
  - portal-rs/crates/admin/src/support.rs
  - portal-rs/crates/admin/src/web.rs
  - portal-rs/crates/admin/src/web_tests.rs
  - portal-rs/crates/content/src/announcements.rs
  - portal-rs/crates/content/src/events/repository.rs
  - portal-rs/crates/payments/src/coin_store.rs
  - portal-rs/crates/support/src/sanctions/store.rs
  - portal-rs/crates/support/src/tickets/staff.rs
  - portal-rs/crates/support/src/tickets/staff_tests.rs
  - portal-rs/crates/wallet/src/ledger.rs
  - portal-rs/crates/wiki/src/repository.rs
  - portal-rs/crates/world/src/admin.rs
  - portal-rs/templates/admin/roles.html
  - portal-rs/migrations/20261115120002_fix_a2-01b-admin_hold_identity.sql
  - docs/agents/tasks/active/OTERYN-20261115-a2-01b-admin.md
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: installed Rust 1.97 repository pin; no tools installed
  - command: cargo clippy --locked --all-targets -p oteryn-admin -p oteryn-content -p oteryn-payments -p oteryn-support -p oteryn-wiki -p oteryn-world -p oteryn-wallet -- -D warnings
    result: PASS
    evidence: all seven touched crates on Rust 1.97
  - command: cargo test --locked -p oteryn-admin -p oteryn-content -p oteryn-payments -p oteryn-support -p oteryn-wiki -p oteryn-world -p oteryn-wallet -- --test-threads=1 --nocapture
    result: PASS
    evidence: 418 passed, zero failed, one unchanged pre-existing ignored browser test; excludes nested support subprocess recount
  - command: six focused PostgreSQL regressions after restoring fixed source snapshots
    result: PASS
    evidence: A01 1.657s; A02 hello 1.875s and result 1.707s; A03 1.938s; A04 1.931s; A05 2.372s
  - command: configured production actor-to-result routers
    result: PASS
    evidence: stale hold form and receipt replay, evidence issuance and two-person reset use guarded production routers
blockers: []
next_action: Coordinator reviews the owner-requested fix branch; no PR or merge is requested
```

The checkpoint head records the admitted base. Git and the final response report
the publication commit; this packet does not embed a self-referential SHA. No PR
exact-head gate is claimed because the owner explicitly requested a branch push
without a PR. The source branch is retained for coordinator review.
