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

Promote the existing Platform terminal branch lifecycle into a reusable organization capability and roll it out to META, Game and Atlas with repository-local deletion authority.

## Acceptance criteria

- [x] Platform exposes a tested reusable lifecycle workflow.
- [x] Reusable workflow uses caller-local tokens and immutable Platform source pinning.
- [ ] META, Game and Atlas adopt thin pinned wrappers plus local policies/ADRs.
- [ ] Exact-head required CI passes before every merge.
- [ ] Provider rollout is verified on merged main without heuristic historical deletion.

## Ownership

```yaml
owned_paths:
  - .github/workflows/terminal-branch-lifecycle-reusable.yml
  - .github/workflows/terminal-branch-lifecycle.yml
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
  - Oteryn/Oteryn adoption
  - Oteryn/Oteryn-Game adoption
  - Oteryn/Oteryn-Atlas adoption
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-22T22:48:00Z
head: 5fe0033759dbde940ce8d60ebe731c0d3aaaa68d
branch: ci/org-terminal-branch-lifecycle
pr: pending
status: validating
context_routes:
  - agent-governance
  - testing
owned_paths:
  - .github/workflows/terminal-branch-lifecycle-reusable.yml
  - .github/workflows/terminal-branch-lifecycle.yml
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
  - reusable workflow uses caller GITHUB_TOKEN and a credential-free pinned Platform tool checkout; destructive git validation remains rooted at caller GITHUB_WORKSPACE.
derived:
  - repository-local GITHUB_TOKEN execution avoids an organization-wide destructive credential.
unknown:
  - exact merged Platform SHA callers will pin after Platform PR closeout.
conflicts: []
first_failure:
  marker: reusable workflow missing
  evidence: local RED test on de6efc530a22938f027ae92717bb3153d2d21e2b
rejected_hypotheses:
  - rely only on GitHub delete_branch_on_merge; it does not cover closed-unmerged or orphan terminal paths.
changed_paths:
  - .github/workflows/terminal-branch-lifecycle-reusable.yml
  - .github/workflows/terminal-branch-lifecycle.yml
  - docs/agents/tasks/active/OTERYN-20260823-org-terminal-branch-lifecycle.md
  - docs/contracts/ORGANIZATION_TERMINAL_BRANCH_LIFECYCLE.md
  - docs/superpowers/specs/2026-08-23-organization-terminal-branch-lifecycle-design.md
  - docs/superpowers/plans/2026-08-23-organization-terminal-branch-lifecycle.md
  - tools/agents/test_terminal_branch_reusable.py
validation:
  - command: python3 tools/agents/test_terminal_branch_reusable.py
    result: PASS
    evidence: 4 tests, 0 failures on head 5fe0033759dbde940ce8d60ebe731c0d3aaaa68d
  - command: python3 tools/agents/test_terminal_branch_cleanup.py
    result: PASS
    evidence: 7 tests, 0 failures on head 5fe0033759dbde940ce8d60ebe731c0d3aaaa68d
  - command: python3 tools/agents/test_terminal_branch_guarded.py
    result: PASS
    evidence: 8 tests, 0 failures on head 5fe0033759dbde940ce8d60ebe731c0d3aaaa68d
  - command: python3 tools/agents/test_terminal_branch_approval.py
    result: PASS
    evidence: 10 tests, 0 failures on head 5fe0033759dbde940ce8d60ebe731c0d3aaaa68d
  - command: git diff --check origin/main...HEAD
    result: PASS
    evidence: no output on head 5fe0033759dbde940ce8d60ebe731c0d3aaaa68d
blockers:
  - none
next_action: open the Platform PR and qualify the exact current head through required CI before merge
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active through provider rollout
source_branch_evidence: pending
```

## Notes

Lifecycle authority: GitHub Issue #1230. Cross-repository writes for META, Game and Atlas are explicitly authorized by the owner for this rollout.
