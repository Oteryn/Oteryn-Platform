---
task_id: OTERYN-20260806-merged-source-branch-lifecycle-decision
programme_id: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
project_lane: oteryn-platform-core
task_kind: architecture
implementation_authorized: false
issue: 586
status: completed
completed_at: 2026-08-06T06:51:12Z
pull_request: 653
merge: 2abfb961201f7f5d359c5b140dba68be492157be
implementation_handoff: 658
---

# OTERYN-20260806-merged-source-branch-lifecycle-decision — Completed

## Result

The repository owner explicitly selected Option A. ADR 0024 was accepted and merged through protected `main` in PR #653.

The durable policy keeps squash-only merging and automatic deletion of ordinary merged PR source branches. Long-lived exceptions require repository-rule or branch-protection enforcement plus durable owner, purpose and expiry/review metadata. Recovery uses PR history, immutable commit SHAs and bounded branch restoration.

`ARCH-DEC-0001` was removed from the active architecture decision backlog in the same bounded package. `ARCH-DEC-0002` and `ARCH-DEC-0003` remain active.

## Validation

Final synchronized head: `d47282cd427ec05091093901f28986fb79f28621`.

- CI run `31078661080`: PASS; documentation classification, aggregate protected `test` PASS, runtime tests NOT_APPLICABLE.
- Agent Governance run `31078661085`: PASS.
- Phase 7 Production-Like Validation run `31078661097`: PASS.
- Edge Security Emulation run `31078661127`: PASS.
- Game Auth Ticket Concurrency run `31078661144`: PASS.
- Platform DB Outage Validation run `31078661147`: PASS.
- Native protocol contract run `31078661120`: PASS.
- Native protocol contract audits run `31078661110`: PASS.
- Unresolved review threads: 0.
- Protected merge: `2abfb961201f7f5d359c5b140dba68be492157be`.

Two governance-schema failures were corrected before the final exact-head generation: missing checkpoint `owned_paths` and an unsupported validation-result token. Neither affected the accepted policy.

## Scope boundary

No branch was deleted and no repository setting, workflow, application, deployment or external repository was changed in the decision package.

Issue #658 owns deterministic inventory, dry-run classification, retention metadata, conservative cleanup and recovery proof. Issue #586 remains open until that implementation is complete.

## Ownership release

All six decision-documentation path claims are released. No active task or lease remains for this decision package.
