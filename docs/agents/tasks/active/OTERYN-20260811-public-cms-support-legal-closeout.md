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

Close Issue #487 by reconciling the current public portal, CMS, support/moderation, administrator and legal audit findings with executable zero-retry evidence or proven runtime repair, then validate and close the exact resulting head.

## Owner execution directive

The owner authorized autonomous `PORTAL-CLOSEOUT` continuation. Repository safety, branch protection, production/deployment, credential, data, external-service and protected-environment authority rules remain controlling.

## Scope and implementation

Discovery found 44 current MEDIUM findings in #487-owned modules: 10 capability, 12 state, 11 overflow, 6 accessibility, 2 portability, 2 viewport and 1 locale. `public.character-bazaar` remains an explicit dependency on Issue #489 and will not be falsely closed by #487.

The package adds the two missing tablet evidence mappings, real Firefox/WebKit execution for the #487 community and support lifecycles, acceptance-only disposable-table failure fixtures, exact 404/419/500-to-recovery probes, a real Support 429 probe, accessibility/overflow execution, and strictness evidence reconciliation. It does not mutate application routes, product schema, production, credentials or protected environments.

The first fresh portability run exposed an unrelated Issue #350 assertion inside `community-data-acceptance.spec.mjs`: that stress extension intentionally permits only its declared Chromium projects. The actual #487 community lifecycle and support-moderation tests passed in Chromium, Firefox and WebKit. The portability projects now exclude only the `@portal-community-stress` marker while retaining the #487 lifecycle in all three browser engines.

## Validation gate

```yaml
validation_gate:
  version: 2
  intensity: HEIGHTENED
  risk: medium
  rationale: Broad public/CMS/support/admin evidence and zero-retry browser failure/recovery paths require heightened exact-head validation.
  self_review:
    result: PENDING
    exact_head: none
    evidence: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-11T14:15:00+02:00
head: 6fb8227424d3e76f0d7b6895d2c9263d4bfbf058
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
proven:
  - Issue #487 is the single remediation owner and PR #986 is the single delivery PR.
  - Canonical discovery found 44 current MEDIUM findings in the #487-owned modules.
  - The support ticket create route has a real six-per-minute authenticated limiter and the new test requires request seven to return 429.
  - The failure fixture is acceptance-only, allowlisted and restores disposable database-table availability in cleanup.
  - Acceptance run 31477217281 had exactly three portability failures and all were the same unrelated Issue #350 project-contract assertion.
  - In run 31477217281 the actual #487 community lifecycle and both support-moderation tests passed under Chromium, Firefox and WebKit.
  - Portability now excludes only the Issue #350 stress marker and preserves the #487 lifecycle under all three engines.
  - The branch was rebuilt on main 859204778f04f3e5993e1534ae7b03b7644849f0 after confirming no intervening main change overlapped the seven #487-owned paths.
  - No application route, view, product schema, deployment, credential, protected environment or external repository was mutated by this package.
derived:
  - Fresh exact-head CI, Acceptance E2E and Portal Exhaustive Audit are authoritative for final closure.
unknown:
  - Result of the next exact-head generation after the checkpoint-contract repair.
  - Whether Issue #489 lands before the final #487 audit reconciliation.
conflicts: []
first_failure:
  marker: issue-350-stress-project-contract-in-portability-profile
  evidence: Acceptance artifact 9095860995 reported three identical failures because the Issue #350 stress evidence intentionally omits portability-chromium, portability-firefox and portability-webkit.
rejected_hypotheses:
  - All #487 capabilities are missing; existing routes and lifecycle evidence disprove blanket absence.
  - Firefox or WebKit broke the #487 community or support lifecycle; those lifecycle tests passed in all three engines in the failing run.
  - Expanding Issue #350 stress evidence to portability is valid; doing so would falsely broaden a separate evidence contract.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260811-public-cms-support-legal-closeout.md
  - docs/testing/PORTAL_STRICTNESS_EVIDENCE.json
  - scripts/acceptance/coverage/portal-evidence-dimensions/modules.json
  - scripts/acceptance/coverage/portal-evidence-dimensions/public-core.json
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/seed-browser-portal-487-strictness.php
  - scripts/acceptance/tests/portal-487-strictness-acceptance.spec.mjs
validation:
  - command: current Issue #487 and canonical pre-repair audit reconciliation
    result: PASS
    evidence: 44 current findings and exact owning surfaces were enumerated before implementation.
  - command: route and middleware inspection
    result: PASS
    evidence: Exact CMS/support mutation routes and the support-ticket-create throttle were verified in repository source.
  - command: Acceptance E2E run 31477217281 artifact 9095860995
    result: FAIL
    evidence: The only three portability failures were the unrelated Issue #350 project-contract assertion; the #487 browser lifecycles themselves passed in all three engines and the profile routing was repaired.
  - command: main overlap comparison fb473b5030e20886692e0833ab944f0717ab3ab7..859204778f04f3e5993e1534ae7b03b7644849f0
    result: PASS
    evidence: Intervening main changes did not touch any of the seven #487-owned changed paths before the branch rebuild.
blockers: []
next_action: Inspect the fresh exact-head CI, Acceptance E2E and Portal Exhaustive Audit generation and repair the first concrete failure, or perform terminal self-review and dependency reconciliation if all #487 gates are green.
```
