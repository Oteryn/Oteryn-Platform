---
task_id: OTERYN-20261005-heavy-evidence-reuse
governing_issue: 1012
required_reads:
  - docs/agents/BUILD_TEST_MATRIX.md
search_first:
  - checkpoint-only heavy workflow reuse
optional_reads: []
---

# OTERYN-20261005 heavy evidence reuse

## Goal

Implement Issue #1012 without weakening exact-head governance or heavy validation.

A later checkpoint/docs/governance-only commit may reuse an earlier successful heavy result only when all of the following are proven for the same PR:

1. the current PR history from the frozen base to final head is unambiguous and linear for reuse purposes;
2. the helper identifies the most recent commit that materially affects the target heavy gate;
3. the target gate's material tree digest is byte-identical between that material head and the current final head;
4. the same workflow has a prior successful pull-request run on that exact material head;
5. the specific heavy evidence job in that run completed successfully;
6. the run is attributable to the same pull request.

Any missing, stale or ambiguous evidence fails closed to executing the heavy job again.

## Acceptance criteria

- [ ] Material change -> heavy run.
- [ ] Checkpoint/docs-only successor -> reuse only with exact prior successful heavy evidence and unchanged gate-specific material digest.
- [ ] Material change after checkpoint -> heavy run again.
- [ ] Ambiguous ancestry/API/evidence -> heavy run.
- [ ] Central runtime-tests and current heavy domain workflows use the same fail-closed helper.
- [ ] Lightweight exact-final-head CI/governance still runs.
- [ ] Required `test` gate accepts runtime evidence reuse only with explicit material head + prior run ID.
- [ ] Positive/negative deterministic tests pass.
- [ ] Live PR proves implementation head executes heavy validation and a later checkpoint-only successor does not rerun unchanged heavy internals.
- [ ] Merge Queue and protected-main readback pass.

## Ownership

```yaml
owned_paths:
  - scripts/ci/heavy_evidence_reuse.py
  - tests/ci/test_heavy_evidence_reuse.py
  - scripts/ci/required_test_gate.py
  - tests/ci/test_required_test_gate.py
  - tests/ci/test_workflow_trigger_economy.py
  - .github/workflows/ci.yml
  - .github/workflows/phase7-production-like-validation.yml
  - .github/workflows/edge-security-emulation.yml
  - .github/workflows/platform-db-outage-validation.yml
  - .github/workflows/game-auth-ticket-concurrency.yml
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/tasks/active/OTERYN-20261005-heavy-evidence-reuse.md
modules:
  - CI routing
  - heavy validation evidence reuse
dependencies:
  - GitHub Actions read-only workflow evidence API
blockers: []
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-05T15:32:00Z
head: 372659cea4507ec21645fa20a791d20bb35ef504
branch: test/issue-1012-heavy-evidence-reuse
pr: 1452
status: validating
terminal_pr_policy: active
context_routes:
  - ci
owned_paths:
  - scripts/ci/heavy_evidence_reuse.py
  - tests/ci/test_heavy_evidence_reuse.py
  - scripts/ci/required_test_gate.py
  - tests/ci/test_required_test_gate.py
  - tests/ci/test_workflow_trigger_economy.py
  - .github/workflows/ci.yml
  - .github/workflows/phase7-production-like-validation.yml
  - .github/workflows/edge-security-emulation.yml
  - .github/workflows/platform-db-outage-validation.yml
  - .github/workflows/game-auth-ticket-concurrency.yml
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/tasks/active/OTERYN-20261005-heavy-evidence-reuse.md
proven:
  - Issue #1012 is open with no current branch or PR ownership
  - current protected main is b402fdf769a78f30ee6c54574900a37b2530b4cf
  - current central CI and four domain workflows classify the accumulated PR diff, which can rerun unchanged heavy internals after a checkpoint-only successor
  - php-coverage-report is already disabled on ordinary pull_request events and is outside this optimization
derived:
  - reuse must be job-evidence based, not merely path-based, because the required final head still needs auditable proof
unknown:
  - exact live GitHub run evidence shape on the first checkpoint-only successor
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - path-ignore alone solves checkpoint-successor reruns
  - generic workflow success is sufficient without proving the heavy evidence job succeeded
  - final-head governance may be skipped when heavy evidence is reused
changed_paths:
  - scripts/ci/heavy_evidence_reuse.py
  - tests/ci/test_heavy_evidence_reuse.py
  - scripts/ci/required_test_gate.py
  - tests/ci/test_required_test_gate.py
  - tests/ci/test_workflow_trigger_economy.py
  - .github/workflows/ci.yml
  - .github/workflows/phase7-production-like-validation.yml
  - .github/workflows/edge-security-emulation.yml
  - .github/workflows/platform-db-outage-validation.yml
  - .github/workflows/game-auth-ticket-concurrency.yml
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/tasks/active/OTERYN-20261005-heavy-evidence-reuse.md
validation:
  - command: live ownership readback
    result: PASS
    evidence: Issue #1012 has no active PR or branch
blockers: []
next_action: validate PR #1452; because no prior same-PR heavy success exists yet, all materially affected heavy internals must run fail-closed on this synchronized generation
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository CI optimization branch with no retention purpose after protected integration
source_branch_evidence: pending
```
