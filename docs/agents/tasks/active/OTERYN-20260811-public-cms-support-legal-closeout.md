---
task_id: OTERYN-20260811-public-cms-support-legal-closeout
mode: implementation
issue: 487
branch: repair/issue-487
status: validating
programme: OTERYN_PLATFORM_REMEDIATION
portal_programme: OTERYN_PORTAL_COMPLETION
---

# OTERYN-20260811-public-cms-support-legal-closeout

## Goal

Close Issue #487 by reconciling every current public portal, CMS, support/moderation, administrator and legal capability/evidence finding against current `main`, adding only missing executable zero-retry evidence or proven runtime repair, and validating the exact resulting head through the canonical Portal Exhaustive Audit.

## Owner execution directive

The owner explicitly authorized autonomous `PORTAL-CLOSEOUT` continuation and removed the foreground execution-budget limit for this task. Repository safety, branch protection, production/deployment, credential, data, external-service and protected-environment authority rules remain controlling.

## Current audit scope

The latest canonical application-state audit used for discovery contained 44 current MEDIUM findings in #487-owned modules:

```yaml
current_finding_counts:
  capability: 10
  state: 12
  overflow: 11
  accessibility: 6
  portability: 2
  viewport: 2
  locale: 1
```

The historical Issue count of 42 is stale. Current exact-head audit evidence controls closure.

`public.character-bazaar` depends on Marketplace surfaces owned by #489. #487 will not invent Marketplace evidence; final closure must either observe #489 already complete or preserve that dependency until #489 is repaired.

## Implemented remediation package

- adds the two missing tablet dimension mappings for `public.localization-core` and `public.news-and-managed-pages`;
- executes the exact `public.community-deaths-and-policy` and `support.moderation-lifecycle` lifecycle specs under bounded Firefox and WebKit projects;
- adds acceptance-only, allowlisted disposable-table failure fixtures; no tracked source file is renamed or hidden;
- adds zero-retry exact-surface 404, 419, 500→recovery and real Support 429 probes;
- adds representative accessibility and horizontal-overflow execution for the six/eleven strictness findings;
- binds the twelve #487 surfaces into `PORTAL_STRICTNESS_EVIDENCE.json` using source markers and only runtime-verified `read_only_surface`, `no_throttle_middleware` or existing operator-locale applicability rules;
- executes the new strictness spec through the existing `resilience-chromium` project, which is mandatory in pull-request `critical` Acceptance E2E.

## Validation gate

```yaml
validation_gate:
  version: 2
  intensity: HEIGHTENED
  classified_by: chatgpt-portal-closeout-20260811-1026
  classified_at: 2026-08-11T10:26:00+02:00
  risk: medium
  triggers:
    - broad public/CMS/support/legal surface package
    - administrator and RBAC evidence included in the owner package
    - zero-retry browser failure/recovery and portability requirements
    - canonical audit closure must be exact-head and fail-closed
  unknown_or_conflict: []
  rationale: Findings are evidence/integration gaps, but privileged administrator paths and broad public browser-state evidence require heightened exact-head validation.
  self_review:
    result: PENDING
    exact_head: none
    evidence: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-11T11:22:00+02:00
head: a53b3614b7b8f8233c1d479c5dc3cc12e70048ae
branch: repair/issue-487
pr: 986
status: validating
context_routes:
  - agent-governance
  - web-cms
  - public-game-data
  - admin-rbac
  - security
  - testing
  - acceptance
  - audit
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260811-public-cms-support-legal-closeout.md
  - docs/testing/PORTAL_STRICTNESS_EVIDENCE.json
  - scripts/acceptance/coverage/portal-evidence-dimensions/modules.json
  - scripts/acceptance/coverage/portal-evidence-dimensions/public-core.json
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/seed-browser-portal-487-strictness.php
  - scripts/acceptance/tests/portal-487-strictness-acceptance.spec.mjs
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260811-public-cms-support-legal-closeout.md
  - docs/testing/PORTAL_STRICTNESS_EVIDENCE.json
  - scripts/acceptance/coverage/portal-evidence-dimensions/modules.json
  - scripts/acceptance/coverage/portal-evidence-dimensions/public-core.json
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/seed-browser-portal-487-strictness.php
  - scripts/acceptance/tests/portal-487-strictness-acceptance.spec.mjs
proven:
  - Issue #487 is the single remediation owner and PR #986 is the single delivery PR.
  - Discovery against canonical audit evidence found 44 current MEDIUM findings: 10 capability, 12 state, 11 overflow, 6 accessibility, 2 portability, 2 viewport and 1 locale.
  - Existing application routes and lifecycle tests prove this is predominantly an evidence-reconciliation package, not blanket missing product behavior.
  - `support-ticket-create` is a real six-per-minute authenticated limiter; the new browser test submits seven valid same-session requests with a valid CSRF token and requires the seventh response to be 429.
  - Disposable failure fixtures map exact #487 surfaces to existing acceptance database tables and restore them in finally/afterEach cleanup.
  - Pull-request Acceptance E2E uses profile `critical`; `critical` executes portability, responsive and resilience, including the new #487 strictness spec and the two newly portable exact lifecycle specs.
  - No application route/view/schema, deployment, production, credential, protected-environment or external-repository behavior has been changed by this package.
derived:
  - Exact-head CI and Portal Exhaustive Audit are now the authoritative mechanism for identifying any wrong applicability assumption, missing evidence marker or actual runtime defect.
unknown:
  - exact first-failure result of the fresh current package generation
  - whether the independent #489 Marketplace dependency lands before #487 reaches final audit closure
conflicts: []
first_failure:
  marker: issue-487-audit-evidence-not-reconciled
  evidence: canonical pre-repair audit emitted 44 MEDIUM findings across #487-owned modules.
rejected_hypotheses:
  - All #487 capabilities are missing; route/action and existing lifecycle evidence disproves blanket absence.
  - Dimension manifest declarations alone are closure evidence; exact browser projects now execute the referenced surfaces.
  - Generic global error pages prove individual #487 surfaces; the package uses exact surface routes or runtime-validated non-applicability.
  - Tracked Blade/source renames are acceptable fault injection; all new server-failure probes use disposable database-table fixtures.
validation:
  - command: current Issue #487 and prior audit reconciliation
    result: PASS
    evidence: 44 current findings and exact owning surfaces were enumerated before implementation.
  - command: route and middleware inspection
    result: PASS
    evidence: exact CMS/support mutation routes and the real support-ticket-create throttle were verified in repository source.
  - command: mandatory Acceptance E2E routing inspection
    result: PASS
    evidence: pull-request critical profile executes portability, responsive and resilience projects at exact head.
blockers: []
next_action: Inspect the fresh exact-head CI, Acceptance E2E and Portal Exhaustive Audit generation; repair the first concrete failure or, if green, perform whole-diff self-review and reconcile the remaining Marketplace dependency.
```
