---
task_id: OTERYN-20260807-issue-365-wiki-flash-regression
issue: 365
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: true
status: completed_on_merge
risk: medium
validation_intensity: STANDARD
delivery_pull_request: 837
delivery_branch: fix/issue-365-wiki-flash-regression
production_activation_authorized: false
cross_repository_mutation_authorized: false
ownership: releases_on_merge
---

# OTERYN-20260807 issue 365 Wiki publication flash regression — completed on merge

## Archive condition

```yaml
archive_state:
  status: completed_on_merge
  effective_when:
    pull_request: 837
    branch: fix/issue-365-wiki-flash-regression
    merged: true
  invalidated_by:
    - PR #837 closed without merge
    - final PR generation does not pass repository-required CI, Agent Governance or applicable zero-retry browser acceptance
```

This record is conditional until PR #837 merges through protected `main`.

## Delivered scope

The repair restores the accessible `Wiki article published.` assertion in the standard Wiki administration journey and fails that clean journey when existing browser diagnostics record any unexplained HTTP >=500 response. Durable `Published` state and `Unpublish to draft` assertions remain intact.

No application/session/authentication/authorization/media/publication runtime behavior, migration, dependency, workflow, deployment, production system, secret or external repository is changed.

## Evidence boundary

- Historical intermittent publication-flash loss is proven; its exact historical runtime root cause remains unknown.
- Frozen matrix run `31097086526` passed the explicit publication flash in 12/12 zero-retry samples.
- Six controlled corrupt-media samples in that matrix also retained the flash, so corrupt thumbnails are not claimed as the historical flash-loss cause.
- PR #751 independently repaired the proven Editorial Media fixture-isolation defect.
- This repair adds regression/evidence enforcement only; it does not introduce a speculative runtime workaround.

## Acceptance and validation gate

Before protected merge, the final effective PR diff must remain bounded to the Wiki acceptance regression plus this lifecycle record, exact-head self-review must be `PASS` with zero material findings, unresolved review threads must be zero, required exact-head CI and Agent Governance must pass, and applicable zero-retry browser acceptance must pass.

Any final-head change supersedes prior workflow evidence and requires the latest generation to satisfy the gate.

## Issue disposition

PR #837 closes Issue #365 only for the bounded acceptance criteria now proven and protected by regression coverage. Closure must not be interpreted as proving the unknown historical runtime mechanism or as a product-runtime change.

## Rollback

Revert the protected squash merge for PR #837 to remove this regression gate. Do not weaken retries, suppress HTTP 5xx diagnostics or invent a runtime cause as a rollback substitute.

## Ownership release

On successful PR #837 merge, implementation ownership of these paths is released:

- `scripts/acceptance/tests/admin-wiki-administration.spec.mjs`
- `docs/agents/tasks/active/OTERYN-20260807-issue-365-wiki-flash-regression.md`
- `docs/agents/tasks/archive/OTERYN-20260807-issue-365-wiki-flash-regression.md`

No continuation authority remains after merge unless a new live finding is separately created and claimed.
