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
checkpoint_version: 1
updated_at: 2026-08-11T08:49:00+02:00
head: 0c67ca77957877d578df5d33a60dd9b4d78ff153
branch: repair/issue-488
pr: 972
status: validating
context_routes:
  - agent-governance
  - architecture
  - security
  - public-web
  - testing
  - content
  - acceptance
  - audit
  - ci-workflow
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
changed_paths:
  - .github/workflows/portal-exhaustive-audit.yml
  - app/Console/Commands/InstallWikiLaunchContent.php
  - app/Console/Commands/ValidateWikiExpectedContent.php
  - app/Wiki/Content/WikiExpectedContentInventory.php
  - app/Wiki/Content/WikiExpectedContentValidator.php
  - docs/agents/tasks/active/OTERYN-20260810-wiki-expected-content-inventory.md
  - docs/testing/PORTAL_STRICTNESS_EVIDENCE.json
  - docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json
  - scripts/acceptance/coverage/portal-evidence-dimensions/content.json
  - scripts/acceptance/playwright.config.mjs
  - scripts/acceptance/playwright.editorial-media.config.mjs
  - scripts/acceptance/playwright.wiki-reconciliation.config.mjs
  - scripts/acceptance/seed-browser-editorial-media.php
  - scripts/acceptance/seed-browser-wiki-reconciliation.php
  - scripts/acceptance/tests/editorial-media-strictness-acceptance.spec.mjs
  - scripts/acceptance/tests/wiki-strictness-acceptance.spec.mjs
  - tests/Feature/Wiki/WikiLaunchContentCommandTest.php
  - tests/Unit/Wiki/WikiExpectedContentInventoryTest.php
  - tools/audit/portal_exhaustive_strictness.py
  - tools/audit/test_portal_exhaustive_audit.py
  - tools/audit/test_portal_exhaustive_strictness.py
proven:
  - Issue #488 is the canonical remediation/evidence owner and PR #972 is the single authoritative delivery PR.
  - User authorization and deterministic claim plus bounded scope extensions are durable in Issue #488 comments.
  - Exact inventory chain is machine JSON -> PHP inventory -> exact reviewed catalog Git blob -> actual categories/articles/locales/provenance/CommonMark links.
  - Prior review findings for unpinned bilingual copy, provenance, reference-style links and missing canonical inventory were repaired with focused validation.
  - Canonical Portal Exhaustive Audit executes `wiki:launch-content:validate --json`, requires exact-head success of Wiki Reconciliation Acceptance, Editorial Media Acceptance and Acceptance E2E and Visual UX, and persists those run identities before audit generation.
  - Wiki/EditorialMedia strictness failure injection uses disposable acceptance database-table fixtures and does not rename tracked source files.
  - On PR head 3f5f872d1d747096fbf8c8c3f28aa3061b3a4f2b, every emitted runtime/evidence gate passed except Agent Governance.
  - Agent Governance run 31442749339 failed on a stale PR merge generation because terminal Synology hygiene task OTERYN-20260811-container-resource-hygiene / PR #973 was still represented as active.
  - Protected main 06ec89521e412c2252f1a10224955b90180e8c35 archived that Synology task through #978.
  - The first closeout-refresh commit 0c67ca77957877d578df5d33a60dd9b4d78ff153 created a fresh PR generation; its CI run 31466298999 then failed before routing because this checkpoint rewrite violated the repository checkpoint schema, not because of application/runtime behavior.
derived:
  - This schema-repair commit is required before exact-head validation can continue.
unknown:
  - final exact-head gate results after the checkpoint schema repair
  - future post-launch Wiki expansion beyond this versioned launch inventory
conflicts: []
first_failure:
  marker: checkpoint-schema-contract
  evidence: CI run 31466298999 / job 93699781976 reported missing changed_paths, context_routes, head, owned_paths; checkpoint_version 2 unsupported; FAIL_BASE_DRIFT unsupported.
rejected_hypotheses:
  - The fresh CI failure represents a Wiki defect; classify-changes stopped at the task checkpoint schema before application routing/tests.
  - Re-running the old Agent Governance event proves current-base compatibility; the old pull_request event is bound to its stale generation.
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
    result: FAIL
    evidence: run 31442749339 / job 93630676124; unrelated terminal task liveness drift
  - command: CI classify-changes on 0c67ca77957877d578df5d33a60dd9b4d78ff153
    result: FAIL
    evidence: run 31466298999 / job 93699781976; checkpoint schema contract only
blockers: []
next_action: Validate this repaired checkpoint through the new exact-head CI generation; then verify Agent Governance, Wiki Reconciliation, Editorial Media Acceptance, Acceptance E2E, Portal Exhaustive Audit, review threads and whole-diff self-review before merge/closeout.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 2
  session_id: portal-closeout-20260811-0837
  session_started_at: 2026-08-11T08:37:00+02:00
  checkpointed_at: 2026-08-11T08:49:00+02:00
  last_progress_at: 2026-08-11T08:49:00+02:00
  phase: final_exact_head_validation
  exact_head: 0c67ca77957877d578df5d33a60dd9b4d78ff153
  pull_request: 972
  active_operation: none
  external_run_ids:
    - 31466298999
    - 31466299057
    - 31466299000
    - 31466299003
    - 31466298952
    - 31466299015
  operation_started_at: null
  wait_deadline_at: null
  check_generation: checkpoint_schema_repair_pending
  checks_used: 0
  status: ready
  safe_to_resume: true
  resume_condition: PR #972 has the schema-repair head and its fresh required-check generation exists.
  next_action: Inspect the new CI classify-changes result first; if green, aggregate exact-head gates and diagnose only new failures.
```
