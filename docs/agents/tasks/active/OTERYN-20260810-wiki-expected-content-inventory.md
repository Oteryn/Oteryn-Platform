---
task_id: OTERYN-20260810-wiki-expected-content-inventory
mode: implementation
issue: 488
branch: repair/issue-488
status: validating
programme: OTERYN_PLATFORM_REMEDIATION
portal_programme: OTERYN_PORTAL_COMPLETION
---

# OTERYN-20260810-wiki-expected-content-inventory

## Goal

Close Issue #488 by making the reviewed Wiki launch corpus independently machine-verifiable, binding that inventory to the canonical Portal Exhaustive Audit, and reconciling every remaining Wiki/EditorialMedia failure, portability, accessibility, locale and overflow finding with exact executable evidence or machine-proven non-applicability.

## Feature scope

```yaml
feature_scope:
  type: content_integrity_and_acceptance_evidence
  complete_user_facing_feature: false
  backend_required: true
  frontend_required: false
  runtime_required: true
  integration_required: true
  e2e_required: true
```

## Ownership

```yaml
project_lane: oteryn-platform-core
owned_paths:
  - app/Wiki/Content/WikiExpectedContentInventory.php
  - app/Wiki/Content/WikiExpectedContentValidator.php
  - app/Console/Commands/ValidateWikiExpectedContent.php
  - app/Console/Commands/InstallWikiLaunchContent.php
  - docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json
  - docs/testing/PORTAL_STRICTNESS_EVIDENCE.json
  - tests/Unit/Wiki/WikiExpectedContentInventoryTest.php
  - tests/Feature/Wiki/WikiLaunchContentCommandTest.php
  - tools/audit/portal_exhaustive_strictness.py
  - tools/audit/test_portal_exhaustive_strictness.py
  - tools/audit/test_portal_exhaustive_audit.py
  - .github/workflows/portal-exhaustive-audit.yml
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/playwright.wiki-reconciliation.config.mjs
  - scripts/acceptance/playwright.editorial-media.config.mjs
  - scripts/acceptance/seed-browser-wiki-reconciliation.php
  - scripts/acceptance/seed-browser-editorial-media.php
  - scripts/acceptance/tests/wiki-strictness-acceptance.spec.mjs
  - scripts/acceptance/tests/editorial-media-strictness-acceptance.spec.mjs
  - scripts/acceptance/coverage/portal-evidence-dimensions/content.json
  - docs/agents/tasks/active/OTERYN-20260810-wiki-expected-content-inventory.md
restricted_paths:
  - app/Wiki/Models/**
  - app/Wiki/Domain/**
  - app/Wiki/Actions/**
  - database/**
  - routes/**
  - resources/views/**
  - deploy/**
  - repository environments
  - secrets and variables
  - production systems
  - external repositories
coordination_key: wiki:expected-content-inventory
```

## Acceptance inventory

- [x] Independent inventory pins catalog version, exact reviewed catalog Git blob, four categories, thirteen EN/PL article identities, metadata and exact per-article provenance.
- [x] Canonical `docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json` is strictly bound to the PHP inventory before runtime validation or installation.
- [x] CommonMark AST link validation catches inline and reference-style external/unexpected links.
- [x] Install validation occurs before publisher lookup or writes; read-only `wiki:launch-content:validate --json` exposes machine PASS/FAIL.
- [x] Canonical Portal Exhaustive Audit validates the Wiki runtime inventory before generation and waits fail-closed for exact-head Wiki Reconciliation, Editorial Media Acceptance and full Acceptance E2E success; run IDs/conclusions are retained in the audit artifact.
- [x] Strictness gaps are represented explicitly: real 404/419/500→recovery probes for admin Wiki and EditorialMedia, existing public Wiki 404/429/503→recovery evidence, explicit accessibility/overflow markers, and machine-verified N/A only for read-only CSRF, no-throttle rate-limit, and unlocalized trusted operator UI rules.
- [x] Failure injection uses only disposable acceptance MariaDB table renames through existing Laravel fixture commands; no tracked application/view file is renamed or modified.
- [x] Standard portability profile executes the real EditorialMedia lifecycle in bounded Firefox and WebKit as well as Chromium.
- [ ] Final exact-head Portal Exhaustive Audit contains no material Wiki/EditorialMedia findings and records runtime-inventory PASS plus exact-head executed workflow evidence on the final refreshed head.
- [ ] Final exact-head CI, Agent Governance, Wiki Reconciliation, Editorial Media Acceptance and full Acceptance E2E all pass on the final refreshed head.
- [ ] Final whole-diff self-review has no open material finding and all PR review threads are terminal.
- [x] No application route/view/schema/deployment/production/credential/external-repository behavior is changed.

## Validation gate

```yaml
validation_gate:
  version: 2
  intensity: HEIGHTENED
  classified_by: chatgpt-portal-closeout-20260810-2352
  classified_at: 2026-08-10T23:52:00+02:00
  risk: high
  triggers:
    - Issue 488 proven HIGH Wiki completeness gap
    - bilingual published Wiki launch content
    - machine-readable audit completion contract
    - canonical audit runtime-validator and executed-workflow coupling
    - privileged Wiki and EditorialMedia failure/CSRF/recovery acceptance
    - cross-browser EditorialMedia mutation lifecycle
    - historical intermittent Wiki publication/media defects
  unknown_or_conflict: []
  rationale: Completion requires independent content truth plus actually executed exact-head acceptance; static markers or standalone JSON cannot self-approve closure.
  self_review:
    result: PENDING
    exact_head: none
    evidence: []
```

## Context checkpoint

```yaml
checkpoint_version: 2
updated_at: 2026-08-11T08:37:00+02:00
invocation_started_at: 2026-08-11T08:37:00+02:00
last_progress_at: 2026-08-11T08:37:00+02:00
head_before_checkpoint: 3f5f872d1d747096fbf8c8c3f28aa3061b3a4f2b
protected_main: 06ec89521e412c2252f1a10224955b90180e8c35
branch: repair/issue-488
pr: 972
status: validating
phase: final_exact_head_validation
session_id: portal-closeout-20260811-0837
session_role: owner_validator
execution_mode: github
execution_reason: GitHub connector and Actions provide the required repository and exact-head validation operations.
context_pressure: high
context_growth: stable
decomposition_decision: phased
validation_level: full
ci_checks_for_current_head: 1
ci_check_generation: stale_base_merge_ref
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 1
stall_warnings: 0
proven:
  - Issue #488 is the canonical remediation/evidence owner and PR #972 is the single authoritative delivery PR.
  - User authorization and deterministic claim plus bounded scope extensions are durable in Issue #488 comments.
  - Exact inventory chain is machine JSON -> PHP inventory -> exact reviewed catalog Git blob -> actual categories/articles/locales/provenance/CommonMark links.
  - Prior review findings for unpinned bilingual copy, provenance, reference-style links and missing canonical inventory were repaired with focused validation.
  - Canonical Portal Exhaustive Audit now executes `wiki:launch-content:validate --json`, requires exact-head success of Wiki Reconciliation Acceptance, Editorial Media Acceptance and Acceptance E2E and Visual UX, and persists those run identities before audit generation.
  - Wiki/EditorialMedia strictness failure injection now uses disposable acceptance database-table fixtures and does not rename tracked source files.
  - On PR head 3f5f872d1d747096fbf8c8c3f28aa3061b3a4f2b, CI, Wiki Reconciliation, Editorial Media Acceptance, Acceptance E2E and Visual UX, Portal Exhaustive Audit, Portal Acceptance Contract, Deep System Validation and the other emitted runtime gates passed.
  - Agent Governance run 31442749339 failed only because its stale PR merge base still contained terminal Synology hygiene task OTERYN-20260811-container-resource-hygiene with PR #973 represented as active and next_action containing merge.
  - Protected main is now 06ec89521e412c2252f1a10224955b90180e8c35 after archive PR #978; current main active-task inventory no longer contains OTERYN-20260811-container-resource-hygiene, so the proven liveness failure root cause has been removed from the trusted base.
derived:
  - A new PR head is required so GitHub builds a fresh merge ref against current main and re-runs exact-head governance without the stale terminal task.
  - The prior Agent Governance failure is base-state drift, not evidence of a Wiki runtime or inventory defect.
unknown:
  - final exact-head gate results after this current-base refresh checkpoint creates a new PR head
  - future post-launch Wiki expansion beyond this versioned launch inventory
conflicts: []
first_failure:
  marker: stale-base-task-liveness
  evidence: Agent Governance run 31442749339 job 93630676124; terminal_pr_stale_next_action and terminal_pr_active_task for OTERYN-20260811-container-resource-hygiene / PR #973.
rejected_hypotheses:
  - Re-running the old Agent Governance event would prove current-base compatibility; the old pull_request event is bound to its stale merge-ref generation.
  - The Wiki implementation caused the governance failure; the failed liveness entries identify an unrelated terminal operations task.
  - Static source markers prove browser execution; canonical audit requires exact-head workflow success.
  - Tracked Blade renames are acceptable fault injection; they were replaced by disposable database fixture operations.
validation:
  - command: CI on 3f5f872d1d747096fbf8c8c3f28aa3061b3a4f2b
    result: PASS
    evidence: run 31442749334
  - command: Wiki Reconciliation Acceptance on 3f5f872d1d747096fbf8c8c3f28aa3061b3a4f2b
    result: PASS
    evidence: run 31442749380
  - command: Editorial Media Acceptance on 3f5f872d1d747096fbf8c8c3f28aa3061b3a4f2b
    result: PASS
    evidence: run 31442749372
  - command: Acceptance E2E and Visual UX on 3f5f872d1d747096fbf8c8c3f28aa3061b3a4f2b
    result: PASS
    evidence: run 31442749332
  - command: Portal Exhaustive Audit on 3f5f872d1d747096fbf8c8c3f28aa3061b3a4f2b
    result: PASS
    evidence: run 31442749376
  - command: Agent Governance on 3f5f872d1d747096fbf8c8c3f28aa3061b3a4f2b with stale base merge ref
    result: FAIL_BASE_DRIFT
    evidence: run 31442749339 / job 93630676124
blockers: []
next_action: Use this material checkpoint commit to create a fresh PR synchronize generation against protected main 06ec89521e412c2252f1a10224955b90180e8c35; then verify all required exact-head gates, review threads and self-review before merge and terminal Issue/task closeout.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 1
  session_id: portal-closeout-20260811-0837
  session_started_at: 2026-08-11T08:37:00+02:00
  checkpointed_at: 2026-08-11T08:37:00+02:00
  last_progress_at: 2026-08-11T08:37:00+02:00
  phase: final_exact_head_validation
  exact_head: 3f5f872d1d747096fbf8c8c3f28aa3061b3a4f2b
  pull_request: 972
  active_operation: none
  external_run_ids:
    - 31442749339
  operation_started_at: null
  wait_deadline_at: null
  check_generation: current_base_refresh_pending
  checks_used: 0
  status: ready
  safe_to_resume: true
  resume_condition: PR #972 has a new head and fresh required-check generation against current protected main.
  next_action: Inspect one aggregate exact-head workflow snapshot for the new PR head; diagnose only failed gates, otherwise complete review/self-review and merge closeout.
```
