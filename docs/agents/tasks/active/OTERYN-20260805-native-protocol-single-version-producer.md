---
task_id: OTERYN-20260805-native-protocol-single-version-producer
coordination_id: OTS-20260804-native-protocol-selection
status: validating
agent: ChatGPT
branch: feat/OTS-20260804-native-protocol-single-version-producer
repair_branch: repair/OTS-20260804-native-protocol-audit-760-current
prior_repair_branches:
  - repair/OTS-20260804-native-protocol-audit-756
  - repair/OTS-20260804-synology-checkout-action
base_branch: main
created: 2026-08-05T14:58:00+02:00
updated: 2026-08-07T08:24:00+02:00
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
- [ ] pass final exact-head CI and a fresh independent zero-finding audit after audit #760 remediation;
- [ ] merge PR #542, archive the task and release ownership.

## Ownership transfer

The older `OTERYN-20260723-native-auth-production-cutover` record described completed hardening plus external production approvals and retained a stale broad Gateway lease. This task supersedes only that stale lease for the authorized native-protocol migration. It does not authorize any production cutover or weaken the older task's external-approval requirements.

## Implementation validation baseline

- Original protected-main baseline: `1b737574851453e950fa485c26f1a322b8e8ddd2`.
- Producer was synchronized with protected main `842df4ac62bb6e928085f2bb328ff96259fa664e` before repair #762.
- Protected main advanced to `d581cc3b294de144e04a9ce373341b6d79af2269`; the producer now contains it as a merge parent via synchronization commit `b7b1d540211281ffae6354ed40a97e604ab238ac`.
- Current producer head before this checkpoint update is `b7b1d540211281ffae6354ed40a97e604ab238ac`.
- Validated implementation head before independent audit #756: `e97e950946ed255dfd399f890591337166c30406`.
- Finalizer run `31106026048`: PHP formatting, targeted Platform migration/producer tests, all Game Gateway Go tests and bounded-diff validation passed.
- Deep System Validation run `31107469030`: passed.
- Acceptance E2E and Visual UX run `31107470030`: passed.
- Native protocol contract audits run `31107468947`: all five lanes passed.
- Runtime activation remains disabled and unauthorized.

## Independent audit #756 and repair #758

Audit #756 recorded `FAIL_MATERIAL_FINDINGS_OPEN` against immutable head `e97e950946ed255dfd399f890591337166c30406`:

1. `OTERYN-AUD-756-01`: explicit JSON `"profile": null` could bypass the native no-profile boundary.
2. `OTERYN-AUD-756-02`: Gateway accepted additional and optional native capabilities despite the canonical exact-capability contract.

Repair review additionally found `OTERYN-REPAIR-758-01`: Game Session v2 dropped the selected Canary compatibility profile before readiness and session issuance.

Repair PR #758 was merged into the producer branch as `3d53a57f752f07d3eca07a05ed5e0f155ad33326`. It tracks identity-key presence independently from values, enforces the exact canonical native capability set, preserves mutually exclusive Oteryn/Canary Game Session v2 identity and adds regression coverage.

## Exact-head workflow repair #759

Exact-head run `31114628441` exposed unsupported `actions/checkout@v7` references in `.github/workflows/build-synology-staging-images.yml`. Repair PR #759 restored both references to supported `actions/checkout@v5` without weakening validation and was merged into the producer branch as `b7f594b118eb38403124dfd41289515cedf5e743`.

## Independent re-audit #760 and repair #762

Fresh read-only audit #760 independently confirmed the #756 and #758 repairs but recorded two additional material findings against immutable head `3d53a57f752f07d3eca07a05ed5e0f155ad33326`:

1. `OTERYN-AUD-760-01`: candidate custom `UnmarshalJSON` methods used internal permissive decoding, so outer `DisallowUnknownFields` did not reject unknown fields inside offer or Platform-policy candidates.
2. `OTERYN-AUD-760-02`: Game Session v2 readiness compared decoded identity values without tracking key presence, so native `"profile": null` or Canary `"native_protocol_version": null` could be accepted.

Repair PR #762:

- was reconstructed on producer head `52c08e5cd4fb3464e032cec878b526dc5f2cb228` and superseded closed unmerged PR #761;
- parses candidate and readiness JSON objects with duplicate-key detection, exact canonical-key allowlists, internal strict decoding and EOF validation;
- rejects case-variant identity aliases that Go's case-insensitive struct matching would otherwise accept;
- retains independent identity-key presence detection so explicit-null foreign identity keys remain forbidden;
- requires exactly `native_protocol_version` for Oteryn readiness and exactly `profile` for Canary compatibility readiness;
- adds regression coverage for explicit-null, absent, unknown, duplicate, trailing and case-variant identity/response cases;
- preserves Canary profile compatibility and the exact native capability contract;
- does not authorize production activation or any protected environment operation.

Focused repair validation on PR #762:

- predecessor head `4151953533df595e1bfff6771c7ffc401752a110`: Agent Governance run `31119531614` passed; focused validation run `31119531398` exposed only `gofmt` drift in `internal/session/client.go` after the GitHub Actions outage cleared;
- formatting-only commit `042d7aa4fb03f3a6a8b7c63aa79b2cbe0f0f9723` corrected the drift without semantic changes;
- exact-head Agent Governance run `31153521548`: PASS;
- exact-head Native audit 760 repair validation run `31153521116`: PASS, including formatting, `go test ./...`, `go vet ./...` and gateway build;
- PR #762 had zero review threads and was squash-merged as `ef37422ddd6e862259f36cbf117fe2682e904ec7`;
- temporary `.github/workflows/repair-760-validation.yml` was removed from the producer in `062fef619e9da175e842020a1b8f14e068616ad5` and must not appear in final main.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T08:24:00+02:00
head: resolve-live-from-pr-542-after-checkpoint-commit
branch: feat/OTS-20260804-native-protocol-single-version-producer
pr: 542
status: validating
context_routes:
  - agent-governance
  - api
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
  - docs/operations/OTERYN_NATIVE_PROTOCOL_PRODUCER.md
  - .github/workflows/native-protocol-contract-audits.yml
  - .github/workflows/build-synology-staging-images.yml
  - docs/agents/tasks/active/OTERYN-20260805-native-protocol-single-version-producer.md
proven:
  - Audit #756 findings were repaired by #758 and re-audit #760 independently confirmed those repairs.
  - Audit #760 findings OTERYN-AUD-760-01 and OTERYN-AUD-760-02 were repaired by #762.
  - Exact canonical JSON key allowlists also close the case-insensitive alias path discovered during repair self-review.
  - PR #762 exact head 042d7aa4fb03f3a6a8b7c63aa79b2cbe0f0f9723 passed Agent Governance and focused Go formatting/test/vet/build validation.
  - PR #762 was integrated into the producer as ef37422ddd6e862259f36cbf117fe2682e904ec7.
  - The temporary focused validation workflow was removed in 062fef619e9da175e842020a1b8f14e068616ad5.
  - Protected main d581cc3b294de144e04a9ce373341b6d79af2269 is included as a merge parent by b7b1d540211281ffae6354ed40a97e604ab238ac.
  - Production native advertisement and activation remain disabled and unauthorized.
derived:
  - Historical CI and audits are provenance only; final merge requires complete validation and a fresh independent audit on one unchanged final PR #542 head.
unknown:
  - Final PR #542 exact-head workflow outcome after this checkpoint update.
  - Outcome of the fresh independent read-only final audit.
conflicts:
  - none
first_failure:
  marker: audit-760-material-findings
  evidence: issue #760 and PR #542 record OTERYN-AUD-760-01 and OTERYN-AUD-760-02; both are now remediated by merged PR #762
rejected_hypotheses:
  - Waive unknown or case-variant candidate fields because outer decoders use DisallowUnknownFields.
  - Treat explicit-null readiness identity keys as equivalent to absent keys.
  - Merge PR #542 before exact-head CI and a fresh independent zero-finding audit.
  - Persist the temporary repair workflow in final main.
  - Broaden the canonical contract or authorize production activation.
changed_paths:
  - services/game-gateway/internal/gateway/types.go
  - services/game-gateway/internal/gateway/strict_json_regression_test.go
  - services/game-gateway/internal/session/client.go
  - services/game-gateway/internal/session/readiness_identity_regression_test.go
  - docs/agents/tasks/active/OTERYN-20260805-native-protocol-single-version-producer.md
validation:
  - command: independent audit #760 on 3d53a57f752f07d3eca07a05ed5e0f155ad33326
    result: FAIL
    evidence: two durable material findings requiring repair
  - command: PR #762 exact-head Agent Governance
    result: PASS
    evidence: run 31153521548 on 042d7aa4fb03f3a6a8b7c63aa79b2cbe0f0f9723
  - command: PR #762 focused Game Gateway validation
    result: PASS
    evidence: run 31153521116 on 042d7aa4fb03f3a6a8b7c63aa79b2cbe0f0f9723
blockers:
  - final PR #542 exact-head CI generation
  - fresh independent audit with PASS_ZERO_MATERIAL_FINDINGS
next_action: Freeze the new PR #542 head after this checkpoint commit, verify all exact-head workflows and zero review threads, then run one fresh independent read-only audit on that same immutable SHA.
```
