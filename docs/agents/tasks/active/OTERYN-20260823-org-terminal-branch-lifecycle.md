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

- [ ] Platform exposes a tested reusable lifecycle workflow.
- [ ] Reusable workflow uses caller-local tokens and immutable Platform source pinning.
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
updated_at: 2026-08-22T22:40:00Z
head: 788f58c031bf575396231a95b6a9d28afbadb67c
branch: ci/org-terminal-branch-lifecycle
pr: none
status: implementing
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
  - Oteryn-Atlas delete_branch_on_merge is enabled but closed-unmerged/orphan refs remain outside that GitHub setting.
derived:
  - repository-local GITHUB_TOKEN execution avoids an organization-wide destructive credential.
unknown:
  - exact merged Platform SHA callers will pin after Platform PR closeout.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - rely only on GitHub delete_branch_on_merge; it does not cover closed-unmerged or orphan terminal paths.
changed_paths:
  - docs/superpowers/specs/2026-08-23-organization-terminal-branch-lifecycle-design.md
  - docs/superpowers/plans/2026-08-23-organization-terminal-branch-lifecycle.md
  - tools/agents/test_terminal_branch_reusable.py
validation:
  - command: python3 tools/agents/test_terminal_branch_reusable.py
    result: NOT_RUN
    evidence: RED test commit not yet materialized
blockers:
  - none
next_action: materialize the RED reusable-workflow contract test and verify it fails because the reusable workflow is absent
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```

## Notes

Lifecycle authority: GitHub Issue #1230. Cross-repository writes for META, Game and Atlas are explicitly authorized by the owner for this rollout.
