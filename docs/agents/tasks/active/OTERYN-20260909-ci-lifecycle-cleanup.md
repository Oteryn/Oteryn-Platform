---
task_id: OTERYN-20260909-ci-lifecycle-cleanup
governing_issue: 1364
status: investigating
project_lane: oteryn-platform-core
execution_mode: github_connector
required_reads:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/AGENTS.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CI_WORKFLOW_LIFECYCLE.md
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/architecture/adr/0039-historical-work-canonicalization-and-managed-recovery.md
search_first:
  - Issue #1364
  - protected main branch protection and rulesets
  - current workflow inventory
  - open PR workflow ownership
optional_reads: []
source_branch_disposition: auto_delete_after_merge
---

# OTERYN-20260909 CI lifecycle cleanup

## Goal

Perform a fresh 55/55 current-main CI lifecycle audit and integrate only evidence-proven workflow retirement, consolidation, or trigger repair while preserving every required merge, governance, security, runtime, deployment, rollback, and steady-state branch-hygiene guarantee.

## Acceptance criteria

- [ ] Every current workflow receives one evidence-backed verdict: `KEEP`, `REPAIR`, `CONSOLIDATE`, `RETIRE`, or `UNKNOWN`.
- [ ] Every retirement/consolidation proves current caller, required-check, trigger, and replacement state before mutation.
- [ ] Dead historical active-task path filters are removed only where they no longer represent a valid CI trigger.
- [ ] Current active PR workflow ownership is refreshed immediately before each mutation and overlapping files remain untouched.
- [ ] Lifecycle registry, retired inventory, and workflow budget exactly match the resulting workflow set.
- [ ] `platform-gate`, merge-group support, fail-closed classification, Agent Governance live validation, ADR 0039 steady-state branch hygiene, deployment/rollback safety, and immutable action pinning remain intact.
- [ ] Focused workflow inventory/routing/trigger-economy tests and exact-head required GitHub checks pass.
- [ ] Whole-diff review has zero unresolved material findings.
- [ ] Integration follows normal protected PR authority and resulting main is re-inventoried before terminal closeout.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260909-ci-lifecycle-cleanup.md
  - docs/agents/tasks/archive/OTERYN-20260909-ci-lifecycle-cleanup.md
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - docs/agents/reports/OTERYN-20260909-ci-lifecycle-audit.md
  - .github/workflows/native-auth-canary-cache-build.yml
  - .github/workflows/native-auth-ephemeral-cutover-rehearsal.yml
modules:
  - ci-workflow-lifecycle
  - workflow-trigger-economy
dependencies:
  - preserve live ownership of workflow files held by other open PRs
blockers: []
cross_repository_tasks: []
```

Workflow files outside the explicit owned set remain discovery-only until their current ownership and a concrete `REPAIR`/`RETIRE` verdict are proven. Any newly claimed workflow path must be added here before mutation.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-09T00:04:17+02:00
head: 102d29241662d45bb810bca65f03386fcb598dad
branch: audit/issue-1364-ci-lifecycle-cleanup
pr: none
status: investigating
context_routes:
  - testing
  - ci-repair
  - agent-governance
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260909-ci-lifecycle-cleanup.md
  - docs/agents/tasks/archive/OTERYN-20260909-ci-lifecycle-cleanup.md
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - docs/agents/reports/OTERYN-20260909-ci-lifecycle-audit.md
  - .github/workflows/native-auth-canary-cache-build.yml
  - .github/workflows/native-auth-ephemeral-cutover-rehearsal.yml
proven:
  - admission protected main is 102d29241662d45bb810bca65f03386fcb598dad
  - classic branch protection requires only platform-gate
  - repository rulesets are empty
  - current lifecycle registry contains 55 workflows with budget 55
  - prior P1.4 audit merged as 557c08e72497ecd6a1b07fe8f282a1754763a4ff and classified the then-current 55 workflows
  - exactly five workflow files changed between that audit merge and admission main: agent-governance.yml, build-synology-staging-images.yml, ci.yml, deploy-synology-staging.yml, synology-rollback-contract.yml
  - current active task directory contains only public-domain-repair and portal-player-first-direction task records besides .gitkeep
  - native-auth production verification is archived/completed
  - native-auth ephemeral rehearsal still binds historical Platform/Gateway/Canary/OTClient revisions and a historical active-task path
  - native-auth Canary cache build is manual/self-file triggered and no current repository consumer of its named artifact has been found
  - Historical Branch Audit is the accepted ADR 0039 steady-state enforcement surface and is not a retirement candidate without replacement proof
  - Agent Governance contains unique live Issue/task/policy/Control-Room validation and is not a retirement candidate merely because some deterministic checks overlap core CI
derived:
  - the prior native-auth ephemeral KEEP rationale must be re-evaluated because its cited active production-verification dependency is now terminal
  - task-specific path references to files absent from the current active-task directory are trigger-debt candidates, not automatic deletion authority
unknown:
  - complete current caller/replacement proof for every retirement candidate
  - final 55/55 verdict distribution
  - whether any additional workflow repair can be safely claimed without colliding with current open PR ownership
conflicts: []
first_failure:
  marker: none
  evidence: audit admission only; no validation failure observed
rejected_hypotheses:
  - workflow age, name, or similarity alone authorizes removal
  - every skipped job is CI waste or a defect
  - Historical Branch Audit may be removed because the historical reconciliation registry is already applied
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260909-ci-lifecycle-cleanup.md
validation:
  - command: live branch protection / ruleset readback
    result: PASS
    evidence: main protected; required context platform-gate; rulesets empty
  - command: current workflow inventory readback
    result: PASS
    evidence: 55 registered workflows and lifecycle budget 55
  - command: current active task directory readback
    result: PASS
    evidence: exactly two active task Markdown files before this task was claimed
blockers: []
next_action: complete the fresh 55/55 workflow verdict ledger and caller/replacement proof, then mutate only paths with a proven safe repair or retirement and no live ownership conflict
```
