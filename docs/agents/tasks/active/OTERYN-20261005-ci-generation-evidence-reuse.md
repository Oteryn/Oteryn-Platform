---
task_id: OTERYN-20261005-ci-generation-evidence-reuse
governing_issue: 1012
required_reads:
  - docs/agents/BUILD_TEST_MATRIX.md
  - tests/ci/test_workflow_trigger_economy.py
  - scripts/ci/classify_changes.py
search_first:
  - checkpoint-only heavy workflow rerun
  - pull_request synchronize before after
optional_reads: []
---

# OTERYN-20261005 CI generation evidence reuse

## Goal

Implement Issue #1012 without weakening final-head governance or fail-closed runtime validation.

For a pull-request `synchronize` generation, a heavy lane may reuse earlier evidence only when:

1. the exact `before -> after` generation delta is non-material for that gate;
2. the gate-relevant material tree has the same deterministic digest;
3. an earlier successful run of the same workflow, same PR, same material digest contains a successful heavy evidence job;
4. all event identity and API evidence is unambiguous.

Any missing, inconsistent or unavailable evidence falls back to running the heavy lane.

## Acceptance criteria

- [ ] Distinguish accumulated PR delivery scope from exact latest-generation scope.
- [ ] Preserve fail-closed reruns for runtime/build/workflow/classifier changes.
- [ ] Reuse only prior successful same-PR heavy evidence with byte-identical gate material tree.
- [ ] Keep final-head lightweight governance and required aggregate gates running.
- [ ] Cover material -> heavy, checkpoint successor -> reuse, material-after-checkpoint -> heavy, ambiguous -> heavy.
- [ ] Record reused source head/run in workflow summary and task evidence.
- [ ] Exact-head CI/governance/heavy checks pass and protected Merge Queue integration is verified.

## Ownership

```yaml
owned_paths:
  - scripts/ci/pr_generation_reuse.py
  - tests/ci/test_pr_generation_reuse.py
  - tests/ci/fixtures/pr-generation-reuse-cases.json
  - .github/workflows/ci.yml
  - .github/workflows/edge-security-emulation.yml
  - .github/workflows/platform-db-outage-validation.yml
  - .github/workflows/game-auth-ticket-concurrency.yml
  - .github/workflows/phase7-production-like-validation.yml
  - tests/ci/test_workflow_trigger_economy.py
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/tasks/active/OTERYN-20261005-ci-generation-evidence-reuse.md
modules:
  - CI routing economy
  - pull-request generation evidence reuse
dependencies:
  - GitHub pull_request synchronize before/after identity
  - GitHub Actions read-only workflow-run/job API
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-05T15:50:00+02:00
head: b402fdf769a78f30ee6c54574900a37b2530b4cf
branch: ci/issue-1012-generation-evidence-reuse
pr: none
status: implementing
terminal_pr_policy: archive_pending
context_routes:
  - ci
owned_paths:
  - scripts/ci/pr_generation_reuse.py
  - tests/ci/test_pr_generation_reuse.py
  - tests/ci/fixtures/pr-generation-reuse-cases.json
  - .github/workflows/ci.yml
  - .github/workflows/edge-security-emulation.yml
  - .github/workflows/platform-db-outage-validation.yml
  - .github/workflows/game-auth-ticket-concurrency.yml
  - .github/workflows/phase7-production-like-validation.yml
  - tests/ci/test_workflow_trigger_economy.py
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/tasks/active/OTERYN-20261005-ci-generation-evidence-reuse.md
proven:
  - Issue #1012 is open with no existing branch or pull request ownership.
  - current protected Platform main is b402fdf769a78f30ee6c54574900a37b2530b4cf.
  - current heavy workflows classify accumulated base-to-head PR scope and can therefore rerun after checkpoint-only successors.
  - pull_request synchronize payloads expose exact previous and new head SHAs as before and after.
  - GitHub workflow-run API supports exact head_sha filtering and read-only workflow run/job evidence.
derived:
  - latest-generation classification alone is insufficient; prior successful same-PR evidence must also be proven before reuse.
  - matching deterministic material-tree digests provide a fail-closed proof that relevant repository bytes did not change.
unknown:
  - exact final implementation head and live reuse result
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - classify only HEAD^ and assume it is the previous PR generation
  - skip heavy work merely because the newest commit message or paths look like documentation
  - treat a successful workflow with a skipped heavy job as sufficient source evidence
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261005-ci-generation-evidence-reuse.md
validation:
  - command: live Issue/PR/branch ownership readback
    result: PASS
    evidence: Issue #1012 open; no matching PR or branch before task claim
blockers: []
next_action: implement fail-closed generation reuse resolver, tests and workflow integration
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: dedicated Issue #1012 implementation branch has no retention purpose after protected integration
source_branch_evidence: pending
```
