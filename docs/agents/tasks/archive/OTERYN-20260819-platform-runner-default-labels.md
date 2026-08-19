---
task_id: OTERYN-20260819-platform-runner-default-labels
status: completed
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: true
policy_version: 2
phase: closeout
session_id: chatgpt-20260819-platform-runner-default-labels
session_role: implementer
execution_mode: chat-github
execution_reason: security correction of the Synology staging-runner label boundary
validation_level: focused
issue: 1155
pull_request: 1176
branch: none
base_sha: 3d07ac95c69250eea15095976e08e083ae66abcb
merged_sha: 14aa7b312cb5dabc13f2ba42c54bb46e79ccd167
owned_paths: []
---

# Platform staging-runner label boundary correction — terminal closeout

## Result

PR #1176 restored the intended repository-scoped custom-label boundary for the Docker-socket-capable Synology staging runner by reinstating `--no-default-labels` while retaining `--labels "$RUNNER_LABELS"` / `oteryn-staging`.

This explicitly supersedes the transient behavior merged by PR #1174. PR #1175 was closed unmerged after the contradiction was discovered. Generic `self-hosted` eligibility is not part of the terminal runner contract.

## Evidence

Exact candidate head: `8192b33d590e314e0e43d0d225d31024c32a035b`.

PASS:
- Agent Governance `32305821631`;
- CI `32305821473`;
- Game Auth Ticket Concurrency `32305821514`;
- Platform DB Outage Validation `32305821471`;
- Phase 7 Production-Like Validation `32305821566`;
- Synology Rollback Contract `32305821495`;
- Synology Production Target Preflight `32305821572`;
- Build Synology Staging Images `32305821573`;
- Edge Security Emulation `32305821480` after one infrastructure-only retry caused by Ubuntu/Azure package-mirror instability.

Full-diff self-review: PASS; zero unresolved review threads. Squash merge: `14aa7b312cb5dabc13f2ba42c54bb46e79ccd167`.

Live runner mutation: `NOT_APPLICABLE`; live runner state already used custom-label-only registration. PR #1164 remains the no-side-effect post-transfer attachment verification authority.

## Context checkpoint

```yaml
checkpoint_version: 1
head: 14aa7b312cb5dabc13f2ba42c54bb46e79ccd167
branch: none
pr: 1176
status: completed
context_routes:
  - agent-governance
  - deployment-operations
  - testing
owned_paths: []
proven:
  - PR 1176 merged as 14aa7b312cb5dabc13f2ba42c54bb46e79ccd167.
  - The canonical runner entrypoint suppresses GitHub default labels and retains the repository-scoped oteryn-staging label.
  - Required and applicable exact-head checks passed before merge.
  - PR 1174 behavior is superseded by the security correction.
derived: []
unknown: []
conflicts: []
first_failure:
  marker: incorrect-runner-label-assumption
  evidence: PR 1164 explicitly documents custom-label-only routing as a Docker-socket isolation boundary.
rejected_hypotheses:
  - Generic self-hosted eligibility is required for PR 1164.
validation:
  - command: exact-head PR 1176 CI and full-diff self-review
    result: PASS
    evidence: head 8192b33d590e314e0e43d0d225d31024c32a035b
  - command: live runner mutation
    result: NOT_APPLICABLE
    evidence: existing live registration already matches the intended isolated-label model
blockers: []
next_action: No action for this archived task; post-transfer closeout continues under PR 1164 / Issue 1155.
```

## Source branch closeout

```yaml
source_branch_disposition: delete_after_closeout_merge
source_branch_reason: PR 1176 is merged and the task has no continuing write ownership.
source_branch_evidence: squash merge 14aa7b312cb5dabc13f2ba42c54bb46e79ccd167 plus this terminal archive record.
```
