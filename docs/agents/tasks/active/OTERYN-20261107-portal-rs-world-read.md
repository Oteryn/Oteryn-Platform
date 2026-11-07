---
task_id: OTERYN-20261107-portal-rs-world-read
governing_issue: 530
required_reads:
  - docs/architecture/OTERYN_PORTAL_RUST_PLAN.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
  - portal-rs/crates/world/tests/fixtures/catalog-fd4be4e2/CONTRACT.md
  - portal-rs/crates/world/tests/fixtures/catalog-fd4be4e2/CATALOG-CONTRACT-NOTES.md
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
updated_at: 2026-11-07T15:37:57Z
head: 6d6176db693af85b9c8cc48d41c37cebf89e1382
branch: codex/portal-rs-world-read
pr: none
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
    evidence: final material tree
  - command: cargo clippy --locked --workspace --all-targets -- -D warnings
    result: PASS
    evidence: final material code; /tmp/portal-world-read-u4-clippy-final2.log
  - command: cargo test --locked --no-fail-fast
    result: PASS
    evidence: 1117 passed, 0 failed, 3 ignored across 82 targets; PostgreSQL 18.6, locks 1024, connections 300
  - command: node run.mjs /tmp/portal-world-read-u4-slow-final/env.json test specs/world-read.spec.mjs
    result: PASS
    evidence: 8 passed with stalled/unavailable artifact URL, wiki publication switch on
  - command: node run.mjs /tmp/portal-world-read-u4-qualified-e2e/env.json test specs/world-read.spec.mjs specs/public.spec.mjs
    result: PASS
    evidence: 12 passed across PL/EN and 1440px/390px on the final binary; /tmp/portal-world-read-u4-qualified-browser.log
blockers: []
next_action: push the qualified successor on the owner-requested branch for coordinator review; re-pin when CATALOG merges
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

Qualification, exact changed-file inventory and source line evidence are in
`portal-rs/e2e/WORLD_READ_REPORT.md`. Final full Rust qualification passed 1117
with 0 failures and 3 ignored; strict Clippy/fmt pass. Normal browser 12/12 and
stalled/unavailable URL browser 8/8 pass. The first broad run had two binary
startup collisions with E2E private listeners; the final full Rust suite ran in
isolation and both startup tests passed. Required wiki fields are checked rather
than inventing presentation names from keys.

Cleanup complete: task portal/worker/producers, slow URL server, disposable
PostgreSQL databases/container/anonymous volume and scratch certificates/keys/env
were removed. Shared environment containers remain in their initial stopped state.
Build caches and non-secret validation logs remain for review. Owner requested
branch retention; no PR or integration action is authorized here.
