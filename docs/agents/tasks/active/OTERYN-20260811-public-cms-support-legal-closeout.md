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

The next exact-head run exposed four declaration/evidence mismatches and one WebKit login-helper stabilization failure. The canonical surface declarations now match the newly executed tablet and Firefox/WebKit evidence, and the shared acceptance login helper uses bounded identity-first/secret-last field stabilization while Playwright retries remain zero.

Current `main` advanced during repair through Issue #691/PR #1000 and added Portal Exhaustive companion workflow coupling. The repair branch was synchronized from the conflict-free GitHub test-merge tree so the validation candidate incorporates current main before final audit and E2E.

## Validation gate

```yaml
validation_gate:
  version: 2
  intensity: HEIGHTENED
  risk: medium
  rationale: Broad public/CMS/support/admin evidence and zero-retry browser failure/recovery paths require heightened exact-head validation.
  self_review:
    result: PASS
    exact_head: 3db2641d2f8e4d300aa100a699b697265627a8a3
    evidence:
      - Comparison 8584a96adfe2fa220b4b9c70e1575c5f7a98ca5b..3db2641d2f8e4d300aa100a699b697265627a8a3 contains exactly the 11 declared #487 paths.
      - The canonical manifest repair is exactly two tablet insertions; the two surface declarations add only Firefox/WebKit identifiers.
      - The WebKit helper repair is bounded to credential-field stabilization and preserves Playwright retries=0.
      - The strictness fixture is acceptance-environment-gated, allowlisted, disposable and restores renamed tables in cleanup.
      - Full Playwright configuration proves public-localization.spec.mjs is already selected by responsive-desktop/tablet/mobile, so the localization tablet mapping is executable rather than declaration-only.
      - PR #986 has no review submissions, unresolved review threads or requested changes at this checkpoint.
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-11T16:08:56+02:00
head: 3db2641d2f8e4d300aa100a699b697265627a8a3
base_head: 8584a96adfe2fa220b4b9c70e1575c5f7a98ca5b
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
  - PR #986 remained on exact head 5a41ae1d55530c8550c1ba8901d1c54a00678cdf through 2026-08-11T15:44:49+02:00, more than the 45-minute ownership lease after its last mutation, so the previous execution was recovered as orphaned with no conflicting live branch mutation observed.
  - Exact-head Portal Acceptance Contract run 31491831223 proved four deterministic declaration mismatches: tablet mappings for public.news-and-managed-pages and public.localization-core, plus browser mappings for public.community-deaths-and-policy and support.moderation-lifecycle.
  - Exact-head Acceptance E2E artifact 9101460137 contained 54 portability tests with exactly one failure: the WebKit support-moderation lifecycle reached the shared login helper after the password field was cleared when the email field was populated.
  - Commit 2360a05dca84db501a3a4d245191eaa6abcadfbf changes portal-coverage-manifest.json by exactly two added tablet declarations and no other lines.
  - Commits 7928bcfaa1f9474b228036f4fc202ea07893bf31 and 6bf17e42d40c80edd513060aacece6a7f4c648b4 add only Firefox/WebKit browser declarations for the two #487 portability surfaces.
  - Commit 02e6e2145bb9c5c53db78b09ffe0b4581b34a98c changes only the shared login helper, adding a three-attempt bounded field-stabilization loop while preserving zero Playwright retries.
  - Comparison 8f6520c391c1e4a8242f95dfaaf15ad93ccb4d4a..2360a05dca84db501a3a4d245191eaa6abcadfbf contains only the four targeted follow-up repair files.
  - Current main advanced to 8584a96adfe2fa220b4b9c70e1575c5f7a98ca5b through PR #1000; its new paths do not overlap the 11 #487 paths, but its Portal Exhaustive workflow coupling is relevant to final validation.
  - GitHub produced conflict-free test merge 3f94b607d91d17085023213337f22ed30fc528b1 with tree 9b04cf3b1cb1b827e21875502f31d3cc8a36e920 for main 8584a96adfe2fa220b4b9c70e1575c5f7a98ca5b plus repair head c983e84fa76f84fb278110d9dfd11aee980b9306; that exact tree was committed as branch merge 3db2641d2f8e4d300aa100a699b697265627a8a3.
  - Comparison current-main 8584a96adfe2fa220b4b9c70e1575c5f7a98ca5b..3db2641d2f8e4d300aa100a699b697265627a8a3 contains exactly the 11 declared #487 paths.
  - Full playwright.config.mjs already selects public-localization.spec.mjs in responsiveMatches, so desktop/tablet/mobile localization evidence is actually executable; no routing repair is required.
  - PR #986 has no review submissions or review threads at the current-main-synchronized checkpoint.
  - No application route, view, product schema, deployment, credential, protected environment or external repository was mutated by this package.
derived:
  - The four strict-contract failures were declaration drift against already executed evidence and are repaired without deleting the evidence that closes the #487 findings.
  - The WebKit failure was a shared acceptance login-fixture stabilization defect, not evidence that the support/moderation product lifecycle is unavailable.
  - Fresh exact-head CI, Acceptance E2E and Portal Exhaustive Audit on the current-main-synchronized head are authoritative for final closure.
unknown:
  - Result of the next exact-head validation generation after the localized-home graceful-degradation assertion repair.
  - Whether Issue #489 lands before the final #487 audit reconciliation; it is currently still open and remains the declared Character Bazaar dependency.
conflicts: []
first_failure:
  marker: exact-head-strict-dimension-contract-mismatch
  evidence: Portal Acceptance Contract run 31491831223 reported exactly four mapping/declaration mismatches for the two #487 viewport findings and two #487 portability findings; all four declarations are now reconciled.
rejected_hypotheses:
  - All #487 capabilities are missing; existing routes and lifecycle evidence disprove blanket absence.
  - Firefox or WebKit broadly break the #487 community or support lifecycle; the previous exact-head portability artifact had 53 passes and one shared login-helper failure.
  - Expanding Issue #350 stress evidence to portability is valid; doing so would falsely broaden a separate evidence contract.
  - Removing the newly executed tablet or cross-engine evidence is a valid strict-contract repair; that would re-create the exact #487 findings being closed instead of reconciling canonical declarations.
  - public.localization-core tablet mapping is declaration-only because public-localization.spec.mjs is absent from the responsive profile; full playwright.config.mjs proves the spec is already included in responsiveMatches.
changed_paths:
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
validation:
  - command: Exact-head GitHub Actions generation on 56c79dab38e6bcd9b773f1b56f713a189dda03ee
    result: FAIL
    evidence: Acceptance E2E 31534516329, Deep System Validation 31534516349 and Portal Exhaustive Acceptance 31534516662 all reproduced one root failure: the localized homepage /en intentionally returned graceful HTTP 200 after news_posts removal while the strictness test incorrectly expected 500.
  - command: node --check scripts/acceptance/tests/portal-487-strictness-acceptance.spec.mjs and git diff --check
    result: PASS
    evidence: The focused repair routes public.localization-core through the existing graceful-home recovery assertion, retains the UNAVAILABLE content-state check and leaves public.news-and-managed-pages as the fail-closed 500 probe.
  - command: Portal Exhaustive Acceptance E2E 31537424537 on 2da3172cc4fbdf73af3e925e8033395078613110
    result: FAIL
    evidence: The repaired graceful homepage and localized homepage probes passed; the next first failure was a later legacy /support layout probe returning 404. The acceptance path now uses the canonical localized /en/support destination already emitted by homepage navigation and owned by support.index.
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
    evidence: Exactly four deterministic mapping/declaration mismatches remained: two viewport declarations and two browser declarations; each has a targeted repair commit.
  - command: Acceptance E2E run 31491831043 artifact 9101460137 on 5a41ae1d55530c8550c1ba8901d1c54a00678cdf
    result: FAIL
    evidence: Primary Chromium smoke passed; portability produced 53 passes and one WebKit support-moderation login-helper password-value failure; commit 02e6e2145bb9c5c53db78b09ffe0b4581b34a98c targets that failure.
  - command: targeted follow-up diff inspection 8f6520c391c1e4a8242f95dfaaf15ad93ccb4d4a..2360a05dca84db501a3a4d245191eaa6abcadfbf
    result: PASS
    evidence: Only the four intended follow-up repair files changed; the large canonical manifest edit is exactly two insertions.
  - command: current-main synchronization via GitHub conflict-free test-merge tree
    result: PASS
    evidence: Tree 9b04cf3b1cb1b827e21875502f31d3cc8a36e920 merged 8584a96adfe2fa220b4b9c70e1575c5f7a98ca5b with c983e84fa76f84fb278110d9dfd11aee980b9306 and was persisted as 3db2641d2f8e4d300aa100a699b697265627a8a3 without force.
  - command: self-review of current-main diff 8584a96adfe2fa220b4b9c70e1575c5f7a98ca5b..3db2641d2f8e4d300aa100a699b697265627a8a3
    result: PASS
    evidence: Exact 11-path scope verified; manifest, surface declarations, strictness fixtures/tests, Playwright profile routing and WebKit helper were inspected with no open material self-review finding.
blockers: []
next_action: Commit and push the focused localized-home assertion repair, then inspect the resulting exact-head required validation once and complete merge closeout only on PASS.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 2
  session_id: 20260811T154449+0200-repair-487
  session_started_at: 2026-08-11T15:44:49+02:00
  checkpointed_at: 2026-08-11T16:08:56+02:00
  last_progress_at: 2026-08-11T16:08:56+02:00
  invocation_started_at: 2026-08-11T15:44:49+02:00
  phase: current-main-exact-head-validation
  exact_head: 3db2641d2f8e4d300aa100a699b697265627a8a3
  base_head: 8584a96adfe2fa220b4b9c70e1575c5f7a98ca5b
  pull_request: 986
  active_operation: observe current-main exact-head workflows
  external_run_ids: []
  operation_started_at: 2026-08-11T16:08:56+02:00
  wait_deadline_at: null
  ci_checks_for_current_head: 0
  ci_check_generation: current-base
  terminal_ci_wait_started_at: null
  terminal_ci_checks_for_current_generation: 0
  unchanged_state_checks: 0
  identical_failure_retries: 0
  repair_cycles_for_current_gate: 2
  context_reconstruction_attempts: 1
  stall_warnings: 0
  status: active
  safe_to_resume: true
  resume_condition: PR #986 remains the single Issue #487 delivery PR and the current-main exact-head workflow generation is available for aggregate inspection.
  next_action: Inspect the first aggregate exact-head workflow generation for the current-main-synchronized checkpoint; repair only a concrete new failure, otherwise complete fresh audit/E2E and merge gates.
```
