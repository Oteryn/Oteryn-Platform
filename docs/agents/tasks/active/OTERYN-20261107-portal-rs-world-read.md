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
updated_at: 2026-11-07T13:39:22Z
head: UNKNOWN
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
  - Locked export at Game d34807f5 allows inactive candidates only and all real fixture capabilities are unsupported/unknown.
  - No accepted capability payload or production publication profile permits public native entities.
  - Atlas needs a Game-side atlas export; Game asset catalogue is empty at d34807f5.
  - Today G-12 remains PROPOSED with no producer.
  - G-2 source visibility is absent; admin allowlist remains mandatory.
derived: []
unknown: []
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
    evidence: 1111 passed, 0 failed, 3 ignored; PostgreSQL 18.6, max_locks_per_transaction=1024, max_connections=300
  - command: cargo fmt --all --check
    result: PASS
    evidence: final material tree
  - command: cargo clippy --locked --workspace --all-targets -- -D warnings
    result: PASS
    evidence: final material tree, no warnings
  - command: node run.mjs /tmp/portal-world-read-e2e-final/env.json test specs/world-read.spec.mjs specs/public.spec.mjs
    result: PASS
    evidence: 12 passed across four locale/viewport projects; existing registration fixture prepared with repository helper
blockers: []
next_action: coordinator review of branch-only delivery and documented Game source gaps
```

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: Owner requested branch-only delivery for coordinator review; retain until coordinator disposition.
source_branch_evidence: corrected owner brief
```

Qualification and complete changed-file inventory: `portal-rs/e2e/WORLD_READ_REPORT.md`. Task-owned databases, container/volume, services and generated keys were cleaned up.
