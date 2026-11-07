---
task_id: OTERYN-20261107-portal-rs-world-read
governing_issue: 530
required_reads:
  - docs/architecture/OTERYN_PORTAL_RUST_PLAN.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
search_first:
  - portal-rs/crates/world
  - portal-rs/crates/wiki
optional_reads: []
---

# Pinned world read cache and production wiki publication

Owner's latest coordinator update governs scope. Governing Issue:
https://github.com/Oteryn/Oteryn-Platform/issues/530 (open at resume).
One writer on `codex/portal-rs-world-read`; no PR, workflow edits or deployment.
Game access is explicitly read-only. Jira mapping unknown; synchronization pending.

## Goal and acceptance

Verify CATALOG_EXPORT_V1 from an explicit path/URL and independent SHA/provenance
fences, rebuild the cache transactionally, and opt-in publish complete categories
through the existing independent admin visibility gate. Refresh must not delay
startup/readiness. No raw Game content fallback, Platform native adapter, atlas
rendering or Today producer. Areas await Game's later additive export revision.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-07T16:10:13Z
head: 2a224efb54ac117f5e11e85d206006f841b68da6
branch: claude/portal-rs-world-read
pr: 1771
status: ready
context_routes:
  - database
  - testing
owned_paths:
  - portal-rs/**
  - docs/agents/tasks/active/OTERYN-20261107-portal-rs-world-read.md
proven:
  - Game lane resolves to fd4be4e214ef33ad65d3ce465dab7160377e62f5; re-pin on merge.
  - G-2 omission is resolved; optional explicit visibility accepts only public and otherwise rejects the whole artifact.
  - Lane notes explicitly keep all eight native capabilities unsupported/unknown; no populated native fixture is claimed.
  - Production publication uses a dedicated wiki pointer, complete-category cache records and independent admin approval.
  - Existing audited staff visibility changes now accept the native pointer; native legacy publication and name replacement are refused.
  - Slow URL probe returned healthz 200 in 0.066s, readyz 200 in 0.092s, wiki 503 in 0.138s after the stalled GET.
  - Atlas still needs a Game-side atlas export; Today G-12 is PROPOSED without a producer.
derived: []
unknown:
  - Game CATALOG merge revision and future capability-payload promotion schedule
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - raw content layout may substitute for an unsupported export capability
changed_paths:
  - portal-rs/bin/oteryn-portal/src/main.rs
  - portal-rs/crates/world/**
  - portal-rs/crates/wiki/src/entities*
  - portal-rs/migrations/20261107120003_world_catalog_publication.*.sql
  - portal-rs/templates/admin/world/index.html
  - portal-rs/e2e/**
validation:
  - command: cargo fmt --all --check
    result: PASS
    evidence: Update 5 material Rust tree
  - command: cargo clippy --locked --workspace --all-targets -- -D warnings
    result: PASS
    evidence: /tmp/portal-world-read-u5-clippy.log
  - command: cargo test --locked -p oteryn-world --test catalog --test catalog_refresh --lib
    result: PASS
    evidence: 42 tests passed; retry recovery, stored fast path, bounded refusal, collision validation, rollback isolation and identity approval semantics
  - command: cargo test --locked --no-fail-fast
    result: PASS
    evidence: 1123 passed, 0 failed, 3 ignored across 83 targets; PostgreSQL 18.6, locks 1024, connections 300; /tmp/portal-world-read-u5-full.log
  - command: node run.mjs /tmp/portal-world-read-u5-e2e/env.json test specs/world-read.spec.mjs
    result: PASS
    evidence: 8 passed, PL/EN at 1440px/390px; /tmp/portal-world-read-u5-browser.log
blockers: []
next_action: push the qualified Update 5 successor to the owner-requested branch for coordinator review; retain branch and re-pin when CATALOG merges
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Owner requested branch-only delivery for coordinator review; re-pin fixtures when CATALOG merges.
source_branch_evidence: latest owner brief; no PR requested
```

Prior checkpoints `faa77e57` and `6d6176db` qualified the inactive consumer,
source transport/cache, stats picker, Atlas flag aliases and Today UUID/docs.
The former G-2 mapping issue is resolved by the latest owner decision and reachable
lane evidence. Current positive tests use synthetic input only, through the
production publisher and real wiki routes; the audited admin route also has
PostgreSQL coverage. No supported native artifact is fabricated.

Update 5 scope is limited to the owner's five review items. Bounded source retries
use configurable initial/capped intervals and attempt count, exponential jitter,
and a whole-attempt deadline. A stored publication matching every source fence
and SHA-256 becomes ready without producer I/O. A loot entity/relation key
collision rejects the whole artifact at validation. Publication rollback deletes
only catalogue-owned versions and dependents; legacy rows/approvals remain.
Visibility intentionally remains stable-identity scoped, including legacy to
catalogue transitions, with an explicit regression and documentation. The world
picker remains required by corrected brief item 4 and the coordinator's first
update; no stats code is changed by Update 5.

Current qualification, source evidence and exact changed-file inventory are in
`portal-rs/e2e/WORLD_READ_REPORT.md`. Update 5 passes fmt, strict workspace
Clippy, full Rust suite (1123 passed / 0 failed / 3 ignored, 83 targets), and
world-read E2E (8/8). Full Rust qualification ran before starting E2E services,
with PostgreSQL 18.6, max_locks_per_transaction=1024, max_connections=300.

Cleanup complete: task portal/worker/producers stopped; disposable PostgreSQL
databases/container/anonymous volume and scratch certificates/keys/auth/env
removed. Shared environment containers remain stopped. Build caches and
non-secret validation logs are intentionally retained. No PR, workflow changes
or deployment; owner-requested branch retention remains in effect.
