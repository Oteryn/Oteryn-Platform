# OTERYN-20260805 — Native protocol single-version producer — ARCHIVED

## Terminal state

```yaml
task_id: OTERYN-20260805-native-protocol-single-version-producer
coordination_id: OTS-20260804-native-protocol-selection
status: ARCHIVED
implementation_pr: 542
implementation_head: 45dbcc00432f5496785b1f6d532367f71d3faec7
implementation_merge: 93b122c29ba774c71ff6921cd5b4c5c57c089b61
base_included: 6953a96ab2ed7fe44d9eb0174abbbea0cff0f334
validation_intensity: HEIGHTENED
self_review: PASS
self_review_exact_head: 45dbcc00432f5496785b1f6d532367f71d3faec7
external_repair_audit: NOT_REQUIRED_UNDER_CURRENT_POLICY
historical_audits:
  - 756
  - 760
superseded_audit_issue: 766
production_activation: DISABLED_AND_UNAUTHORIZED
ownership: RELEASED
continuation_authority: none
```

## Delivered outcome

PR #542 migrated the disabled Oteryn native producer identity across Platform, World Registry, Game Gateway and Game Session v2 from transitional native `profile` identity to integer `native_protocol_version = 1`, schema revision `2`, and canonical schema SHA-256 `9c67f19525400fb9890d2a3541ceb6d02eb955061540ad39ca1c1d891c06eba9`.

The delivered contract preserves legacy no-offer behavior and Canary `profile` compatibility, keeps native advertisement and production activation disabled, and maintains fail-closed native selection/readiness with the exact canonical base capability set.

## Historical findings and remediation

- Audit #756 identified explicit-null identity-key bypass and non-exact native capability acceptance.
- Repair PR #758 resolved those findings and preserved Canary Game Session v2 profile identity.
- Repair PR #759 restored supported `actions/checkout@v5` references for Synology staging validation.
- Re-audit #760 identified permissive custom candidate unmarshalling and readiness identity-presence ambiguity.
- Repair PR #762 added strict internal JSON decoding, canonical-key allowlists, duplicate-key and trailing-value rejection, exact identity-key presence rules, case-variant alias rejection and focused regression coverage.
- PR #762 exact head `042d7aa4fb03f3a6a8b7c63aa79b2cbe0f0f9723` passed Agent Governance run `31153521548` and focused Game Gateway format/test/vet/build run `31153521116` before integration as `ef37422ddd6e862259f36cbf117fe2682e904ec7`.
- Temporary `.github/workflows/repair-760-validation.yml` was removed before final delivery.
- Audit Issue #766 targeted a superseded head and was closed `not_planned`, not represented as PASS, after protected-main governance adopted one-Issue/one-owner exact-head self-review with `external_repair_auditor_required: false`.

## Final exact-head evidence

Final implementation head `45dbcc00432f5496785b1f6d532367f71d3faec7` included protected main `6953a96ab2ed7fe44d9eb0174abbbea0cff0f334` with `behind_by = 0`; the effective delivery diff remained exactly 24 intended implementation, migration, contract, test, workflow and task paths.

Exact-head HEIGHTENED self-review was recorded on PR #542 with `result: PASS`, acceptance/full-diff/negative-path/rollback/compatibility/related-PR checks all true and zero findings. Unresolved review threads were zero before merge.

All 14 exact-head workflow runs completed successfully:

- Portal Exhaustive Audit `31158363987`: PASS.
- Agent Governance `31158363992`: PASS.
- Phase 7 Production-Like Validation `31158363994`: PASS.
- Native protocol contract `31158363885`: PASS.
- Platform DB Outage Validation `31158363915`: PASS.
- Game Gateway CI `31158363954`: PASS.
- Portal Acceptance Contract `31158363884`: PASS.
- Edge Security Emulation `31158364029`: PASS.
- Native protocol contract audits `31158364019`: PASS, including all five audit lanes.
- Build Synology Staging Images `31158363988`: PASS.
- Game Auth Ticket Concurrency `31158364000`: PASS.
- Acceptance E2E and Visual UX `31158363934`: PASS.
- CI `31158363966`: PASS.
- Deep System Validation `31158363928`: PASS, including formatting/static analysis, complete PHP regression/concurrency suites, dependency advisories, strict portal evidence contracts and the complete zero-retry browser matrix.

## Safety, compatibility and rollback

- Migrated Oteryn candidate rows remain disabled.
- Migration rollback restores the legacy native profile/schema identity while keeping rows disabled.
- Oteryn Game Session v2 identity is `native_protocol_version = 1` without `profile`; Canary compatibility retains `profile` without native version identity.
- Explicit-null foreign identity keys, unknown keys, duplicates, trailing JSON and non-canonical case variants fail closed.
- Native offered/required capabilities must equal the canonical base set exactly; optional native capabilities are empty.
- Production rollout, native advertisement, deployment and protected-environment actions were not authorized or performed.
- Repository rollback is a bounded revert of merge `93b122c29ba774c71ff6921cd5b4c5c57c089b61` while production activation remains disabled.

## Closeout

PR #542 is merged and terminal. The active task record is removed by this closeout, path/branch ownership and leases are released, and this archived record grants no continuation authority. The next remediation item in the owner-ordered queue is Issue #558; Issue #365 remains later in sequence.