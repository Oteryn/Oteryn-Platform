---
task_id: OTERYN-20260805-native-protocol-single-version-producer
coordination_id: OTS-20260804-native-protocol-selection
status: ready
agent: ChatGPT
branch: feat/OTS-20260804-native-protocol-single-version-producer
base_branch: main
created: 2026-08-05T14:58:00+02:00
updated: 2026-08-06T15:38:00+02:00
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
- Current-main comparison at the implementation merge head: `behind_by = 0`; exactly 20 declared product, contract, migration, test and task paths; no transient repair workflow or diagnostic evidence remains.
- Deep System Validation on `1a20eacdc0bf9dcc50600ab696da512e3c99a564` exposed only PHPStan typing defects in the new migration regression; commit `13e70d6b943d66d8342f6232a0293efc29601655` repaired them without changing runtime behavior.
- Runtime activation remains disabled and unauthorized.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-06T15:38:00+02:00
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
owned_paths:
  - app/GameAuth/Worlds/**
  - app/Http/Controllers/GameAuth/GameLoginContextController.php
  - database/migrations/**native_protocol_identity**
  - services/game-gateway/**
  - tests/Feature/GameAuth/**
  - docs/contracts/GAME_SESSION_CANARY_CONTRACT.md
  - docs/contracts/WORLD_REGISTRY_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20260805-native-protocol-single-version-producer.md
proven:
  - Protected main 1b737574851453e950fa485c26f1a322b8e8ddd2 is included by the implementation merge history.
  - Finalizer run 31106026048 passed Platform migration and producer tests, all Game Gateway tests, formatting and bounded-diff validation.
  - The final bounded implementation diff contains exactly 20 declared paths and no transient repair workflow or diagnostic first-failure artifact.
  - Native advertisement and production activation remain disabled and unauthorized.
  - Commit 13e70d6b943d66d8342f6232a0293efc29601655 makes the migration regression statically typed after the first exact-head Deep System Validation failure.
derived:
  - The implementation is ready for a new exact-head CI generation and independent audit rotation after this checkpoint-only commit.
unknown:
  - Final exact-head repository workflow outcomes after this checkpoint commit.
  - Outcomes of the five required independent read-only audits.
conflicts:
  - none
first_failure:
  marker: deep-system-static-analysis-on-1a20eacdc0bf9dcc50600ab696da512e3c99a564
  evidence: run 31106219881 reported mixed migration and database-value typing in NativeProtocolIdentityMigrationTest.php
rejected_hypotheses:
  - Enable native advertisement or production activation as part of this producer migration.
  - Preserve a native profile alias or placeholder alongside native_protocol_version.
  - Change Canary compatibility identity from its existing profile mechanism.
changed_paths:
  - 20 declared implementation, migration, contract, test and task paths relative to included main
validation:
  - command: finalizer run 31106026048
    result: PASS
    evidence: PHP format, targeted Platform tests, all Game Gateway tests and bounded-diff validation succeeded
  - command: Deep System Validation run 31106219881 on 1a20eacdc0bf9dcc50600ab696da512e3c99a564
    result: FAIL
    evidence: PHPStan identified mixed typing only in the new migration regression test
  - command: exact-head repository workflows after static typing repair and checkpoint normalization
    result: NOT_RUN
    evidence: a new workflow generation is triggered by this checkpoint commit
blockers:
  - none; exact-head CI and five independent audits are ready for execution
next_action: Verify all exact-head workflows, then rotate PR 542 to five independent read-only audits on the immutable branch tip.
```
