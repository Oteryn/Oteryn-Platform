---
task_id: OTERYN-20260823-org-terminal-branch-lifecycle
required_reads:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/architecture/adr/0037-terminal-source-branch-lifecycle.md
search_first:
  - terminal_branch_cleanup
  - terminal-branch-lifecycle
optional_reads: []
---

# OTERYN-20260823-org-terminal-branch-lifecycle

## Goal

Publish the existing Platform terminal branch lifecycle as a reusable organization capability and hand off repository-local adoption to the explicitly created META, Game and Atlas lifecycle issues.

## Acceptance criteria

- [x] Platform exposes a tested reusable lifecycle workflow.
- [x] Reusable workflow uses caller-local tokens and immutable Platform source pinning.
- [x] Organization caller contract documents read, close and reviewed-apply boundaries.
- [x] META #51, Game #65 and Atlas #93 own repository-local adoption from the exact merged Platform SHA.
- [x] Exact-head Platform CI, Agent Governance and Terminal Branch Lifecycle validation pass on the implementation candidate.
- [ ] Archive this Platform task record after PR #1232 reaches terminal state; parent Issue #1230 remains open until provider rollout is verified.

## Ownership

```yaml
owned_paths:
  - .github/workflows/terminal-branch-lifecycle-reusable.yml
  - .github/workflows/terminal-branch-lifecycle.yml
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - tools/agents/test_terminal_branch_reusable.py
  - docs/contracts/ORGANIZATION_TERMINAL_BRANCH_LIFECYCLE.md
  - docs/superpowers/specs/2026-08-23-organization-terminal-branch-lifecycle-design.md
  - docs/superpowers/plans/2026-08-23-organization-terminal-branch-lifecycle.md
  - docs/agents/tasks/active/OTERYN-20260823-org-terminal-branch-lifecycle.md
modules:
  - agent-governance
  - ci
dependencies:
  - docs/architecture/adr/0037-terminal-source-branch-lifecycle.md
blockers:
  - none
cross_repository_tasks:
  - Oteryn/Oteryn#51
  - Oteryn/Oteryn-Game#65
  - Oteryn/Oteryn-Atlas#93
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-22T22:55:00Z
head: 1691e3fde02dc17054a6c38940bb246aad050e84
branch: ci/org-terminal-branch-lifecycle
pr: 1232
status: ready
terminal_pr_policy: archive_pending
context_routes:
  - agent-governance
  - testing
owned_paths:
  - .github/workflows/terminal-branch-lifecycle-reusable.yml
  - .github/workflows/terminal-branch-lifecycle.yml
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - tools/agents/test_terminal_branch_reusable.py
  - docs/contracts/ORGANIZATION_TERMINAL_BRANCH_LIFECYCLE.md
  - docs/superpowers/specs/2026-08-23-organization-terminal-branch-lifecycle-design.md
  - docs/superpowers/plans/2026-08-23-organization-terminal-branch-lifecycle.md
  - docs/agents/tasks/active/OTERYN-20260823-org-terminal-branch-lifecycle.md
proven:
  - Oteryn-Platform already implements exact-head terminal closed-unmerged cleanup and ADR 0037.
  - RED 1: reusable workflow contract failed 3/3 because the reusable workflow did not exist.
  - GREEN 1: reusable workflow contract 3/3 PASS; existing cleanup 7/7 PASS; guarded 8/8 PASS; approval 10/10 PASS.
  - RED 2: lifecycle-CI integration contract failed because the existing workflow did not execute/track the reusable contract test.
  - GREEN 2: reusable contract 4/4 PASS; cleanup 7/7 PASS; guarded 8/8 PASS; approval 10/10 PASS; git diff --check PASS.
  - exact-head PR CI exposed the repository workflow lifecycle registry/budget guard; the reusable workflow is now explicitly registered and the reviewed workflow budget is 54.
  - exact-head CI run 32603550450 completed classify-changes, runtime-tests, test and platform-gate successfully.
  - exact-head Agent Governance run 32603550526 succeeded.
  - exact-head Terminal Branch Lifecycle run 32603550431 succeeded.
  - reusable workflow uses caller GITHUB_TOKEN and a credential-free pinned Platform tool checkout; destructive git validation remains rooted at caller GITHUB_WORKSPACE.
  - caller adoption issues exist as META #51, Game #65 and Atlas #93.
derived:
  - repository-local GITHUB_TOKEN execution avoids an organization-wide destructive credential.
unknown:
  - exact squash-merged Platform SHA callers will pin after PR #1232 reaches terminal state.
conflicts: []
first_failure:
  marker: reusable workflow missing
  evidence: local RED test on de6efc530a22938f027ae92717bb3153d2d21e2b
rejected_hypotheses:
  - rely only on GitHub delete_branch_on_merge; it does not cover closed-unmerged or orphan terminal paths.
changed_paths:
  - .github/workflows/terminal-branch-lifecycle-reusable.yml
  - .github/workflows/terminal-branch-lifecycle.yml
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - docs/agents/tasks/active/OTERYN-20260823-org-terminal-branch-lifecycle.md
  - docs/contracts/ORGANIZATION_TERMINAL_BRANCH_LIFECYCLE.md
  - docs/superpowers/specs/2026-08-23-organization-terminal-branch-lifecycle-design.md
  - docs/superpowers/plans/2026-08-23-organization-terminal-branch-lifecycle.md
  - tools/agents/test_terminal_branch_reusable.py
validation:
  - command: python3 tools/agents/test_terminal_branch_reusable.py
    result: PASS
    evidence: 4 tests, 0 failures on implementation candidate
  - command: python3 tools/agents/test_terminal_branch_cleanup.py
    result: PASS
    evidence: 7 tests, 0 failures on implementation candidate
  - command: python3 tools/agents/test_terminal_branch_guarded.py
    result: PASS
    evidence: 8 tests, 0 failures on implementation candidate
  - command: python3 tools/agents/test_terminal_branch_approval.py
    result: PASS
    evidence: 10 tests, 0 failures on implementation candidate
  - command: python3 tools/validation/test_workflow_inventory.py && python3 tools/validation/workflow_inventory.py
    result: PASS
    evidence: 18 tests, 54 registered workflows within reviewed budget 54
  - command: git diff --check origin/main...HEAD
    result: PASS
    evidence: no whitespace errors on implementation candidate
  - command: GitHub Actions CI run 32603550450
    result: PASS
    evidence: classify-changes, runtime-tests, test and platform-gate all SUCCESS
  - command: GitHub Actions Agent Governance run 32603550526
    result: PASS
    evidence: checkpoint-validation SUCCESS
  - command: GitHub Actions Terminal Branch Lifecycle run 32603550431
    result: PASS
    evidence: validate and live dry-run SUCCESS
blockers:
  - none
next_action: archive this Platform task record after PR #1232 reaches terminal state; continue repository adoption under META #51, Game #65 and Atlas #93
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: PR #1232 is not terminal yet; repository lifecycle will classify the source ref after terminalization
source_branch_evidence: pending
```

## Notes

Lifecycle authority for the organization rollout remains GitHub Issue #1230. Cross-repository writes for META, Game and Atlas are explicitly authorized by the owner for this rollout.
