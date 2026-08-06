---
task_id: OTERYN-20260805-native-protocol-single-version-producer
coordination_id: OTS-20260804-native-protocol-selection
status: implementing
agent: ChatGPT
branch: feat/OTS-20260804-native-protocol-single-version-producer
repair_branch: repair/OTS-20260804-native-protocol-audit-756
base_branch: main
created: 2026-08-05T14:58:00+02:00
updated: 2026-08-06T17:00:00+02:00
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
- [ ] pass exact-head CI and a fresh independent zero-finding audit after audit #756 remediation;
- [ ] merge, archive and release ownership.

## Ownership transfer

The older `OTERYN-20260723-native-auth-production-cutover` record described completed hardening plus external production approvals and retained a stale broad Gateway lease. This task supersedes only that stale lease for the authorized native-protocol migration. It does not authorize any production cutover or weaken the older task's external-approval requirements.

## Implementation validation

- Included protected main: `1b737574851453e950fa485c26f1a322b8e8ddd2`.
- Validated implementation merge head before independent audit: `e97e950946ed255dfd399f890591337166c30406`.
- Finalizer run `31106026048`: PHP formatting, targeted Platform migration/producer tests, all Game Gateway Go tests and bounded-diff validation passed.
- Deep System Validation run `31107469030`: passed.
- Acceptance E2E and Visual UX run `31107470030`: passed.
- Native protocol contract audits run `31107468947`: all five lanes passed.
- Exact audited head had 14 terminal successful workflow runs and zero unresolved review threads.
- Runtime activation remains disabled and unauthorized.

## Independent audit #756

Audit #756 recorded `FAIL_MATERIAL_FINDINGS_OPEN` against immutable head `e97e950946ed255dfd399f890591337166c30406` with two material findings:

1. `OTERYN-AUD-756-01`: explicit JSON `"profile": null` was treated as an absent key by the custom Go unmarshaler and could bypass the native no-profile boundary.
2. `OTERYN-AUD-756-02`: Gateway accepted additional and optional native capabilities despite the canonical exact-capability contract and stricter Platform producer.

The repair branch:

- detects identity-key presence independently from decoded values for both offer and policy candidates;
- rejects native `profile` and compatibility-family `native_protocol_version` even when explicitly `null`;
- requires the native offered and required capability lists to equal the canonical list exactly;
- requires native optional capabilities to be empty;
- adds raw-JSON and capability-set negative regression coverage;
- aligns service fixtures with the canonical native set.

The audited implementation SHA remains unchanged. A new exact head and fresh independent audit are required after the repair merges.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-06T17:00:00+02:00
head: resolve-live-from-repair-branch
branch: repair/OTS-20260804-native-protocol-audit-756
producer_branch: feat/OTS-20260804-native-protocol-single-version-producer
producer_pr: 542
audit_issue: 756
status: implementing
context_routes:
  - agent-governance
  - api
  - game-gateway
  - protocol
  - security
  - testing
  - workflows
owned_paths:
  - services/game-gateway/**
  - docs/agents/tasks/active/OTERYN-20260805-native-protocol-single-version-producer.md
proven:
  - Protected main 1b737574851453e950fa485c26f1a322b8e8ddd2 is included by the producer history.
  - Audit 756 independently validated behind_by zero, 21 declared producer paths, 14 successful exact-head workflows and zero pre-existing review threads.
  - Audit 756 recorded two material fail-closed inconsistencies and blocked merge of PR 542.
  - The repair branch does not mutate the audited SHA or authorize production activation.
  - Identity-key presence is now detected independently of JSON null decoding.
  - Native offered, required and optional capabilities are now checked against the exact canonical contract.
derived:
  - The repair must merge into the producer branch before any new exact-head audit can be valid.
unknown:
  - Repair branch CI outcome.
  - New producer exact-head workflow outcome after repair integration.
  - Outcome of the required fresh independent read-only audit.
conflicts:
  - none
first_failure:
  marker: audit-756-material-findings
  evidence: issue 756 and PR 542 comments record OTERYN-AUD-756-01 and OTERYN-AUD-756-02
rejected_hypotheses:
  - Record PASS_ZERO_MATERIAL_FINDINGS despite material findings.
  - Merge PR 542 before remediation and fresh independent validation.
  - Change the canonical contract to permit optional native capabilities within the repair.
changed_paths:
  - services/game-gateway/internal/gateway/types.go
  - services/game-gateway/internal/gateway/native_protocol.go
  - services/game-gateway/internal/gateway/native_protocol_test.go
  - services/game-gateway/internal/gateway/service_test.go
  - docs/agents/tasks/active/OTERYN-20260805-native-protocol-single-version-producer.md
validation:
  - command: independent audit 756 on e97e950946ed255dfd399f890591337166c30406
    result: FAIL_MATERIAL_FINDINGS_OPEN
    evidence: two durable material findings on issue 756 and PR 542
  - command: repair branch review and CI
    result: PENDING
    evidence: repair changes prepared for review against the immutable audited head
blockers:
  - repair branch validation and integration
  - new exact-head CI generation
  - fresh independent audit with zero open material findings
next_action: Open the repair PR against the producer branch, validate and merge it, then create a fresh exact-head audit issue.
```
