---
task_id: OTERYN-20260811-engineering-excellence-hardening
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
  - docs/architecture/TEST_STRATEGY.md
search_first:
  - Issue #1002
  - PR #1001 Actions-major cleanup ownership
  - PR #992 agent-governance ownership
  - PR #338 Game Catalog consumer ownership
optional_reads: []
---

# OTERYN-20260811-engineering-excellence-hardening

## Goal

Close Issue #1002 by making repository verification, E2E dependency installation, prompt/governance regression checks, selected CI routing, Game Catalog staging validation, and Synology staging release identity more deterministic and maintainable without changing product behavior or performing protected live operations.

## Acceptance criteria

- [ ] `scripts/acceptance/package-lock.json` is committed, consistent with `package.json`, and acceptance workflows use deterministic `npm ci` where this task owns the install path.
- [ ] Dependabot covers npm dependencies under `/scripts/acceptance`.
- [ ] One canonical developer verification command exists and covers Composer validation/audit, formatting, static analysis, base tests, strict acceptance-contract checks, and integration-test registration.
- [ ] Every `tests/Integration/**/*Test.php` is machine-registered to an explicit proving workflow/command; unregistered integration tests fail validation.
- [ ] Game Catalog cross-repository staging is selected semantically and no longer keyed to historical PR #272.
- [ ] Prompt/agent-governance changes have executable deterministic regression fixtures, with stochastic model-behavior limits stated truthfully.
- [ ] Agent Governance executes the applicable deterministic prompt/governance regression checks after overlapping PR #992 releases ownership.
- [ ] Workflow complexity/drift is reduced with machine-enforced classification/consistency rather than additional narrative-only rules, reusing current-main governance after #992 rather than duplicating it.
- [ ] Synology staging deployment resolves Platform/Gateway images from an explicit `sha-<40 hex>` release identity to immutable image digests before deployment.
- [ ] Exact-head self-review is PASS, applicable validation is green, E2E is PASS or specifically NOT_APPLICABLE, related PRs are intentional, and required exact-head CI passes before merge.

## Ownership

```yaml
project_lane: oteryn-platform-core
task_kind: implementation
risk_gate: HEIGHTENED
owned_paths:
  - composer.json
  - scripts/acceptance/package-lock.json
  - .github/dependabot.yml
  - tests/Integration/REGISTRY.json
  - tools/validation/verify_integration_test_registration.py
  - tools/validation/test_verify_integration_test_registration.py
  - .github/workflows/game-catalog-contract.yml
  - .github/workflows/deploy-synology-staging.yml
  - docs/agents/tasks/active/OTERYN-20260811-engineering-excellence-hardening.md
deferred_overlap_paths:
  - path: .github/workflows/deep-system-validation.yml
    owner: PR #1001
    rule: do not edit until PR #1001 is terminal and current main is refreshed
  - path: .github/workflows/agent-governance.yml
    owner: PR #992
    rule: do not edit until PR #992 is terminal and current main is refreshed
  - path: tools/agents/**
    owner: PR #992 for policy-consistency surfaces
    rule: extend current-main implementation after #992 instead of duplicating it
modules:
  - build-test
  - ci
  - agent-governance
  - synology-staging-deployment
dependencies:
  - Issue #1002
  - terminal state of PR #1001 before deep-system workflow edit
  - terminal state of PR #992 before agent-governance/tool extension
blockers:
  - none for currently non-overlapping implementation
cross_repository_tasks:
  - none
```

PR #338 does not change `.github/workflows/game-catalog-contract.yml`; its changed-file inventory was checked before claiming that path. No external repository write is part of this task.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-11T13:45:04Z
head: 681455739a054f344dc0e9478ff79821ac4a401d
branch: fix/engineering-excellence-hardening
pr: none
status: implementing
phase: non_overlapping_implementation
session_id: agent-20260811T134504Z
session_role: implementer
execution_mode: github
execution_reason: repository writes and exact-head validation are available through the authenticated GitHub connector and Actions
context_routes:
  - testing
  - agent-governance
  - ci-repair
context_pressure: high
context_growth: stable
context_score: 10
decomposition_decision: phased
decomposition_reason: one cohesive engineering-verification contract with temporary path-release dependencies on two current PRs
invocation_started_at: 2026-08-11T13:40:48Z
last_progress_at: 2026-08-11T13:45:04Z
ci_checks_for_current_head: 0
ci_check_generation: draft
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 0
stall_warnings: 0
owned_paths:
  - composer.json
  - scripts/acceptance/package-lock.json
  - .github/dependabot.yml
  - tests/Integration/REGISTRY.json
  - tools/validation/verify_integration_test_registration.py
  - tools/validation/test_verify_integration_test_registration.py
  - .github/workflows/game-catalog-contract.yml
  - .github/workflows/deploy-synology-staging.yml
  - docs/agents/tasks/active/OTERYN-20260811-engineering-excellence-hardening.md
proven:
  - trusted base main is 681455739a054f344dc0e9478ff79821ac4a401d and protected with required contexts classify-changes and test
  - scripts/acceptance/package.json pins @playwright/test 1.60.0 but the acceptance package has no committed package-lock on the audited base
  - phpunit.xml registers Unit and Feature suites while the only current tests/Integration test is invoked explicitly by game-catalog-contract.yml
  - PR #1001 currently owns deep-system-validation.yml for Actions-major reconciliation
  - PR #992 currently owns Agent Governance policy-consistency surfaces
  - PR #338 changed-file inventory does not include game-catalog-contract.yml
derived:
  - lockfile generation can be bootstrapped safely on the task branch through GitHub Actions if no local network-capable npm environment is available
  - prompt evaluation should extend, not duplicate, the policy-consistency machinery that lands from PR #992
unknown:
  - final current-main form of overlapping workflow/governance files after PRs #1001 and #992 become terminal
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - adding tests/Integration directly to phpunit.xml; the existing cross-repository test requires external generated snapshot environment and would make the default suite invalid
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260811-engineering-excellence-hardening.md
validation:
  - command: repository/PR ownership preflight
    result: PASS
    evidence: live main, active tasks and open PR changed-file inventories inspected before path claim
blockers:
  - none for currently non-overlapping implementation
next_action: Open the draft PR, then implement the deterministic dependency, verification, Game Catalog trigger and staging digest changes without touching #1001/#992-owned paths.
```

## Notes

Issue: #1002. This task changes repository and staging deployment definitions only. It does not authorize a staging deployment, production deployment, environment approval, credential mutation, live database mutation, payment action, or cross-repository write.