---
task_id: OTERYN-20261005-heavy-evidence-reuse
governing_issue: 1012
required_reads:
  - docs/agents/BUILD_TEST_MATRIX.md
search_first:
  - checkpoint-only heavy workflow reuse
optional_reads: []
---

# OTERYN-20261005 heavy workflow evidence reuse — terminal archive

## Goal

Implement Issue #1012 without weakening required protection: a later checkpoint/docs-only commit may reuse a prior successful heavy-workflow result only when the same PR has directly attributable successful evidence for an ancestor head and the non-governance/material repository tree is provably byte-identical.

## Acceptance criteria

- [x] Distinguish the latest commit scope from the accumulated PR delivery scope.
- [x] Fail closed to heavy validation for material/runtime/workflow/classifier changes.
- [x] Reuse prior successful heavy evidence only for same-PR exact ancestor heads with byte-identical material tree.
- [x] Keep lightweight exact-final-head governance and aggregate required checks.
- [x] Cover material -> heavy, checkpoint successor -> reuse, material-after-checkpoint -> heavy, ambiguous state -> heavy.
- [x] Record reused material head/run IDs in workflow summaries and terminal evidence.
- [x] Preserve security/auth/database/production-like required semantics.

## Ownership

```yaml
owned_paths:
  - scripts/ci/heavy_evidence_reuse.py
  - tests/ci/test_heavy_evidence_reuse.py
  - tests/ci/test_workflow_trigger_economy.py
  - .github/workflows/ci.yml
  - .github/workflows/edge-security-emulation.yml
  - .github/workflows/platform-db-outage-validation.yml
  - .github/workflows/game-auth-ticket-concurrency.yml
  - .github/workflows/phase7-production-like-validation.yml
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/tasks/active/OTERYN-20261005-heavy-evidence-reuse.md
modules:
  - CI routing
  - heavy workflow evidence reuse
dependencies:
  - GitHub Actions read-only workflow/run metadata
blockers: []
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-05T19:05:00+02:00
head: b1e48bcfeb9ed114f112d69ac98914bf0b196874
branch: ci/issue-1012-heavy-evidence-reuse
pr: 1453
status: completed
terminal_pr_policy: archive_pending
context_routes:
  - ci
owned_paths:
  - scripts/ci/heavy_evidence_reuse.py
  - tests/ci/test_heavy_evidence_reuse.py
  - tests/ci/test_workflow_trigger_economy.py
  - .github/workflows/ci.yml
  - .github/workflows/edge-security-emulation.yml
  - .github/workflows/platform-db-outage-validation.yml
  - .github/workflows/game-auth-ticket-concurrency.yml
  - .github/workflows/phase7-production-like-validation.yml
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/tasks/active/OTERYN-20261005-heavy-evidence-reuse.md
proven:
  - Issue #1012 is open with no existing branch or PR owner.
  - current heavy workflows classify the accumulated PR base-to-head range, so a later docs/checkpoint commit can still emit unchanged heavy internals.
  - material implementation head 6339b0b8648bfe6176ff223848e10e59cec37aa9 passed core CI runtime-tests in run 37344020823
  - material implementation head 6339b0b8648bfe6176ff223848e10e59cec37aa9 passed Edge Security Emulation run 37344020756
  - material implementation head 6339b0b8648bfe6176ff223848e10e59cec37aa9 passed Platform DB Outage Validation run 37344020734
  - material implementation head 6339b0b8648bfe6176ff223848e10e59cec37aa9 passed Game Auth Ticket Concurrency run 37344020775
  - material implementation head 6339b0b8648bfe6176ff223848e10e59cec37aa9 passed Phase 7 Production-Like Validation run 37344020696
  - checkpoint-only successor 029d9585b7fae329645ffa226d2d708fd1708878 kept Agent Governance 37344463351 and required CI/platform-gate 37344463502 green
  - checkpoint-only successor skipped runtime-tests job 111879563348 after prior CI material evidence run 37344020823
  - checkpoint-only successor skipped Edge validate job 111879521994 after prior material evidence run 37344020756
  - checkpoint-only successor skipped DB Outage validate job 111879521328 after prior material evidence run 37344020734
  - checkpoint-only successor skipped Game Auth concurrency-proof job 111879522311 after prior material evidence run 37344020775
  - checkpoint-only successor skipped Phase 7 validate job 111879509339 after prior material evidence run 37344020696
  - final second checkpoint head 03d6daa2c2778b00fefc00871cb5a32d7ccd6bf2 kept Agent Governance 37344711222 and CI/platform-gate 37344711472 green while runtime-tests and all four heavy domain proof jobs were skipped
  - Merge Queue CI run 37344947643 passed runtime-tests, php-coverage-report, required test and platform-gate on resulting integration SHA b1e48bcfeb9ed114f112d69ac98914bf0b196874
  - protected-main CI run 37345393737 passed runtime-tests, php-coverage-report, required test and platform-gate on b1e48bcfeb9ed114f112d69ac98914bf0b196874
  - BUILD_TEST_MATRIX requires later docs-only commits to run only path-selected checks while later runtime/build changes invalidate prior heavy evidence.
derived:
  - reuse must be attributable to an exact prior PR head and successful heavy job, not inferred only from latest path names
  - material tree identity provides a fail-closed proof that skipped heavy internals would test identical material bytes
unknown: []
conflicts: []
first_failure:
  marker: initial-task-pr-binding
  evidence: Agent Governance run 37344020746 failed only because the newly opened PR #1453 was not yet recorded in the active task; the checkpoint successor recorded pr 1453 and Agent Governance run 37344463351 passed
rejected_hypotheses:
  - trigger-level paths-ignore alone solves accumulated PR diff reruns
  - a docs-only latest commit alone is enough evidence to skip heavy validation
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261005-heavy-evidence-reuse.md
validation:
  - command: material-head CI run 37344020823
    result: PASS
    evidence: runtime-tests PASS; required test PASS; platform-gate PASS on 6339b0b8648bfe6176ff223848e10e59cec37aa9
  - command: material-head heavy domain workflows
    result: PASS
    evidence: Edge 37344020756; DB Outage 37344020734; Game Auth Concurrency 37344020775; Phase 7 37344020696
  - command: checkpoint-only successor reuse proof
    result: PASS
    evidence: successor 029d9585b7fae329645ffa226d2d708fd1708878; CI 37344463502 runtime-tests SKIPPED and platform-gate PASS; Edge 37344463686 validate SKIPPED; DB 37344463191 validate SKIPPED; Game Auth 37344463674 concurrency-proof SKIPPED; Phase 7 37344463389 validate SKIPPED; Agent Governance 37344463351 PASS
  - command: second checkpoint successor exact-head validation
    result: PASS
    evidence: final head 03d6daa2c2778b00fefc00871cb5a32d7ccd6bf2; Agent Governance 37344711222 PASS; CI 37344711472 test/platform-gate PASS with runtime-tests SKIPPED; Edge 37344711269 validate SKIPPED; DB 37344711431 validate SKIPPED; Game Auth 37344711308 concurrency-proof SKIPPED; Phase 7 37344711324 validate SKIPPED
  - command: Merge Queue exact integration validation
    result: PASS
    evidence: merge_group CI 37344947643 passed runtime-tests, php-coverage-report, test and platform-gate on integration SHA b1e48bcfeb9ed114f112d69ac98914bf0b196874
  - command: protected-main post-merge CI
    result: PASS
    evidence: push CI 37345393737 passed runtime-tests, php-coverage-report, test and platform-gate
blockers: []
next_action: merge the sequential lifecycle closeout PR, verify this source ref is absent, then close Issue #1012 as completed
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository CI task
source_branch_evidence: implementation PR #1453 merged as b1e48bcfeb9ed114f112d69ac98914bf0b196874 from exact head 03d6daa2c2778b00fefc00871cb5a32d7ccd6bf2; the same ref was recreated at protected main solely for this sequential lifecycle closeout and remains owned by the closeout PR until repository auto-delete removes it
```
