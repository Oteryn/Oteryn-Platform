---
task_id: OTERYN-20261107-catalog-repin-669d5af5
governing_issue: 530
required_reads:
  - portal-rs/crates/world/README.md
  - portal-rs/crates/world/CATALOG_WIKI_REQUIREMENTS.md
search_first:
  - portal-rs/crates/world/tests/fixtures
optional_reads: []
---

# Re-pin CATALOG_EXPORT_V1 evidence to merged Game main

Governing Issue: https://github.com/Oteryn/Oteryn-Platform/issues/530.
The Game CATALOG lane (Oteryn-Game #3025) merged as
`669d5af5261a60d347167b7c880a4498aebe72ed`. This task replaces the lane pin
`fd4be4e214ef33ad65d3ce465dab7160377e62f5` with that merged revision.
Game access is read-only. Jira mapping unknown; synchronization pending.

## Goal and acceptance

Fixture directory, SOURCE note, vendored reference producer and docs point at the
merged revision with recomputed SHA-256 values. The negative artifact bytes and
SHA-256 stay unchanged because the contract and negative input are byte-identical.
No Rust behaviour, migration, workflow or deployment change.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-11-07T20:30:00Z
head: a60d84cc49e84eb46e6e5123cce23de12afd14bc
branch: claude/catalog-repin-669d5af5
pr: 1779
status: ready
context_routes:
  - testing
owned_paths:
  - portal-rs/crates/world/README.md
  - portal-rs/crates/world/CATALOG_WIKI_REQUIREMENTS.md
  - portal-rs/crates/world/tests/fixtures/catalog-669d5af5/**
  - portal-rs/crates/world/tests/support/catalog.rs
  - portal-rs/e2e/README.md
  - portal-rs/e2e/WORLD_READ_REPORT.md
  - portal-rs/e2e/local.mjs
  - portal-rs/e2e/support/catalog-producer.py
  - docs/agents/tasks/active/OTERYN-20261107-catalog-repin-669d5af5.md
proven:
  - Contract SHA-256 9bc87fba… and negative input SHA-256 d10b03da… are identical at fd4be4e2 and 669d5af5.
  - Merged producer SHA-256 865afe56… differs from the lane copy only by a nested data visibility==public check.
  - The Rust verifier already rejects non-public nested visibility (catalog/mod.rs and g2_visibility tests).
  - Merged notes still keep all eight native capabilities unsupported/unknown (notes lines 54–78).
derived:
  - The negative artifact SHA-256 4fe8ce44… is unchanged; the Rust support fixture asserts it.
unknown:
  - SHA-256 of the Game main CI artifact game-platform-catalog at 669d5af5 (requested from the coordinator)
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - portal-rs/crates/world/**
  - portal-rs/e2e/README.md
  - portal-rs/e2e/WORLD_READ_REPORT.md
  - portal-rs/e2e/local.mjs
  - portal-rs/e2e/support/catalog-producer.py
validation:
  - command: cargo test -p oteryn-world
    result: PASS
    evidence: 53 passed, 0 failed; PostgreSQL 18.6 local
  - command: python3 tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: legacy token ratchet ok
blockers: []
next_action: land PR 1779 via squash auto-merge, then archive this packet
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR path; PR 1779 squash-merged as 83fe20bf
source_branch_evidence: GET branches/claude/catalog-repin-669d5af5 returns 404 after the PR 1779 merge
```
