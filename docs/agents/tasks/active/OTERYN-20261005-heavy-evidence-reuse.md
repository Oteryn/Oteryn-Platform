---
task_id: OTERYN-20261005-heavy-evidence-reuse
governing_issue: 1012
required_reads:
  - docs/agents/BUILD_TEST_MATRIX.md
search_first:
  - checkpoint-only heavy workflow reuse
optional_reads: []
---

# OTERYN-20261005 heavy workflow evidence reuse

## Goal

Implement Issue #1012 without weakening required protection: a later checkpoint/docs-only commit may reuse a prior successful heavy-workflow result only when the same PR has directly attributable successful evidence for an ancestor head and the non-governance/material repository tree is provably byte-identical.

## Acceptance criteria

- [ ] Distinguish the latest commit scope from the accumulated PR delivery scope.
- [ ] Fail closed to heavy validation for material/runtime/workflow/classifier changes.
- [ ] Reuse prior successful heavy evidence only for same-PR exact ancestor heads with byte-identical material tree.
- [ ] Keep lightweight exact-final-head governance and aggregate required checks.
- [ ] Cover material -> heavy, checkpoint successor -> reuse, material-after-checkpoint -> heavy, ambiguous state -> heavy.
- [ ] Record reused material head/run IDs in workflow summaries and terminal evidence.
- [ ] Preserve security/auth/database/production-like required semantics.

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
updated_at: 2026-10-05T16:45:00Z
head: b402fdf769a78f30ee6c54574900a37b2530b4cf
branch: ci/issue-1012-heavy-evidence-reuse
pr: none
status: implementing
terminal_pr_policy: active
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
  - BUILD_TEST_MATRIX requires later docs-only commits to run only path-selected checks while later runtime/build changes invalidate prior heavy evidence.
derived:
  - reuse must be attributable to an exact prior PR head and successful heavy job, not inferred only from latest path names
  - material tree identity provides a fail-closed proof that skipped heavy internals would test identical material bytes
unknown:
  - exact implementation-head CI result
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - trigger-level paths-ignore alone solves accumulated PR diff reruns
  - a docs-only latest commit alone is enough evidence to skip heavy validation
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261005-heavy-evidence-reuse.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: implementation pending
blockers: []
next_action: implement reusable evidence resolver, wire four heavy workflows, add fail-closed tests, then open and validate PR
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository CI task
source_branch_evidence: pending
```
