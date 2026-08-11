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
updated_at: 2026-08-11T15:44:49+02:00
head: 5a41ae1d55530c8550c1ba8901d1c54a00678cdf
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
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - scripts/acceptance/coverage/surfaces/community-data-completeness.json
  - scripts/acceptance/coverage/surfaces/support-moderation-lifecycle.json
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/seed-browser-portal-487-strictness.php
  - scripts/acceptance/tests/helpers.mjs
  - scripts/acceptance/tests/portal-487-strictness-acceptance.spec.mjs
proven:
  - Issue #487 is the single remediation owner and PR #986 is the single delivery PR.
  - Canonical discovery found 44 current MEDIUM findings in the #487-owned modules.
  - The support ticket create route has a real six-per-minute authenticated limiter and the new test requires request seven to return 429.
  - The failure fixture is acceptance-only, allowlisted and restores disposable database-table availability in cleanup.
  - Acceptance run 31477217281 had exactly three portability failures and all were the same unrelated Issue #350 project-contract assertion.
  - In run 31477217281 the actual #487 community lifecycle and both support-moderation tests passed under Chromium, Firefox and WebKit.
  - Portability now excludes only the Issue #350 stress marker and preserves the #487 lifecycle under all three engines.
  - The branch was rebuilt on main 859204778f04f3e5993e1534ae7b03b7644849f0 after confirming no intervening main change overlapped the original seven #487-owned paths.
  - PR #986 remained on exact head 5a41ae1d55530c8550c1ba8901d1c54a00678cdf through 2026-08-11T15:44:49+02:00, more than the 45-minute ownership lease after its last mutation, so the previous execution is recovered as orphaned with no conflicting live branch mutation observed.
  - Current main 681455739a054f344dc0e9478ff79821ac4a401d is two commits ahead of the branch base and changes only composer.lock plus the unrelated dependency-refresh task archival; it does not overlap the expanded #487 repair paths.
  - Exact-head Portal Acceptance Contract run 31491831223 proves four deterministic declaration mismatches: tablet mappings for public.news-and-managed-pages and public.localization-core, plus browser mappings for public.community-deaths-and-policy and support.moderation-lifecycle.
  - Exact-head Acceptance E2E artifact 9101460137 contains 54 portability tests with exactly one failure: the WebKit support-moderation lifecycle reached the shared login helper after the password field was cleared when the email field was populated.
  - No application route, view, product schema, deployment, credential, protected environment or external repository was mutated by this package.
derived:
  - The dimension records already prove the intended tablet and cross-engine execution, so the four strict-contract mismatches require the corresponding canonical surface declarations to be reconciled rather than deleting the new evidence.
  - The WebKit failure is a shared acceptance login-fixture stabilization defect, not evidence that the support/moderation product lifecycle is unavailable.
  - Fresh exact-head CI, Acceptance E2E and Portal Exhaustive Audit are authoritative for final closure.
unknown:
  - Result of the next exact-head generation after the verified declaration and WebKit login repairs.
  - Whether Issue #489 lands before the final #487 audit reconciliation.
conflicts: []
first_failure:
  marker: exact-head-strict-dimension-contract-mismatch
  evidence: Portal Acceptance Contract run 31491831223 reports exactly four mapping/declaration mismatches for the two #487 viewport findings and two #487 portability findings.
rejected_hypotheses:
  - All #487 capabilities are missing; existing routes and lifecycle evidence disprove blanket absence.
  - Firefox or WebKit broadly break the #487 community or support lifecycle; the current exact-head portability artifact has 53 passes and one shared login-helper failure.
  - Expanding Issue #350 stress evidence to portability is valid; doing so would falsely broaden a separate evidence contract.
  - Removing the newly executed tablet or cross-engine evidence is a valid strict-contract repair; that would re-create the exact #487 findings being closed instead of reconciling canonical declarations.
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
  - command: Portal Acceptance Contract run 31491831223 on 5a41ae1d55530c8550c1ba8901d1c54a00678cdf
    result: FAIL
    evidence: Exactly four deterministic mapping/declaration mismatches remain: two viewport declarations and two browser declarations.
  - command: Acceptance E2E run 31491831043 artifact 9101460137 on 5a41ae1d55530c8550c1ba8901d1c54a00678cdf
    result: FAIL
    evidence: Primary Chromium smoke passed; portability produced 53 passes and one WebKit support-moderation login-helper password-value failure.
  - command: main overlap comparison 859204778f04f3e5993e1534ae7b03b7644849f0..681455739a054f344dc0e9478ff79821ac4a401d
    result: PASS
    evidence: Intervening main changes touch composer.lock and the dependency-refresh task archival only, with no #487 repair-path overlap.
blockers: []
next_action: Apply the verified canonical surface-declaration reconciliation and WebKit login-fixture stabilization, then inspect the resulting exact-head validation generation.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 1
  session_id: 20260811T154449+0200-repair-487
  session_started_at: 2026-08-11T15:44:49+02:00
  checkpointed_at: 2026-08-11T15:44:49+02:00
  last_progress_at: 2026-08-11T15:44:49+02:00
  phase: repair-failing-exact-head-gates
  exact_head: 5a41ae1d55530c8550c1ba8901d1c54a00678cdf
  pull_request: 986
  active_operation: none
  external_run_ids: [31491830977, 31491830999, 31491831043, 31491831052, 31491831223]
  operation_started_at: null
  wait_deadline_at: null
  check_generation: exact-head-5a41ae1d
  checks_used: 1
  status: active
  safe_to_resume: true
  resume_condition: PR #986 remains the single Issue #487 delivery PR and no conflicting live mutation appears on repair/issue-487.
  next_action: Apply the verified canonical surface-declaration reconciliation and WebKit login-fixture stabilization, then inspect the resulting exact-head validation generation.
```
