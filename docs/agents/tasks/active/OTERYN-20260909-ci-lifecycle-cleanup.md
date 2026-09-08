---
task_id: OTERYN-20260909-ci-lifecycle-cleanup
governing_issue: 1364
status: implementing
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

Perform a fresh 55/55 current-main CI lifecycle audit and integrate only evidence-proven workflow retirement or trigger repair while preserving every required merge, governance, security, runtime, deployment, rollback, and steady-state branch-hygiene guarantee.

## Acceptance criteria

- [x] Every current workflow receives one evidence-backed verdict: `KEEP`, `REPAIR`, `CONSOLIDATE`, `RETIRE`, or `UNKNOWN`.
- [x] Every retirement/consolidation proves current caller, required-check, trigger, and replacement state before mutation.
- [ ] Dead historical active-task path filters are removed only where they no longer represent a valid CI trigger.
- [x] Current active PR workflow ownership is refreshed immediately before mutation and overlapping files remain untouched.
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
  - tests/ci/test_workflow_trigger_economy.py
  - .github/workflows/branch-lifecycle.yml
  - .github/workflows/cloudflare-oteryn-edge-audit.yml
  - .github/workflows/cloudflare-oteryn-endpoints.yml
  - .github/workflows/cloudflare-oteryn-hsts-stage1.yml
  - .github/workflows/cloudflare-oteryn-public-edge-repair.yml
  - .github/workflows/cloudflare-zone-edge-audit.yml
  - .github/workflows/game-catalog-contract.yml
  - .github/workflows/native-auth-canary-cache-build.yml
  - .github/workflows/native-auth-ephemeral-cutover-rehearsal.yml
  - .github/workflows/native-protocol-contract-audits.yml
  - .github/workflows/synology-production-target-preflight.yml
  - .github/workflows/synology-rollback-contract.yml
modules:
  - ci-workflow-lifecycle
  - workflow-trigger-economy
dependencies:
  - preserve live ownership of workflow files held by other open PRs
blockers: []
cross_repository_tasks: []
```

All workflow files outside this explicit set remain discovery-only. In particular, current open PR ownership keeps `build-synology-staging-images.yml`, `codeql.yml`, and `repair-synology-autostart.yml` out of this writer's mutation set.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-09T00:18:00+02:00
head: 102d29241662d45bb810bca65f03386fcb598dad
branch: audit/issue-1364-ci-lifecycle-cleanup
pr: none
status: implementing
context_routes:
  - testing
  - ci-repair
  - agent-governance
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260909-ci-lifecycle-cleanup.md
  - docs/agents/tasks/archive/OTERYN-20260909-ci-lifecycle-cleanup.md
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - docs/agents/reports/OTERYN-20260909-ci-lifecycle-audit.md
  - tests/ci/test_workflow_trigger_economy.py
  - .github/workflows/branch-lifecycle.yml
  - .github/workflows/cloudflare-oteryn-edge-audit.yml
  - .github/workflows/cloudflare-oteryn-endpoints.yml
  - .github/workflows/cloudflare-oteryn-hsts-stage1.yml
  - .github/workflows/cloudflare-oteryn-public-edge-repair.yml
  - .github/workflows/cloudflare-zone-edge-audit.yml
  - .github/workflows/game-catalog-contract.yml
  - .github/workflows/native-auth-canary-cache-build.yml
  - .github/workflows/native-auth-ephemeral-cutover-rehearsal.yml
  - .github/workflows/native-protocol-contract-audits.yml
  - .github/workflows/synology-production-target-preflight.yml
  - .github/workflows/synology-rollback-contract.yml
proven:
  - admission protected main is 102d29241662d45bb810bca65f03386fcb598dad
  - classic branch protection requires only platform-gate and repository rulesets are empty
  - current main platform-gate passed in run 34282843333 job 102251547776; classify-changes and aggregate test passed while runtime-tests and php-coverage-report correctly skipped for the docs-only main change
  - current lifecycle registry contains 55 workflows with budget 55
  - prior P1.4 audit merged as 557c08e72497ecd6a1b07fe8f282a1754763a4ff and classified the then-current 55 workflows
  - exactly five workflow files changed between that audit merge and admission main: agent-governance.yml, build-synology-staging-images.yml, ci.yml, deploy-synology-staging.yml, synology-rollback-contract.yml
  - all other workflow definitions are byte-lineage unchanged from the P1.4 baseline; lifecycle context was re-evaluated rather than blindly inherited
  - current active task directory contains only public-domain-repair and portal-player-first-direction task records besides .gitkeep before this audit claim
  - current open PR ownership excludes build-synology-staging-images.yml, codeql.yml, and repair-synology-autostart.yml from mutation here
  - native-auth production verification is archived/completed
  - native-auth ephemeral rehearsal still binds historical Platform/Gateway/Canary/OTClient revisions and a historical missing active-task path; repository search finds no current caller beyond lifecycle/history references
  - native-auth Canary cache build is manual/self-file triggered, binds historical Canary b15b7d544f4795e3a2a65b88de35391b9fd0a20d, and repository search finds no current consumer of its artifact native-auth-rehearsal-canary-cache-headers
  - neither native-auth workflow is required by branch protection or a reusable workflow caller
  - ten retained workflows contain direct path triggers to task Markdown that is no longer present under docs/agents/tasks/active; those task-specific trigger entries have no durable runtime semantics
  - Historical Branch Audit is the accepted ADR 0039 steady-state enforcement surface and remains KEEP
  - Agent Governance contains unique live Issue/task/policy/Control-Room validation and remains KEEP; deterministic overlap alone does not prove safe consolidation
  - parallel-coordinator-prompt-eval remains KEEP because it explicitly evaluates a non-default suite not covered by the default prompt_eval invocation
  - recover-synology-staging-schema remains KEEP as the documented supported manual recovery entry point and is cross-checked by rollback contracts
derived:
  - final fresh verdict distribution is KEEP 43, REPAIR 10, RETIRE 2, CONSOLIDATE 0, UNKNOWN 0
  - retiring the two native-auth historical proof workflows reduces executable workflow count from 55 to 53 without removing a current required/called proof boundary
  - removing stale task-only path entries narrows accidental CI admission; it does not suppress product/script/workflow triggers
unknown: []
conflicts: []
first_failure:
  marker: current-main-agent-governance
  evidence: admission-main Agent Governance checkpoint-validation job 102251475134 failed while protected platform-gate remained PASS; this audit does not treat non-required red governance as removable and must preserve its unique live checks
rejected_hypotheses:
  - workflow age, name, or similarity alone authorizes removal
  - every skipped job is CI waste or a defect
  - Historical Branch Audit may be removed because the historical reconciliation registry is already applied
  - Agent Governance can be deleted because checkpoint/prompt checks overlap core CI
  - task-path trigger cleanup may remove product, script, contract, workflow-self, or operational marker triggers
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260909-ci-lifecycle-cleanup.md
validation:
  - command: live branch protection / ruleset readback
    result: PASS
    evidence: main protected; required context platform-gate; rulesets empty
  - command: current main exact check-run readback
    result: PASS
    evidence: CI run 34282843333 platform-gate job 102251547776 success; classify/test success
  - command: current workflow inventory readback
    result: PASS
    evidence: 55 registered workflows and lifecycle budget 55 before cleanup
  - command: current active task directory readback
    result: PASS
    evidence: exactly two active task Markdown files before this task was claimed
  - command: open PR changed-path ownership refresh
    result: PASS
    evidence: no open PR owns this task's claimed mutation paths; known excluded workflow owners remain untouched
blockers: []
next_action: persist the 55/55 audit ledger, implement the two proven retirements and ten task-trigger repairs with a regression guard, reconcile lifecycle registry to 53, then validate the exact candidate through GitHub Actions
```
