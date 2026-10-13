---
task_id: OTERYN-20261013-platform-pg-p6a-impl
governing_issue: 1474
required_reads:
  - docs/operations/PLATFORM_POSTGRES_TRUST_SAFEGUARDS_DESIGN.md
  - docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md
search_first:
  - app/GameAuth/NativeEvidence/NativeEvidenceHighWaterWitness.php
  - deploy/synology/scripts/lib.sh
optional_reads:
  - deploy/synology/scripts/generate-native-admission-key.sh
---

# OTERYN-20261013 Platform PostgreSQL P6a: trust safeguards implementation

## Goal

Governing GitHub Issue: #1474 (native-only migration) is the canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

Implement the accepted P6a design note (PR #1589, owner-accepted residual risks 2026-10-12): the provenance-only init, the read-only trust observation, the first-publication fence in the producer, the `attempt_evidence.py` writer and classifier, the `lib.sh` deploy guard, the evidence mounts and the kid-history check. No missing-profile repair, no cutover, no destructive Synology step. `cutover-postgres.sh` and the Cloudflare/cutover workflow belong to the P6b session and are not touched.

## Acceptance criteria

- [ ] Code and tests of the design note's *Files and tests* table are implemented.
- [ ] Bot user id 41898282 for `github-actions[bot]` is verified through the GitHub API: comment 5358096189 on issue #1183 carries `github-actions[bot]` with id 41898282.
- [ ] Exact-head CI (`platform-gate`) passes, the complete Codex review is addressed and no thread is unresolved.

## Ownership

```yaml
owned_paths:
  - app/GameAuth/NativeEvidence/
  - app/Console/Commands/
  - config/game-auth.php
  - tests/Feature/GameAuth/
  - tests/ci/test_synology_attempt_evidence.py
  - tests/ci/test_synology_native_admission_trust_publication.py
  - tests/ci/test_synology_native_admission_gate.py
  - deploy/synology/scripts/attempt_evidence.py
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/generate-native-admission-key.sh
  - deploy/synology/compose.yml
  - deploy/synology/runner/compose.yml
  - deploy/synology/README.md
  - docs/operations/PLATFORM_POSTGRES_TRUST_SAFEGUARDS_DESIGN.md
  - docs/agents/tasks/active/OTERYN-20261013-platform-pg-p6a-impl.md
  - docs/agents/tasks/archive/OTERYN-20261012-platform-pg-p6a.md
modules:
  - operations
  - database
dependencies:
  - PR 1589 (merged P6a design note)
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-13T00:30:00Z
head: UNKNOWN
branch: claude/platform-p6a-tooling-20261013
pr: none
status: implementing
context_routes:
  - operations
  - database
owned_paths:
  - app/GameAuth/NativeEvidence/
  - app/Console/Commands/
  - config/game-auth.php
  - tests/Feature/GameAuth/
  - tests/ci/
  - deploy/synology/
  - docs/agents/tasks/active/OTERYN-20261013-platform-pg-p6a-impl.md
proven:
  - github-actions[bot] has user id 41898282 (issue comment 5358096189 on #1183, read through the GitHub API).
derived: []
unknown:
  - CI result of the PHP tests (PHP runs only in CI for this task).
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - none
changed_paths:
  - deploy/synology/
  - tests/ci/
validation:
  - command: python3 -m pytest tests/ci
    result: PASS
    evidence: local run of the existing and new Python tests
blockers:
  - none
next_action: finish the PHP and Python tool, open the PR, run the full Codex review, then merge.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository implementation PR
source_branch_evidence: pending merge; verify the source ref disappears after merge
```
