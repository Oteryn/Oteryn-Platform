---
task_id: OTERYN-20260805-native-protocol-single-version-producer
coordination_id: OTS-20260804-native-protocol-selection
status: ready
agent: ChatGPT
branch: feat/OTS-20260804-native-protocol-single-version-producer
base_branch: main
created: 2026-08-05T14:58:00+02:00
updated: 2026-08-06T15:31:00+02:00
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
- Runtime activation remains disabled and unauthorized.

## Checkpoint

```yaml
phase: audit
exact_base: 1b737574851453e950fa485c26f1a322b8e8ddd2
implementation_head: 80c8b8035a33caadc2cbbb250676ce5afc64ae48
final_candidate_head: resolve_live
branch: feat/OTS-20260804-native-protocol-single-version-producer
production_enabled: false
exact_head_ci: pending
independent_audits_required: 5
independent_audits_completed: 0
next_action: run exact-head CI and rotate to five independent read-only audits before merge
```
