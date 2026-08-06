---
task_id: OTERYN-20260805-native-protocol-single-version-producer
coordination_id: OTS-20260804-native-protocol-selection
status: implementing
agent: ChatGPT
branch: feat/OTS-20260804-native-protocol-single-version-producer
repair_branch: repair/OTS-20260804-synology-checkout-action
base_branch: main
created: 2026-08-05T14:58:00+02:00
updated: 2026-08-06T17:20:00+02:00
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
  - .github/workflows/build-synology-staging-images.yml
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

Repair review of the same immutable producer head also found `OTERYN-REPAIR-758-01`: Game Session v2 removed `profile` from every selected family, so a selected Canary compatibility tuple lost its required profile identity before readiness and session issuance.

Repair PR #758:

- detects identity-key presence independently from decoded values for both offer and policy candidates;
- rejects native `profile` and compatibility-family `native_protocol_version` even when explicitly `null`;
- requires the native offered and required capability lists to equal the canonical list exactly;
- requires native optional capabilities to be empty;
- preserves mutually exclusive family identity in Game Session v2: native version for Oteryn and profile for Canary compatibility;
- rejects contradictory family identity locally before a session-service request;
- adds raw-JSON, capability-set and Canary Game Session v2 regression coverage;
- aligns service fixtures with the canonical native set.

PR #758 was squash-merged into the producer branch as `3d53a57f752f07d3eca07a05ed5e0f155ad33326`. The original audited SHA remains immutable.

## Exact-head CI blocker discovered after remediation

The first exact-head validation generation on `3d53a57f752f07d3eca07a05ed5e0f155ad33326` exposed a pre-existing invalid action reference in `.github/workflows/build-synology-staging-images.yml`:

- run `31114628441`, attempt 1, failed before checkout because GitHub could not resolve action download information;
- attempt 2 deterministically reported `Unable to resolve action actions/checkout@v7`;
- all three image build jobs completed successfully;
- only `Validate Synology deployment package` could not start.

Repair PR #759 changes both invalid checkout references to the repository-supported `actions/checkout@v5` without changing deployment validation or image-build behavior. A new exact producer head and complete workflow generation are required after integration.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-06T17:20:00+02:00
head: 15d66fe2f4e23ee27c6b42621c0d77e6ef42cd93
branch: repair/OTS-20260804-synology-checkout-action
pr: 759
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
  - .github/workflows/build-synology-staging-images.yml
  - docs/agents/tasks/active/OTERYN-20260805-native-protocol-single-version-producer.md
proven:
  - Protected main 1b737574851453e950fa485c26f1a322b8e8ddd2 is included by the producer history.
  - Audit 756 independently recorded two material fail-closed inconsistencies and blocked merge of PR 542.
  - Repair PR 758 remediated audit findings and preserved Canary Game Session v2 profile identity.
  - Exact head 3d53a57f752f07d3eca07a05ed5e0f155ad33326 remained behind_by zero with zero review threads.
  - Game Gateway CI, native contract, governance, security, production-like and other completed workflows passed on that head.
  - Synology validation attempts failed before executing repository validation because actions/checkout@v7 does not exist.
  - Both Synology workflow checkout references are now pinned to actions/checkout@v5.
derived:
  - The workflow repair must merge into the producer branch and regenerate the exact-head workflow set before independent audit.
unknown:
  - PR 759 governance result.
  - Complete exact-head workflow outcome after PR 759 integration.
  - Outcome of the required fresh independent read-only audit.
conflicts:
  - none
first_failure:
  marker: synology-checkout-action-unresolvable
  evidence: run 31114628441 attempts 1 and 2 failed at Set up job; attempt 2 named actions/checkout@v7 explicitly
rejected_hypotheses:
  - Treat the infrastructure/setup failure as successful validation.
  - Merge PR 542 while a required exact-head workflow is red.
  - Remove or bypass the Synology validation job.
changed_paths:
  - .github/workflows/build-synology-staging-images.yml
  - docs/agents/tasks/active/OTERYN-20260805-native-protocol-single-version-producer.md
validation:
  - command: Build Synology Staging Images run 31114628441 attempt 1
    result: FAIL
    evidence: action download resolution failed before repository steps
  - command: Build Synology Staging Images run 31114628441 attempt 2
    result: FAIL
    evidence: actions/checkout@v7 could not be resolved
  - command: PR 759 governance and integration validation
    result: NOT_RUN
    evidence: minimal workflow repair prepared against the producer branch
blockers:
  - PR 759 governance and integration
  - new producer exact-head CI generation
  - fresh independent audit with zero open material findings
next_action: Validate and merge PR 759 into the producer branch, then run a complete exact-head validation generation before creating a fresh audit issue.
```
