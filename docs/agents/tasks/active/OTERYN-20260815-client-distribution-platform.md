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
- [x] Browser publication, approved policy/signing intent, reconciled signed-generation state and Platform-active updater state are separate facts.
- [x] Platform stores and validates a bounded PUBLIC signed-generation projection; no private signing key or signing secret is accepted or persisted.
- [x] Policy operation identity and signed-generation identity are idempotent; identity reuse with different intent fails closed.
- [x] Import rejects stale/replayed metadata, expired metadata, channel/policy mismatch, policy-target mismatch, duplicate/extra/missing platform targets and exact target/length/digest mismatch.
- [x] Activation rejects stale, expired, superseded, non-latest-policy and withdrawn-current generations.
- [x] Browser/admin presentation describes administrator-supplied SHA-256 truthfully and never presents it as publisher-signature verification.
- [x] Admin diagnostics are behind the existing auth + confirmed MFA + `downloads.manage` boundary and explicitly state that Platform reconciliation is not TUF client verification or production activation.
- [x] Unit and Feature coverage include sequence ordering, rollback, withdrawal, revocation, idempotency, replay/staleness, mismatch handling, private-key rejection and browser/updater divergence.
- [x] Existing Downloads Playwright acceptance exercises browser publication plus updater enable → policy approval → public signed-generation reconciliation → Platform activation and localized presentation.
- [x] Existing `downloads-acceptance.yml` is extended rather than creating a new workflow; workflow budget remains 53.
- [x] The isolated acceptance workflow proves migration rollback and re-apply before browser lifecycle execution.
- [ ] Exact reconstructed PR head passes required GitHub CI and Downloads Acceptance.
- [ ] Whole-diff self-review and review-thread hygiene are terminal with no unresolved material finding.
- [ ] Issue #1039, source branch and task lifecycle are terminal after implementation merge.
- [ ] Real first-party updater/client cross-repository E2E remains a separately authorized future gate and is not claimed by this task.

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

- Platform persists browser metadata, updater policy intent, exact public signed-generation projection and Platform-active reconciliation state.
- Platform does not persist private updater signing keys and does not perform or claim client-side TUF signature verification.
- Exact signed policy association is proven by canonical policy target path + byte length + SHA-256 and exact target tuple/path/length/digest reconciliation.
- The administrator-supplied artifact SHA-256 remains described as supplied metadata; matching it to reconciled public signed target metadata proves association consistency, not independent publisher authenticity.
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
```

## Context checkpoint

```yaml
checkpoint_version: 2
updated_at: 2026-08-16T11:18:27+02:00
status: validating
phase: reconstructed_platform_vertical_slice_ready_for_exact_head_validation
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
unknown:
  - exact CI run ids and terminal conclusions until reconstructed head is pushed
  - real updater/client cross-repository behavior because that separate authority is not granted
conflicts: []
first_failure: null
rejected_hypotheses:
  - lexical display version can safely order updates
  - browser is_current can stand in for updater-active state
  - administrator-supplied SHA-256 proves publisher authenticity
  - Laravel should own private TUF signing keys
  - a new task-specific workflow is needed
validation:
  - command: exact reconstructed GitHub Actions
    result: PENDING
    evidence: run ids are populated after branch reconstruction is published
blockers: []
next_action: Publish the reconstructed exact-main implementation head, capture exact GitHub run ids, repair any first relevant failure, complete exact-head self-review, then squash-merge #1073 and execute lifecycle-only task archival.
```

## Recovery checkpoint

```yaml
checkpoint_type: RECOVERY
captured_at: 2026-08-16T11:18:27+02:00
current_state: coherent implementation tree prepared from exact protected main; no external mutation or production action performed
last_completed_step: Platform-only data/model/action/request/admin/public/test/acceptance implementation assembled against main 9336cd1f240196908a84cdea124992300bede59c
active_operation: publish reconstructed PR head and execute existing exact-head GitHub validation lanes
external_run_ids: []
expected_success_marker: exact PR head has required CI plus Downloads Acceptance success and zero unresolved material review findings
expected_failure_marker: first failing required job or review finding on exact PR head
wait_deadline: none; independent programme audit work remains available while Actions execute
next_step_on_success: whole-diff self-review, mark PR ready, exact expected-head squash merge, Issue #1039 close, lifecycle archive
next_step_on_failure: inspect first relevant failing job log, repair root cause on the same branch, rerun exact-head validation
parallel_work_allowed: true
parallel_work_scope: Platform-side audit of #338 only; no Canary/server reads or writes and no merge while producer proof is absent
context_pressure: medium
rotation_reason: null
```