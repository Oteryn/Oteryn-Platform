---
task_id: OTERYN-20260815-client-distribution-platform
repository: blakinio/Oteryn-Platform
mode: implementation
issue: 1039
programme: OTERYN_PORTAL_COMPLETION
status: validating
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/TERMINAL_ONLY_COMMUNICATION.md
  - docs/agents/CI_WORKFLOW_LIFECYCLE.md
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_SCOPE.json
  - docs/architecture/CLIENT_DISTRIBUTION_ARCHITECTURE.md
  - docs/architecture/adr/0035-first-party-client-distribution-and-updater-trust-boundary.md
---

# OTERYN-20260815-client-distribution-platform

## Goal

Implement the complete Platform-only vertical slice of ADR 0035 for first-party client distribution without taking custody of private signing keys, mutating an external updater repository, writing another repository, deploying, or activating production.

## Acceptance criteria

- [x] Existing browser Download Center publication and immutable approved artifact references remain supported independently from updater state.
- [x] Updater-enabled releases receive opaque stable identities and positive channel-scoped monotonic integer sequences; display-version text is never security ordering.
- [x] Updater targets are exact platform + architecture targets with no fallback identity.
- [x] Immutable policy revisions model minimum support, optional/recommended/required update mode, release revocations, exact target revocations and explicit older-sequence rollback authorization.
- [x] Withdrawal is separate from revocation and preserves browser/release history.
- [x] Browser publication, approved policy/signing intent, reconciled protected-integration generation state and Platform-active updater state are separate facts.
- [x] Platform stores and validates a bounded PUBLIC generation projection only through an internal protected-integration boundary after required TUF verification outside Laravel; no ordinary web-admin route can import or activate it.
- [x] No private signing key or signing secret is accepted or persisted, including through direct internal action input.
- [x] Policy operation identity and protected-generation identity are idempotent; identity reuse with different intent fails closed.
- [x] Import rejects stale/replayed metadata, expired metadata, channel/policy mismatch, policy-target mismatch, duplicate/extra/missing platform targets and exact target/length/digest mismatch.
- [x] Metadata rollback is fail-closed while freshness-only Timestamp advancement can preserve unchanged Root/Targets/Snapshot versions.
- [x] Activation rejects stale, expired, superseded, non-latest-policy and withdrawn-current generations.
- [x] Browser/admin presentation describes administrator-supplied SHA-256 truthfully and never presents it as publisher-signature verification.
- [x] Admin diagnostics are behind the existing auth + confirmed MFA + `downloads.manage` boundary and explicitly state that Platform reconciliation is not TUF client verification or production activation.
- [x] Unit and Feature coverage include sequence ordering, rollback, withdrawal, revocation, idempotency, replay/staleness, freshness-only metadata advancement, mismatch handling, private-key rejection, absent web-admin generation mutation routes and browser/updater divergence.
- [x] Existing Downloads Playwright acceptance exercises browser publication plus updater enable → policy approval → acceptance-only protected-integration simulation → Platform-active read-only diagnostics and localized presentation; it explicitly does not claim cryptographic TUF signing or production activation.
- [x] Existing `downloads-acceptance.yml` is extended rather than creating a new workflow; workflow budget remains 53.
- [x] The isolated acceptance workflow proves migration rollback and re-apply before browser lifecycle execution.
- [ ] Exact reconstructed PR head passes required GitHub CI and Downloads Acceptance.
- [ ] Whole-diff self-review and review-thread hygiene are terminal with no unresolved material finding.
- [ ] Issue #1039, source branch and task lifecycle are terminal after implementation merge.
- [ ] Real protected signer/repository integration and real first-party updater/client cross-repository E2E remain separately authorized future gates and are not claimed by this task.

## Ownership

```yaml
owned_paths:
  - app/Downloads/**
  - app/Http/Controllers/Downloads/**
  - app/Http/Requests/Downloads/**
  - database/migrations/2026_08_16_100000_add_client_updater_distribution_tables.php
  - lang/en/downloads.php
  - lang/pl/downloads.php
  - resources/views/downloads/**
  - resources/views/admin/downloads/**
  - routes/modules/downloads.php
  - tests/Unit/Downloads/**
  - tests/Feature/Downloads/**
  - scripts/acceptance/seed-downloads-state.php
  - scripts/acceptance/tests/downloads-lifecycle-acceptance.spec.mjs
  - .github/workflows/downloads-acceptance.yml
  - docs/agents/tasks/active/OTERYN-20260815-client-distribution-platform.md
  - docs/agents/tasks/archive/OTERYN-20260815-client-distribution-platform.md
modules:
  - Downloads
  - client distribution Platform trust boundary
dependencies:
  - ADR 0035 accepted on protected main
  - existing immutable Download Center boundary
  - existing Downloads Acceptance workflow
blockers: []
```

## Architecture boundary

- Platform persists browser metadata, updater policy intent, exact bounded public generation projection received from the protected integration and Platform-active reconciliation state.
- Platform does not persist private updater signing keys, expose an ordinary web-admin generation import/activation route, or perform/claim TUF signature verification.
- The internal generation reconciliation action is a receiving boundary for a future separately protected release-publishing integration after that integration has verified the required TUF signatures/repository coherence; the real integration itself is outside this task.
- Exact policy association is checked by canonical policy target path + byte length + SHA-256 and exact target tuple/path/length/digest reconciliation.
- The administrator-supplied artifact SHA-256 remains supplied metadata; matching it to the bounded public target projection proves association consistency, not independent publisher authenticity.
- External signer/repository writes, production activation and real first-party updater/client cross-repository E2E are outside this task's authority.

## Reconstruction evidence

```yaml
protected_main: 9336cd1f240196908a84cdea124992300bede59c
protected_main_tree: 5d1921a69091816b98e2c22b7e0621d1cd82b1b0
previous_pr_head: 72a0aea11c4b610a2b7e561f39a5fa68c0ad05cc
previous_pr_changed_files:
  - docs/agents/tasks/active/OTERYN-20260815-client-distribution-platform.md
previous_material_implementation: none
overlap_resolution: reconstruct exact implementation tree from current protected main; do not replay stale implementation history
workflow_policy: reuse_or_extend
workflow_budget: 53
new_workflows: 0
clean_material_commit: da514f3eda178e904a6fc481616a737d1b12d8c4
first_published_reconstruction_head: 4a2fa4e3b19571e257d5462f6c6b58ef841a4ad0
pre_authority_boundary_repair_head: 9af2db04d29ba726c3f171ae975388e70f906414
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-16T11:49:00+02:00
head: 9af2db04d29ba726c3f171ae975388e70f906414
status: validating
phase: authority_boundary_repaired_ready_for_exact_head_validation
branch: feat/issue-1039-client-distribution-platform
pr: 1073
context_routes:
  - agent-governance
  - security
  - testing
  - api
owned_paths:
  - app/Downloads/**
  - app/Http/Controllers/Downloads/**
  - app/Http/Requests/Downloads/**
  - database/migrations/2026_08_16_100000_add_client_updater_distribution_tables.php
  - lang/en/downloads.php
  - lang/pl/downloads.php
  - resources/views/downloads/**
  - resources/views/admin/downloads/**
  - routes/modules/downloads.php
  - tests/Unit/Downloads/**
  - tests/Feature/Downloads/**
  - scripts/acceptance/seed-downloads-state.php
  - scripts/acceptance/tests/downloads-lifecycle-acceptance.spec.mjs
  - .github/workflows/downloads-acceptance.yml
repository_mutation_authorization: PROVEN
external_repository_mutation_authorization: ABSENT_AND_NOT_REQUIRED
production_activation_authorization: ABSENT_AND_NOT_REQUIRED
owner_funded_agent_authorization: ABSENT_AND_NOT_USED
proven:
  - protected main was re-read at 9336cd1f240196908a84cdea124992300bede59c before reconstruction
  - PR #1073 contained only this task record before reconstruction
  - ADR 0035 and CLIENT_DISTRIBUTION_ARCHITECTURE are accepted authority for this slice
  - the existing browser Download Center and Downloads Acceptance workflow were extended rather than replaced
  - no new workflow was created and the registered workflow budget remains 53
  - all implementation writes are confined to Oteryn-Platform
  - first published reconstruction head was 4a2fa4e3b19571e257d5462f6c6b58ef841a4ad0 with merge base exactly protected main and behind_by zero
  - Agent Governance run 31939121881 isolated the original first_failure scalar defect
  - Agent Governance run 31939204176 proved the remaining checkpoint contract omissions exactly after all earlier governance tests passed
  - whole-diff self-review identified that ordinary downloads.manage web routes could incorrectly mint reconciled/Platform-active generation state without an independently authenticated protected integration
  - the repaired tree removes those web mutation routes/controller methods/forms, hardens the internal importer itself against unknown/secret-shaped fields, and changes browser acceptance to an acceptance-environment-only internal integration simulation
derived:
  - ordinary Download Center administration may approve Platform policy intent but cannot itself establish a trusted signed-generation fact
  - real TUF signature/repository verification belongs to the separately protected integration and first-party updater, not Laravel web administration
  - a fresh exact head is required after the authority-boundary repair
unknown:
  - terminal conclusions of exact-head CI/Downloads Acceptance for the repaired tree until GitHub executes them
  - real protected signer/repository integration behavior because that separate authority and implementation are outside this task
  - real updater/client cross-repository behavior because that separate authority is not granted
conflicts: []
first_failure:
  marker: self-review-web-admin-signed-generation-authority
  evidence: pre-repair head 9af2db04d29ba726c3f171ae975388e70f906414 exposed POST generation import and activation through the ordinary downloads.manage web boundary, which was inconsistent with ADR 0035 separation of web administration from protected TUF authority
rejected_hypotheses:
  - lexical display version can safely order updates
  - browser is_current can stand in for updater-active state
  - administrator-supplied SHA-256 proves publisher authenticity
  - Laravel should own private TUF signing keys
  - ordinary downloads.manage web administration should be able to mint reconciled signed-generation state
  - TUF Targets and Snapshot versions must increase on every Timestamp freshness refresh
  - a new task-specific workflow is needed
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260815-client-distribution-platform.md
  - app/Downloads/**
  - app/Http/Controllers/Downloads/**
  - app/Http/Requests/Downloads/**
  - database/migrations/2026_08_16_100000_add_client_updater_distribution_tables.php
  - lang/en/downloads.php
  - lang/pl/downloads.php
  - resources/views/downloads/**
  - resources/views/admin/downloads/**
  - routes/modules/downloads.php
  - tests/Unit/Downloads/**
  - tests/Feature/Downloads/**
  - scripts/acceptance/seed-downloads-state.php
  - scripts/acceptance/tests/downloads-lifecycle-acceptance.spec.mjs
  - .github/workflows/downloads-acceptance.yml
validation:
  - command: Agent Governance run 31939121881 on head 4a2fa4e3b19571e257d5462f6c6b58ef841a4ad0
    result: FAIL
    evidence: checkpoint-validation rejected first_failure scalar encoding; all earlier governance tests in that job passed
  - command: Agent Governance run 31939204176 on head 6bbbb06578a1686d2fb35f3e8869e7e22b37e7ca
    result: FAIL
    evidence: checkpoint-validation required changed_paths, derived, head and checkpoint_version 1; all earlier governance tests in that job passed
  - command: repaired authority-boundary exact-head GitHub Actions
    result: NOT_RUN
    evidence: the repaired tree has not yet been published to the branch at this checkpoint
blockers: []
next_action: Publish the repaired exact PR head, capture all GitHub Actions run ids, inspect and repair the first material failure if any, then complete whole-diff self-review and terminal merge/lifecycle closeout only after all gates are green.
```

## Recovery checkpoint

```yaml
checkpoint_type: RECOVERY
captured_at: 2026-08-16T11:49:00+02:00
current_state: Platform-only implementation is reconstructed and the web-admin/protected-integration authority boundary has been repaired before merge
last_completed_step: removed ordinary web mutation authority for protected generation state and moved acceptance reconciliation to an acceptance-only internal integration simulation
active_operation: publish repaired tree and execute exact-head GitHub validation lanes
external_run_ids:
  - 31939121881
  - 31939204176
expected_success_marker: repaired exact PR head has required CI plus Downloads Acceptance success and zero unresolved material review findings
expected_failure_marker: first failing required job or review finding on the repaired exact PR head
wait_deadline: none; same-PR self-review remains available while Actions execute
next_step_on_success: whole-diff self-review, mark PR ready, exact expected-head squash merge, Issue #1039 close, lifecycle archive
next_step_on_failure: inspect first relevant failing job log, repair root cause on the same branch, rerun exact-head validation
parallel_work_allowed: true
parallel_work_scope: same-PR self-review only until entry task is terminal
context_pressure: medium
rotation_reason: null
```