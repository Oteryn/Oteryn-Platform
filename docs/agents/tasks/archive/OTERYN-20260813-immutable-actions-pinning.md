---
task_id: OTERYN-20260813-immutable-actions-pinning
mode: implementation
branch: ci/immutable-actions-pinning-1008
status: completed
project_lane: oteryn-platform-core
issue: 1008
pr: 1022
merge_commit: 1422b931a2ccc0e3200100e40e63763f7e0cb883
---

# Immutable GitHub Actions dependency pinning

## Goal

Close Issue #1008 by pinning external GitHub Actions dependencies to reviewed immutable commit SHAs, preserving Dependabot support, and enforcing the policy with a deterministic validator.

## Acceptance

- [x] Inventory and classify external and local `uses:` references.
- [x] Pin external actions to reviewed full 40-character SHAs without downgrades.
- [x] Preserve semantic-version comments and Dependabot github-actions support.
- [x] Add fail-closed validator and positive/negative fixtures.
- [x] Wire validation into unconditional CI pre-classification.
- [x] Obtain terminal exact-head CI, perform final review, squash merge, close #1008, and archive this record.

## Final closeout

```yaml
checkpoint_version: 1
updated_at: 2026-08-13T19:47:00+02:00
candidate_head: b3c078ac46178988615839d7ffa209a499951383
merged_main: 1422b931a2ccc0e3200100e40e63763f7e0cb883
branch: ci/immutable-actions-pinning-1008
pr: 1022
issue: 1008
status: completed
proven:
  - Current main before merge was 81316d95f849972a694039b0e65245c4db3d8272 and was the exact merge-base of the final candidate; behind_by was 0.
  - Fresh final compare against current main showed only immutable Actions pinning, deterministic validator/fixtures, required CI test adaptations and this task record; build-synology-staging-images.yml differed from current main by exactly six uses pins.
  - No pull-request review threads were open on the final candidate.
  - Exact-head CI run 31722411720 completed successfully and its unconditional classification job passed workflow inventory, immutable GitHub Actions references, task checkpoint validation and changed-path classification.
  - Deep System Validation run 31722411617 completed successfully on the same exact candidate head.
  - Build Synology Staging Images run 31722411751 completed successfully on the same exact candidate head.
  - Phase 7 Production-Like Validation run 31722411700 completed successfully on the same exact candidate head.
  - Platform DB Outage Validation run 31722411645, Game Auth Ticket Concurrency run 31722411672 and Edge Security Emulation run 31722411699 completed successfully on the same exact candidate head.
  - All pull-request workflow runs returned for candidate head b3c078ac46178988615839d7ffa209a499951383 were terminal success at final verification.
  - PR #1022 was squash-merged with expected_head_sha protection; merge commit is 1422b931a2ccc0e3200100e40e63763f7e0cb883.
  - Issue #1008 is closed with state_reason completed.
  - No production deployment, protected-environment mutation or secret change was performed as part of this task.
unknown: []
conflicts: []
blockers: []
```

## Safety

Repository CI hardening only. No deployment, protected-environment change, or secret mutation was performed as part of this task.
