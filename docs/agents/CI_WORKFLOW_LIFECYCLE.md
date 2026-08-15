# CI Workflow Lifecycle

## Purpose

GitHub Actions workflows are durable CI architecture, not task history. A task, incident, audit, experiment or one-off proof does not earn a permanent workflow merely because a dedicated workflow was convenient while that work was active.

The machine-enforced inventory is `docs/agents/CI_WORKFLOW_LIFECYCLE.json`. `tools/validation/workflow_inventory.py` fails closed when an unregistered workflow appears, a retired workflow returns, a registered workflow disappears without reconciliation, or the reviewed workflow budget is exceeded.

## Creation rule

Default to **reuse or extension** of an existing workflow.

Create a new workflow only when at least one durable property is genuinely distinct:

- trigger/event lifecycle;
- permissions or trust boundary;
- protected environment or runner requirement;
- service/runtime environment that cannot be safely shared;
- externally visible required-check lifecycle;
- scheduled/manual operational lifecycle that must remain independently callable.

A different test filename, feature name, task ID, audit name, prompt name or acceptance profile is not sufficient by itself. Prefer:

1. another job in the existing domain workflow;
2. a matrix/profile/input in an existing workflow;
3. a reusable `workflow_call`;
4. a new workflow only when the durable boundary above requires one.

Every new workflow change must update the lifecycle registry in the same PR and state its review/retirement condition.

## Temporary and task-specific workflows

A task-specific validation workflow is temporary by default.

- Add it only when retained trusted workflows cannot prove the change.
- Scope it to the task and exact safe runner/environment.
- Do not make it a production/deployment bypass.
- Remove it before terminal merge unless a durable lifecycle is explicitly proven and registered.
- Preserve historical evidence in the PR, workflow run and Git history; keeping an executable `.yml` file is not archival policy.

Completed audit/programme workflows are removed when their durable proof has moved into current CI/domain contracts. Unique long-lived manual operational capabilities may remain only when they have a current owner/purpose and are registered.

## Test-routing rule

Workflow-file edits are routed by the smallest proving layer:

- `ci.yml`, `scripts/ci/**`, `tests/ci/**`: fail closed to all routing gates because they control gate selection;
- a heavy lane's own workflow definition: core CI plus that lane only;
- ordinary workflow definitions: core CI plus the workflow's own event-triggered validation;
- agent-governance workflow changes: dedicated Agent Governance validation.

Do not interpret “workflow changed” as “every runtime risk changed.”

## Pull-request and main-push economy

Automatic domain workflows must use `paths` or `paths-ignore` at event admission whenever the workflow is not intentionally repository-wide. Supersedable PR workflows must cancel an older run for the same PR/ref.

Documentation/task/governance-only commits must not start unrelated application, browser, outage, edge, concurrency or container proof.

Heavy validation is run once on the coherent exact candidate head unless an earlier heavy feasibility proof is justified by the risk.

## Coverage measurement

Normal pull requests keep coverage instrumentation off so the fast path remains fast. PHP application coverage is measured on relevant `main` pushes in the existing `CI` workflow and stored as a bounded artifact.

`docs/agents/CI_COVERAGE_POLICY.json` starts in `report_only` mode because historical CI had coverage disabled and therefore no verified baseline exists. After an observed stable baseline, promote the policy to `enforce` with a reviewed floor. Never invent a percentage or lower an enforced floor merely to make CI green.

Coverage is one signal, not a substitute for focused negative-path, integration, concurrency, browser or contract proof.

## Current cleanup

Issue #1085 retires these obsolete workflow definitions without removing their proving coverage:

- `account-security-format-diagnostics.yml` → blocking Pint in `ci.yml`;
- `account-security-static-diagnostics.yml` → blocking PHPStan/Larastan in `ci.yml`;
- `portal-exhaustive-acceptance.yml` → direct `acceptance-validation.yml` critical profile;
- `portal-exhaustive-trigger-coupling.yml` → lifecycle registry + trigger-economy contract;
- `portal-exhaustive-audit.yml` → current strict ledgers/focused domain validation and explicit `portal-e2e-audit.yml` orchestration;
- `deep-system-validation.yml` → current risk-routed CI/domain workflows and explicit comprehensive audit orchestration.

Their historical runs and Git history remain provenance. Retiring a workflow does not erase evidence.

## Review checklist

Before merging a CI workflow change, verify:

- changed path has a durable workflow owner or is explicitly temporary;
- no existing workflow/reusable profile already provides the required proof;
- PR and `main` triggers are no broader than necessary;
- concurrency cancels superseded PR work when safe;
- permissions are minimum necessary;
- external actions are immutable-SHA pinned;
- artifacts have bounded retention and secret-safe content;
- heavy jobs are routed only for affected risk;
- exact-head required checks remain green;
- lifecycle registry and workflow inventory agree;
- task-specific workflow retirement is complete or explicitly justified.
