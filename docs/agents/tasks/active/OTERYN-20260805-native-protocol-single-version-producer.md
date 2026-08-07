---
task_id: OTERYN-20260805-native-protocol-single-version-producer
coordination_id: OTS-20260804-native-protocol-selection
status: validating
agent: ChatGPT
project_lane: oteryn-platform-core
task_kind: implementation
phase: validate
branch: feat/OTS-20260804-native-protocol-single-version-producer
base_branch: main
created: 2026-08-05T14:58:00+02:00
updated: 2026-08-07T09:25:00+02:00
risk: high
execution_mode: github-only
implementation_authorized: true
production_activation_authorized: false
decomposition_decision: phased
context_pressure: medium
context_growth: stable
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

Migrate the disabled Platform and Game Gateway producer from the transitional native profile model to exactly `family = oteryn`, `native_protocol_version = 1`, schema revision `2`, and schema SHA-256 `9c67f19525400fb9890d2a3541ceb6d02eb955061540ad39ca1c1d891c06eba9`, while preserving Canary compatibility and keeping production activation disabled.

## Exact dependencies

- Platform canonical correction merge: `c0b8703d326a04b43ae8e06f6192b0cb91c859b7`.
- Otheryn correspondence merge: `92bd106a92a8c3622de85099e2152db5b8cf2bde`.
- Rust correspondence merge: `c923ad8a1dff17b4933a6110931b0823cec2c590`.

## Acceptance

- [x] remove the Oteryn native `profile` identity and use required integer `native_protocol_version = 1`;
- [x] migrate existing disabled rows safely and provide reversible rollback;
- [x] require exact family/version/transport/schema/hash/capabilities in selection and readiness;
- [x] reject explicit-null foreign identity keys, unknown keys, duplicate keys, trailing JSON and non-canonical/case-variant aliases;
- [x] preserve mutually exclusive Canary `profile` compatibility identity;
- [x] keep every native candidate disabled and production activation unauthorized;
- [x] cover parser, downgrade, readiness, migration, rollback and producer integration behavior with focused regression tests;
- [ ] record exact-final-head full-diff self-review `PASS` with HEIGHTENED protocol/migration evidence;
- [ ] pass all applicable exact-head CI and real integration/E2E validation selected by the repository for the final head;
- [ ] keep zero unresolved material findings, requested changes, review threads, ownership conflicts, `UNKNOWN` or `CONFLICT` merge blockers;
- [ ] merge PR #542, verify the resulting state, archive this task and release ownership.

## Ownership and production boundary

This task supersedes only the stale Gateway lease from `OTERYN-20260723-native-auth-production-cutover` for the authorized repository migration. It does not authorize production cutover, production deployment, protected-environment approval, production secrets, or live account/session/data mutation.

## Audit and repair provenance

Historical independent audits remain useful falsification evidence but are not a mandatory different-agent merge handoff under the current protected-main remediation policy (`AGENTS.override.md` revision 3.0 and `REMEDIATION_AUDIT_RISK_GATE.md` version 2).

- Audit #756 on `e97e950946ed255dfd399f890591337166c30406` found explicit-null identity bypass and non-exact capabilities.
- Repair #758 fixed those findings and restored Canary Game Session v2 profile preservation.
- Repair #759 restored supported Synology `actions/checkout@v5` references.
- Re-audit #760 on `3d53a57f752f07d3eca07a05ed5e0f155ad33326` found permissive custom candidate unmarshalling and readiness identity-presence ambiguity.
- Repair PR #762 added duplicate-key detection, exact canonical-key allowlists, strict internal decoding, EOF validation, exact identity-key presence rules and case-variant alias rejection.
- PR #762 exact head `042d7aa4fb03f3a6a8b7c63aa79b2cbe0f0f9723` passed Agent Governance run `31153521548` and focused Go format/test/vet/build run `31153521116`, then integrated as `ef37422ddd6e862259f36cbf117fe2682e904ec7`.
- Temporary `.github/workflows/repair-760-validation.yml` was removed in `062fef619e9da175e842020a1b8f14e068616ad5` and is not part of the intended final diff.

## Current-main reconciliation

- The former repository-wide `league/commonmark` advisory blocker was repaired on protected main by PR #768 (`league/commonmark` 2.9.0); no advisory suppression was introduced.
- Protected main later incorporated the owner-directed one-Issue/one-owner remediation model and reduced Actions fan-out in PR #764, plus lifecycle closeouts and Wiki fixture repair PR #751.
- Producer synchronization currently includes protected main `503eb774bb485703b1f2212857ef5c1375c8ebbb` as a parent through synchronization commit `e748aa4c674448a9bb3f07284a4393c09e8f25ea` before this documentation checkpoint commit.
- The effective PR #542 diff before this checkpoint remained exactly 24 intended producer implementation/migration/contract/test/workflow/task paths and `behind_by = 0`.

## Validation gate

```yaml
validation_gate:
  version: 2
  intensity: HEIGHTENED
  classified_by: ChatGPT
  classified_at: 2026-08-07T09:25:00+02:00
  risk: high
  triggers:
    - public protocol identity and downgrade boundary
    - database migration and rollback
    - Game Session readiness compatibility
    - CI workflow changes
  unknown_or_conflict: []
  rationale: Final delivery changes a public protocol producer identity, migration and readiness boundary, so focused negative-path evidence plus the full applicable exact-head suite is required.
  self_review:
    result: PENDING
    exact_head: resolve-live-after-this-checkpoint-commit
    evidence:
      - Historical #756/#760 findings are repaired and preserved as provenance.
      - Final full-diff review must be repeated on the live immutable head after this checkpoint commit.
  external_repair_auditor_required: false
```

## Context checkpoint

```yaml
checkpoint_version: 2
phase: validate
session_id: chatgpt-20260807-0914-producer-closeout
session_role: implementer-validator
execution_mode: github-only
status: validating
project_lane: oteryn-platform-core
task_kind: implementation
context_pressure: medium
context_growth: stable
decomposition_decision: phased
branch: feat/OTS-20260804-native-protocol-single-version-producer
pr: 542
head: resolve-live-after-this-checkpoint-commit
current_main_before_checkpoint: 503eb774bb485703b1f2212857ef5c1375c8ebbb
last_completed_step: synchronized current protected main into producer and reclassified final gate under current one-owner remediation policy
proven:
  - PR #762 fixes audit #760 findings and passed focused exact-head Go validation before integration.
  - Exact canonical JSON-key allowlists close the case-insensitive alias path discovered during repair self-review.
  - CommonMark 2.9.0 security repair is on protected main and therefore included by current synchronization.
  - Current protected-main remediation policy does not require a second implementation auditor to approve the repair.
  - Production activation remains disabled and unauthorized.
derived:
  - Historical CI on superseded runtime heads is supporting evidence only; final exact-head validation must be re-established after current-main synchronization.
unknown:
  - Terminal outcome of applicable workflows emitted for the new final PR #542 head.
conflicts: []
first_failure:
  marker: historical-audit-760-material-findings
  evidence: OTERYN-AUD-760-01 and OTERYN-AUD-760-02, both repaired by PR #762
rejected_hypotheses:
  - suppress CommonMark advisories instead of consuming the protected-main dependency repair
  - retain a mandatory different-agent repair-audit handoff after protected-main governance explicitly removed it
  - merge without exact-head HEIGHTENED validation and self-review
  - persist the temporary repair workflow
  - broaden the native contract or authorize production activation
validation:
  - command: PR #762 exact-head Agent Governance
    result: PASS
    evidence: run 31153521548 on 042d7aa4fb03f3a6a8b7c63aa79b2cbe0f0f9723
  - command: PR #762 focused Game Gateway validation
    result: PASS
    evidence: run 31153521116 on 042d7aa4fb03f3a6a8b7c63aa79b2cbe0f0f9723
blockers:
  - final exact-head self-review and HEIGHTENED validation not yet recorded for the post-synchronization head
next_action: Resolve the live PR #542 head, inspect its full diff and review state, record exact-head self-review, then verify all applicable exact-head CI/E2E and merge only if every gate remains green.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 1
  session_id: chatgpt-20260807-0914-producer-closeout
  session_started_at: 2026-08-07T09:14:00+02:00
  checkpointed_at: 2026-08-07T09:25:00+02:00
  last_progress_at: 2026-08-07T09:25:00+02:00
  phase: final-validation
  exact_head: resolve-live-after-this-checkpoint-commit
  pull_request: 542
  active_operation: exact-head self-review then final CI
  external_run_ids: []
  operation_started_at: null
  wait_deadline_at: null
  check_generation: final-ready
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: PR #542 head remains owned, mergeable and synchronized with current protected main
  next_action: Fetch PR #542 live head and full diff, record exact-head self-review, then observe the aggregate final required-check state.
```
