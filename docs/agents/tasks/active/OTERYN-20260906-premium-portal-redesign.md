---
task_id: OTERYN-20260906-premium-portal-redesign
governing_issue: 1297
required_reads:
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/agents/BUILD_TEST_MATRIX.md
search_first:
  - resources/views
  - public/css
  - scripts/acceptance
optional_reads: []
---

# Complete Oteryn player portal redesign

## Goal

Issue #1297; the owner's full 2026-09-07 brief supersedes the earlier conservative presentation scope. Deliver a substantially new complete player-facing Laravel/Blade portal through PR #1298 for human visual review. No merge, auto-merge, deployment, production access or backend/security contract change.

## Acceptance criteria

- [x] Recover the previously prepared implementation without starting a competing PR.
- [x] Establish a standalone public design system, grouped navigation and new homepage composition.
- [x] Implement dedicated editorial, world/player/community, knowledge, account and identity page families.
- [ ] Reconcile every player-facing route/view in the coverage ledger.
- [ ] Verify actual guest/authenticated, EN/PL and important edge-state workflows.
- [ ] Inspect phone/tablet/desktop/wide Laravel renders and correct findings.
- [ ] Pass relevant feature/static/browser checks and full exact-head self-review.
- [ ] Remove the temporary tracked-source transport and publish final evidence.
- [ ] Mark the single PR ready for human visual review, without merging it.

## Ownership

```yaml
owned_paths:
  - public/css/portal-system.css
  - public/css/portal-pages.css
  - public/css/home-production.css
  - public/js/portal-navigation.js
  - public/images/oteryn-world.svg
  - resources/views/home.blade.php
  - resources/views/game/**
  - resources/views/identity/**
  - resources/views/characters/**
  - resources/views/game-auth/oauth/**
  - resources/views/game-catalog/**
  - resources/views/news/**
  - resources/views/pages/**
  - resources/views/events/**
  - resources/views/announcements/**
  - resources/views/public/**
  - resources/views/downloads/**
  - resources/views/wiki/**
  - resources/views/support/**
  - resources/views/marketplace/**
  - resources/views/payments/**
  - resources/views/player-companion/**
  - resources/views/errors/**
  - lang/en/portal.php
  - lang/pl/portal.php
  - scripts/acceptance/tests/portal*.mjs
  - scripts/acceptance/tests/00-portal-source-recovery.spec.mjs
  - scripts/acceptance/tests/accessibility-critical.spec.mjs
  - scripts/acceptance/tests/homepage-navigation-seo.spec.mjs
  - scripts/acceptance/tests/portability-critical.spec.mjs
  - scripts/acceptance/tests/responsive-critical.spec.mjs
  - tests/Feature/HomeTest.php
  - tests/Feature/PublicPortalRedesignTest.php
  - docs/agents/tasks/active/OTERYN-20260906-premium-portal-redesign.md
  - docs/testing/PORTAL_REDESIGN_REVIEW_2026-09-06.md
modules: [PublicPortal, QualityE2E]
dependencies: [existing isolated acceptance runtime]
blockers: []
cross_repository_tasks: []
```

All admin views, backend/domain/controllers/routes, deployment, workflow and protection paths are excluded. Existing browser tests may adapt navigation selectors to the new interaction, not weaken their assertions. #1294 and #1303 own unrelated audit/governance paths; the catalog consumer #338 remains untouched.

Parallel-first lanes: inventory, coherent implementation, visual/functional QA and integration. This invocation executes them serially because the exposed tools have no independent-agent execution action; one writer owns the existing branch and shared CSS/Blade composition. GitHub-hosted independent suites run concurrently. No fictitious delegation or model/effort setting is claimed.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-07T09:13:00Z
head: be67a1e343780449fabd9e3d236643363a0304ab
branch: feat/20260906-premium-portal-redesign
pr: 1298
status: implementing
context_routes: [web-cms, auth-identity, public-game-data, testing, agent-governance]
owned_paths:
  - public/css/**
  - public/js/portal-navigation.js
  - public/images/oteryn-world.svg
  - resources/views/**
  - lang/en/portal.php
  - lang/pl/portal.php
  - scripts/acceptance/tests/**
  - tests/Feature/HomeTest.php
  - tests/Feature/PublicPortalRedesignTest.php
  - docs/agents/tasks/active/OTERYN-20260906-premium-portal-redesign.md
  - docs/testing/PORTAL_REDESIGN_REVIEW_2026-09-06.md
proven:
  - Current main is 3557085c20512d25576d8884cc54471665784b00; its only delta from admission is D26 task-inventory governance, not runtime or governing instructions.
  - PR 1298 was still draft at 06afcd7004246c5388ccd625346baa232a4bc55a before recovery.
  - Prepared tree 2eafba13c9c6051ccdd37e44793ba4909758c6db was recovered as commit be67a1e343780449fabd9e3d236643363a0304ab.
  - The recovered diff changes player presentation, translations and navigation-aware tests; no backend or workflow change is present.
  - The recovered presentation referenced a missing world illustration; this candidate supplies a passive original SVG asset.
derived:
  - Recovered implementation is a useful coherent candidate, not verified completion.
unknown:
  - Final rendered quality and exact-head acceptance results remain unproven.
conflicts: []
first_failure:
  marker: RECOVERED_CANDIDATE_NOT_YET_VALIDATED
  evidence: Prior branch contained no published full redesign; recover the prepared tree, then test its actual runtime.
rejected_hypotheses:
  - The Markdown attachment is unreadable; its full contents were available and define this task.
changed_paths:
  - resources/views/**
  - public/css/**
  - public/images/oteryn-world.svg
  - scripts/acceptance/tests/**
validation:
  - command: node --check on temporary source recovery test; XML parse of new world SVG
    result: PASS
    evidence: Local syntax checks only; no Laravel/runtime readiness claim.
blockers: []
next_action: Retrieve the candidate acceptance artifact, inspect its exact tracked source and real Laravel screenshots, and repair the observed failures.
project_lane: oteryn-platform-core
admission_main_sha: 294e18909b8319695021011ccbeb1386cac32ced
policy_version: 2
task_kind: implementation
phase: implement
execution_mode: github-actions
execution_reason: local sandbox has no outbound DNS or Composer; existing isolated Actions runtime supplies real validation
context_pressure: medium
context_growth: stable
context_score: 7
estimate_confidence: medium
decomposition_decision: single
decomposition_reason: exposed tools provide no subagent action; one integrated public/player design owner
session_id: portal-redesign-20260907T090400Z
session_rotation_count: 1
heavy_validation_runs: 0
invocation_started_at: 2026-09-07T09:04:00Z
last_progress_at: 2026-09-07T09:13:00Z
ci_checks_for_current_head: 0
ci_check_generation: recovered-candidate
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
context_reconstruction_attempts: 1
stall_warnings: 0
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 2
  session_id: portal-redesign-20260907T090400Z
  session_started_at: 2026-09-07T09:04:00Z
  checkpointed_at: 2026-09-07T09:13:00Z
  last_progress_at: 2026-09-07T09:13:00Z
  phase: recovered-candidate-validation
  exact_head: be67a1e343780449fabd9e3d236643363a0304ab
  pull_request: 1298
  active_operation: publish coherent candidate with missing asset and isolated source transport
  external_run_ids: []
  operation_started_at: null
  wait_deadline_at: null
  check_generation: recovered-candidate
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: refresh live PR head and ensure no conflicting writer before publication or repairs
  next_action: Retrieve the candidate acceptance artifact, inspect its exact tracked source and real Laravel screenshots, and repair the observed failures.
```

Prior recovery generation 1 used branch head 41794acb6f112aa12d78820b35c2cede5bb2de05, two observations and one repair cycle. Its expired session is historical; no wait is being restarted. The new source transport exports only tracked Git trees from the candidate and fixed main, never live files, environment variables, credentials, sessions, databases or authentication traces. It is removed before readiness.

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: owner explicitly requires human visual review and forbids autonomous merge
source_branch_evidence: Issue 1297; PR 1298; complete owner brief
```

## Execution resources

Only the task's local sandbox and existing GitHub-hosted acceptance job are used. No workstation, Synology, self-hosted runner, staging, production or protected environment is touched. GitHub runner service containers are runner-managed ephemeral resources. The only temporary repository resource is the source-recovery test; remove it before final readiness. Screenshots use isolated synthetic fixtures and exclude secret-bearing enrollment/reset/recovery values.
