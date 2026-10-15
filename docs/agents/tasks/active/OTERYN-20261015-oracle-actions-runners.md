---
task_id: OTERYN-20261015-oracle-actions-runners
governing_issue: 1639
required_reads: []
search_first: []
optional_reads: []
---

# Oracle Actions runners

Governing Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1639.
The owner explicitly authorized moving both organization runners and deployment
workflows to Oracle, preserving direct oteryn.com and all migrated authority.
One writer: this Codex task, branch codex/oracle-actions-runners-20261015.
Game deployment coordination: Oteryn/Oteryn-Game#2217; no protocol changes.

## Acceptance criteria

- Runners stay in their repository-restricted groups with existing component labels.
- No runner receives a Docker socket, host secrets or general host shell.
- A root-owned component-bound forced command accepts only status or deployment
  of exact current protected main after its GitHub Actions aggregate gate passes.
- Preserve private environment, migrated storage, custody, content and the durable
  Game incarnation controller. Back up before migrations; fail closed on failure.
- Legacy NAS/Tunnel workflows cannot route jobs or overwrite Oracle configuration.
- Real protected workflow executions prove runner identity, ARM64 and readiness.
- Game daily physical and logical backups are verified and persist across reboot.

## Ownership

Owned: deploy/oracle/**, .github/workflows/oracle-operations.yml and this packet.
Dependencies: existing installed durable controller and coherent migration.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-15T22:00:00Z
head: 0628d229a53bf54d2d978a98b81204753ac1bf79
branch: codex/oracle-actions-runners-20261015
pr: none
status: implementing
context_routes:
  - deployment
owned_paths:
  - deploy/oracle/**
  - .github/workflows/oracle-operations.yml
  - docs/agents/tasks/active/OTERYN-20261015-oracle-actions-runners.md
proven:
  - New component runners registered offline in existing repository-restricted groups
  - Native Account repair merged via real Merge Queue and browser roster passed
derived: []
unknown:
  - Protected workflow deployment acceptance
  - Final native gameplay entry
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths: []
validation:
  - command: not-run
    result: NOT_RUN
    evidence: candidate under authoring
blockers: []
next_action: qualify bounded deployment adapter and publish coherent candidate
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository delivery
source_branch_evidence: verify after protected integration
```
