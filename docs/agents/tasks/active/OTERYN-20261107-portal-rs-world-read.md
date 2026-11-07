---
task_id: OTERYN-20261107-portal-rs-world-read
governing_issue: 530
required_reads:
  - docs/architecture/OTERYN_PORTAL_RUST_PLAN.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
  - portal-rs/crates/world/tests/fixtures/catalog-d34807f5/CONTRACT.md
search_first:
  - portal-rs/crates/world
  - portal-rs/crates/atlas
optional_reads: []
---

# Pinned world read cache

Owner's latest Game-coordinator update governs scope and supersedes raw content reading. Programme: https://github.com/Oteryn/Oteryn-Platform/issues/530 (open at admission). Branch only; no PR, workflow edits or deployment. Prior uncommitted import CLI draft discarded. Jira mapping unknown; synchronization pending.

## Goal and acceptance

Verify the locked CATALOG_EXPORT_V1 offline artifact into an immutable inactive read-cache candidate, preserve admin approval and all unavailable-source gates, expose the registry world picker, and qualify negative native evidence. No raw Game layout reader, tile rendering, Today producer, Game writes or runtime fetches.

## Ownership

Owned: portal-rs native catalog consumer, small Atlas flag aliases, Today documentation/UUID configuration, world stats and focused E2E support/specs; this packet. One writer. PR #1763 merged at the branch base 3d6a1ae2; account spec is untouched and README edits are bounded to world provisioning. Main later advanced to 682a764f with only the old local-E2E packet archival; affected runtime and governing instructions are unchanged. Game repository access is explicitly read-only at the owner-specified commit.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-07T14:48:22Z
head: faa77e570704834c8d148eedd5bfe5e4a30c5e33
branch: codex/portal-rs-world-read
pr: none
status: waiting
context_routes:
  - database
  - testing
owned_paths:
  - portal-rs/**
  - docs/agents/tasks/active/OTERYN-20261107-portal-rs-world-read.md
proven:
  - Locked export at Game d34807f5 allows inactive candidates only and all real fixture capabilities are unsupported/unknown.
  - No accepted capability payload or production publication profile permits public native entities.
  - Atlas needs a Game-side atlas export; Game asset catalogue is empty at d34807f5.
  - Today G-12 remains PROPOSED with no producer.
  - G-2 is accepted, but exported field placement/omission semantics are not available at the pinned merged source.
  - The supplied Game CATALOG branch returned 404; clarification requested.
derived: []
unknown:
  - Exact G-2 exported field path and omission semantics; supplied Game ref is not reachable
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - raw content layout is an accepted native integration contract
changed_paths:
  - portal-rs/bin/oteryn-portal/src/main.rs
  - portal-rs/crates/world/**
  - portal-rs/crates/atlas/src/**
  - portal-rs/crates/stats/src/world/**
  - portal-rs/crates/community/src/today/README.md
  - portal-rs/templates/stats/world/**
  - portal-rs/migrations/20261107120001_world_catalog_candidates.*.sql
  - portal-rs/e2e/**
validation:
  - command: cargo test --locked --no-fail-fast
    result: PASS
    evidence: 1115 passed, 0 failed, 3 ignored; PostgreSQL 18.6, max_locks_per_transaction=1024, max_connections=300
  - command: cargo fmt --all --check
    result: PASS
    evidence: final material tree
  - command: cargo clippy --locked --workspace --all-targets -- -D warnings
    result: PASS
    evidence: final material tree, no warnings
  - command: node run.mjs /tmp/portal-world-read-u3-e2e/env.json test specs/world-read.spec.mjs specs/public.spec.mjs
    result: PASS
    evidence: 12 passed across four locale/viewport projects; existing registration fixture prepared with repository helper
blockers:
  - G-2 exported field mapping/omission policy unavailable; coordinator question pending
next_action: obtain reachable Game CATALOG producer/schema pin and exact G-2 encoding, then implement marked/omitted visibility tests
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Owner requested branch-only delivery for coordinator review; retain until coordinator disposition.
source_branch_evidence: corrected owner brief
```

Qualification and complete changed-file inventory: `portal-rs/e2e/WORLD_READ_REPORT.md`. Task-owned databases, container/volume, services and generated keys were cleaned up.

Update 2: add supported-carrier cache construction and capability-complete read checks; test-only publication from Game producer synthetic vector proves wiki/search HTTP 200. Production activation remains unavailable. Exact wiki adapter scope is in `portal-rs/crates/world/CATALOG_WIKI_REQUIREMENTS.md`.

Update 3: accepted catalog/G-2 decisions; configured SHA-256 required for local files and URLs; HTTPS bounded GET with no redirects; generated artifacts removed from the tree and generated only in test/scratch. No additional Atlas changes. Exact G-2 path and absent-field semantics requested because the supplied unmerged Game branch returns 404.

Current continuation: uncommitted Updates 2/3 extend supported carrier cache, URL/file + independent SHA transport, remove committed generated artifacts, and add test-only synthetic supported cache -> wiki/search 200. G-2 field path/omission semantics remain REQUIRED and requested via async question; the supplied Game ref returned 404. Do not guess an export layout or claim G-2 completion. Production remains inactive.

Validation logs: `/tmp/portal-world-read-u3-focused.log` contains passing cache PostgreSQL, synthetic wiki 200 and 12 catalogue unit tests. `/tmp/portal-world-read-u3-clippy.log` is clean after test lints. Current full run `/tmp/portal-world-read-u3-tests-final2.log` (exec session 84858) validates migration fixture fixes but started before the last per-snapshot FOR UPDATE lock/concurrent-refresh regression; run final validation after this run finishes. Seven old Admin/Content fixtures incorrectly executed down migrations; now use `oteryn_db::migrate`. `crates/db/build.rs` tracks additions to the migrations directory.

Resource recovery: disk filled with task-built test ELFs. Auto-reviewed escalation removed 108 task-log-referenced, task-timestamped, inactive test executables, preserving sources/libraries/shared services. Workspace now has about 8 GiB free. Avoid additional feature-profile builds; use workspace tests. Task PostgreSQL: `oteryn-portal-world-read-pg18-u2`, label `portal-rs-world-read-update2`, loopback 15432, PostgreSQL 18.6 / locks 1024 / connections 300; databases `oteryn_test` and `portal_world_read_u3_final` are task-owned. No local portal currently runs. After final checks run the generated-negative-fixture local E2E harness, then clean only task-owned services/keys/container/volume.

Update 2/3 qualified checkpoint: fmt and strict Clippy pass; full Rust suite
1115 passed / 0 failed / 3 ignored; all 12 affected browser cases pass with the
scratch-generated negative export. Concurrent cache rebuild, rollback/repair,
URL pin/bounds/redirect refusal and test-only synthetic wiki 200 are exercised.
Task-owned U2/U3 PostgreSQL, processes, databases, volume and scratch keys were
removed after qualification. G-2 exact export decoding remains blocked and is not
claimed complete. Production publication stays inactive. All other described
consumer/cache/transport changes are reviewable. See WORLD_READ_REPORT.md.
