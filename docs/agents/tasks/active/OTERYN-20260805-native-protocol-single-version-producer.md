---
task_id: OTERYN-20260805-native-protocol-single-version-producer
coordination_id: OTS-20260804-native-protocol-selection
status: ready
agent: ChatGPT
branch: feat/OTS-20260804-native-protocol-single-version-producer
base_branch: main
created: 2026-08-05T14:58:00+02:00
updated: 2026-08-06T15:43:00+02:00
risk: high
execution_mode: github-only
implementation_authorized: true
production_activation_authorized: false
supersedes_task: OTERYN-20260723-native-auth-production-cutover
owned_paths:
  - app/GameAuth/Worlds/**
  - app/Http/Controllers/GameAuth/GameLoginContextController.php
  - database/migrations/**native_protocol_identity**
  - services/game-gateway/**
  - tests/Feature/GameAuth/**
  - docs/contracts/GAME_SESSION_CANARY_CONTRACT.md
  - docs/contracts/WORLD_REGISTRY_CONTRACT.md
  - docs/operations/OTERYN_NATIVE_PROTOCOL_PRODUCER.md
  - .github/workflows/native-protocol-contract-audits.yml
  - docs/agents/tasks/active/OTERYN-20260805-native-protocol-single-version-producer.md
shared_path_lease: []
---

# OTERYN-20260805 native protocol single-version producer

## Goal

Migrate the disabled Platform and Game Gateway producer from the transitional native profile model to exactly `family = oteryn`, `native_protocol_version = 1`, schema revision `2`, and schema SHA-256 `9c67f19525400fb9890d2a3541ceb6d02eb955061540ad39ca1c1d891c06eba9`.

## Exact dependencies

- Platform canonical correction merge: `c0b8703d326a04b43ae8e06f6192b0cb91c859b7`.
- Otheryn correspondence merge: `92bd106a92a8c3622de85099e2152db5b8cf2bde`.
- Rust correspondence merge: `c923ad8a1dff17b4933a6110931b0823cec2c590`.

## Acceptance

- [x] remove the native profile field from API, DB, World Registry, readiness, Game Session v2 and tests;
- [x] add required integer `native_protocol_version = 1` without alias or placeholder;
- [x] migrate existing disabled rows safely and provide reversible rollback;
- [x] require exact family/version/transport/schema/hash/capabilities in selection and readiness;
- [x] preserve legacy no-offer behavior and Canary compatibility mechanisms unchanged;
- [x] keep every native candidate disabled and production activation unauthorized;
- [x] pass parser, replay, downgrade, readiness, data migration, rollback and producer E2E tests;
- [ ] pass exact-head CI and five independent audits;
- [ ] merge, archive and release ownership.

## Ownership transfer

The older `OTERYN-20260723-native-auth-production-cutover` record described completed hardening plus external production approvals and retained a stale broad Gateway lease. This task supersedes only that stale lease for the authorized native-protocol migration. It does not authorize any production cutover or weaken the older task's external-approval requirements.

## Implementation validation

- Included protected main: `1b737574851453e950fa485c26f1a322b8e8ddd2`.
- Validated implementation merge head: `80c8b8035a33caadc2cbbb250676ce5afc64ae48`.
- Finalizer run `31106026048`: PHP formatting, targeted Platform migration/producer tests, all Game Gateway Go tests and bounded-diff validation passed.
- Current-main comparison at the implementation merge head: `behind_by = 0`; the bounded producer diff contains declared product, contract, migration, test, workflow and task paths with no transient repair workflow or diagnostic evidence.
- Deep System Validation on `1a20eacdc0bf9dcc50600ab696da512e3c99a564` exposed PHPStan typing defects only in the new migration regression; commit `13e70d6b943d66d8342f6232a0293efc29601655` repaired them without changing runtime behavior.
- Native protocol contract audit run `31106841527` showed four audit lanes passing and exposed an obsolete documentation-only restriction in Audit 1. Commit `ed087f0abd95cf548f81792155c8127089ff1b0e` replaced it with a governed producer-runtime allowlist while retaining strict rejection of unrelated runtime paths.
- Runtime activation remains disabled and unauthorized.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-06T15:43:00+02:00
head: resolve-live-from-pr-542-before-audit
branch: feat/OTS-20260804-native-protocol-single-version-producer
pr: 542
status: ready
context_routes:
  - agent-governance
  - api
  - database
  - game-gateway
  - protocol
  - security
  - testing
  - workflows
owned_paths:
  - app/GameAuth/Worlds/**
  - app/Http/Controllers/GameAuth/GameLoginContextController.php
  - database/migrations/**native_protocol_identity**
  - services/game-gateway/**
  - tests/Feature/GameAuth/**
  - docs/contracts/GAME_SESSION_CANARY_CONTRACT.md
  - docs/contracts/WORLD_REGISTRY_CONTRACT.md
  - .github/workflows/native-protocol-contract-audits.yml
  - docs/agents/tasks/active/OTERYN-20260805-native-protocol-single-version-producer.md
proven:
  - Protected main 1b737574851453e950fa485c26f1a322b8e8ddd2 is included by the implementation merge history.
  - Finalizer run 31106026048 passed Platform migration and producer tests, all Game Gateway tests, formatting and bounded-diff validation.
  - The bounded diff contains only declared implementation, migration, contract, test, workflow and task paths and no transient repair artifacts.
  - Native advertisement and production activation remain disabled and unauthorized.
  - Commit 13e70d6b943d66d8342f6232a0293efc29601655 makes the migration regression statically typed.
  - Commit ed087f0abd95cf548f81792155c8127089ff1b0e permits only the governed native producer runtime boundary in Audit 1 and continues to reject unrelated runtime paths.
derived:
  - The corrected implementation and audit workflow are ready for one final exact-head CI generation and fresh independent validation.
unknown:
  - Final exact-head repository workflow outcomes after this checkpoint commit.
  - Outcome of the required fresh independent read-only validator review.
conflicts:
  - none
first_failure:
  marker: exact-head-validation-repairs-before-final-audit
  evidence: runs 31106219881 and 31106841527 identified static-test typing and an obsolete documentation-only audit boundary respectively
rejected_hypotheses:
  - Enable native advertisement or production activation as part of this producer migration.
  - Preserve a native profile alias or placeholder alongside native_protocol_version.
  - Weaken Audit 1 to permit arbitrary application, service, database, configuration, route or test paths.
  - Change Canary compatibility identity from its existing profile mechanism.
changed_paths:
  - declared native producer implementation, migration, contract, test, audit-workflow and task paths relative to included main
validation:
  - command: finalizer run 31106026048
    result: PASS
    evidence: PHP format, targeted Platform tests, all Game Gateway tests and bounded-diff validation succeeded
  - command: Deep System Validation run 31106219881
    result: FAIL
    evidence: PHPStan identified mixed typing in the new migration regression and the test was repaired on a later head
  - command: Native protocol contract audits run 31106841527
    result: FAIL
    evidence: four lanes passed and Audit 1 rejected legitimate governed producer paths under its obsolete documentation-only rule
  - command: exact-head workflows after governed Audit 1 repair and checkpoint update
    result: NOT_RUN
    evidence: this commit triggers the final repository workflow generation
blockers:
  - none; final exact-head CI and fresh independent validation are ready for execution
next_action: Verify the final exact-head workflows, then rotate PR 542 to a fresh independent read-only validator on the immutable branch tip.
```
