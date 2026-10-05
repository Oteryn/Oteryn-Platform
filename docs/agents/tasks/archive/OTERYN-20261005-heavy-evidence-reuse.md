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

## Terminal outcome

Issue #1012 is implemented and protected-integrated.

PR #1453 merged through Merge Queue as `b1e48bcfeb9ed114f112d69ac98914bf0b196874`. The source branch `ci/issue-1012-heavy-evidence-reuse` is absent.

The implementation adds fail-closed reuse for:

- core CI `runtime-tests`;
- Edge Security Emulation;
- Platform DB Outage Validation;
- Game Auth Ticket Concurrency;
- Phase 7 Production-Like Validation.

Reuse is allowed only on `pull_request` when the latest successor chain is docs/agent-governance-only, a prior successful heavy job exists for the exact same PR ancestor head, and the material repository tree SHA-256 digest is identical. Missing/ambiguous evidence runs heavy validation.

Merge Queue and protected-main generations do not reuse PR evidence.

## Acceptance criteria

- [x] Distinguish latest commit scope from accumulated PR delivery scope.
- [x] Fail closed to heavy validation for material/runtime/workflow/classifier changes.
- [x] Reuse prior successful heavy evidence only for same-PR exact ancestor heads with byte-identical material tree.
- [x] Keep lightweight exact-final-head governance and aggregate required checks.
- [x] Cover material -> heavy, checkpoint successor -> reuse, material-after-checkpoint -> heavy, ambiguous state -> heavy.
- [x] Record reused material head/run IDs in task/PR evidence.
- [x] Preserve security/auth/database/production-like and integration validation semantics.

## Live evidence

### Material generation

Exact material head: `6339b0b8648bfe6176ff223848e10e59cec37aa9`.

- CI `37344020823`: `runtime-tests` PASS, required `test` PASS, `platform-gate` PASS.
- Edge Security `37344020756`: `validate` PASS.
- DB Outage `37344020734`: `validate` PASS.
- Game Auth Concurrency `37344020775`: `concurrency-proof` PASS.
- Phase 7 `37344020696`: `validate` PASS.

### First checkpoint-only successor

Exact head: `029d9585b7fae329645ffa226d2d708fd1708878`.

- CI `37344463502`: `runtime-tests` job `111879563348` SKIPPED; required `test` and `platform-gate` PASS.
- Edge `37344463686`: `validate` job `111879521994` SKIPPED.
- DB Outage `37344463191`: `validate` job `111879521328` SKIPPED.
- Game Auth `37344463674`: `concurrency-proof` job `111879522311` SKIPPED.
- Phase 7 `37344463389`: `validate` job `111879509339` SKIPPED.
- Agent Governance `37344463351`: PASS.

The reusable prior material run IDs were `37344020823`, `37344020756`, `37344020734`, `37344020775`, and `37344020696`.

### Second consecutive checkpoint-only successor

Exact final PR head: `03d6daa2c2778b00fefc00871cb5a32d7ccd6bf2`.

- CI `37344711472`: `runtime-tests` SKIPPED; required `test` and `platform-gate` PASS.
- Edge `37344711269`: `validate` SKIPPED.
- DB Outage `37344711431`: `validate` SKIPPED.
- Game Auth `37344711308`: `concurrency-proof` SKIPPED.
- Phase 7 `37344711324`: `validate` SKIPPED.
- Agent Governance `37344711222`: PASS.
- CodeQL `37344711348`: all analyses PASS.

This proves reuse can walk across multiple consecutive docs/governance successors to the last exact material evidence head.

### Merge Queue fail-closed integration generation

Synthetic candidate: `b1e48bcfeb9ed114f112d69ac98914bf0b196874`.

Merge-group CI `37344947643` executed rather than reused:

- `runtime-tests` PASS;
- `php-coverage-report` PASS;
- required `test` PASS;
- `platform-gate` PASS.

### Protected-main fail-closed generation

Protected `main@b1e48bcfeb9ed114f112d69ac98914bf0b196874`.

Push CI `37345393737` executed rather than reused:

- `runtime-tests` PASS;
- `php-coverage-report` PASS;
- required `test` PASS;
- `platform-gate` PASS.

Post-merge Agent Governance `37345393917` failed only because the now-merged task was still in `active/`; this archive closeout is the lifecycle repair.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-05T17:10:00Z
head: b1e48bcfeb9ed114f112d69ac98914bf0b196874
branch: docs/issue-1012-heavy-evidence-reuse-closeout
pr: 1454
status: completed
terminal_pr_policy: archive_pending
context_routes:
  - ci
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20261005-heavy-evidence-reuse.md
proven:
  - PR #1453 merged through Merge Queue as b1e48bcfeb9ed114f112d69ac98914bf0b196874
  - material head 6339b0b8648bfe6176ff223848e10e59cec37aa9 ran all heavy internals successfully
  - checkpoint successor 029d9585b7fae329645ffa226d2d708fd1708878 reused prior heavy evidence and skipped redundant heavy internals
  - second checkpoint successor 03d6daa2c2778b00fefc00871cb5a32d7ccd6bf2 also reused across the consecutive checkpoint chain
  - merge-group CI 37344947643 ran runtime-tests and coverage successfully without PR reuse
  - protected-main CI 37345393737 ran runtime-tests and coverage successfully without PR reuse
  - source branch ci/issue-1012-heavy-evidence-reuse is absent
derived:
  - heavy evidence reuse reduces redundant PR work without weakening integration or protected-main validation
unknown: []
conflicts: []
first_failure:
  marker: initial-task-pr-binding
  evidence: Agent Governance 37344020746 failed because active task PR identity was not yet recorded; successor checkpoint recorded PR #1453 and governance passed
rejected_hypotheses:
  - trigger-level paths-ignore alone solves accumulated PR diff reruns
  - latest docs-only path classification alone is sufficient evidence for reuse
  - PR reuse may be carried into Merge Queue or protected-main validation
changed_paths:
  - .github/workflows/ci.yml
  - .github/workflows/edge-security-emulation.yml
  - .github/workflows/platform-db-outage-validation.yml
  - .github/workflows/game-auth-ticket-concurrency.yml
  - .github/workflows/phase7-production-like-validation.yml
  - scripts/ci/heavy_evidence_reuse.py
  - scripts/ci/required_test_gate.py
  - tests/ci/test_heavy_evidence_reuse.py
  - tests/ci/test_required_test_gate.py
  - tests/ci/test_workflow_trigger_economy.py
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/tasks/archive/OTERYN-20261005-heavy-evidence-reuse.md
validation:
  - command: material PR generation
    result: PASS
    evidence: CI 37344020823; Edge 37344020756; DB 37344020734; Game Auth 37344020775; Phase 7 37344020696
  - command: checkpoint-only successor reuse
    result: PASS
    evidence: 029d9585b7fae329645ffa226d2d708fd1708878 with heavy jobs skipped and required gates green
  - command: consecutive checkpoint-only successor reuse
    result: PASS
    evidence: 03d6daa2c2778b00fefc00871cb5a32d7ccd6bf2 with heavy jobs skipped and required gates green
  - command: Merge Queue exact integration validation
    result: PASS
    evidence: CI 37344947643 on b1e48bcfeb9ed114f112d69ac98914bf0b196874; runtime-tests, coverage, test and platform-gate PASS
  - command: protected-main validation
    result: PASS
    evidence: CI 37345393737 on b1e48bcfeb9ed114f112d69ac98914bf0b196874; runtime-tests, coverage, test and platform-gate PASS
blockers: []
next_action: validate and merge PR #1454, verify closeout branch absence, close Issue #1012 as completed
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: lifecycle-only closeout branch has no retention or recovery purpose
source_branch_evidence: PR #1453 merged as b1e48bcfeb9ed114f112d69ac98914bf0b196874 from exact final head 03d6daa2c2778b00fefc00871cb5a32d7ccd6bf2 and source branch ci/issue-1012-heavy-evidence-reuse is absent
```
