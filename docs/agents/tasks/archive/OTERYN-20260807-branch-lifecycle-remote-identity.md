---
task_id: OTERYN-20260807-branch-lifecycle-remote-identity
issue: 815
status: completed
completed_at: 2026-08-07T14:16:09Z
implementation_pull_request: 822
implementation_head: 911837bed2daa57be59323395bf0552d67de05a1
implementation_merge: da0ae1e792a90bb0774b6028195b9f4519f50516
risk: high
validation_intensity: HEIGHTENED
self_review: PASS
material_findings: 0
production_activation_authorized: false
ownership: RELEASED_ON_ARCHIVE_MERGE
---

# OTERYN-20260807 branch lifecycle remote identity — Completed

## Result

Issue #815 is repaired. Destructive Branch Lifecycle git operations can no longer rely on ambient working-directory state or an unverified local remote while GitHub API safety reads target `blakinio/Oteryn-Platform`.

The delivered safety boundary:

- binds all destructive git subprocesses to the configured repository root;
- verifies `git rev-parse --show-toplevel` resolves to that exact root before destructive execution;
- resolves exactly one push URL for the configured git remote;
- normalizes supported GitHub HTTPS, SCP-style SSH and `ssh://git@github.com/...` forms;
- requires the normalized remote owner/repository identity to equal `GitHubClient.repo` before `git push`;
- fails closed for missing, ambiguous, unsupported or foreign push remotes and for wrong-root execution;
- preserves the exact `--force-with-lease=refs/heads/<branch>:<expected_sha>` atomic deletion boundary introduced by Issue #793.

## Delivery

- Implementation PR: #822.
- Final exact implementation head: `911837bed2daa57be59323395bf0552d67de05a1`.
- Implementation base/main: `b4f4ad5325d3eeb5733947ad2902ef50e6c6c14a`.
- Protected squash merge: `da0ae1e792a90bb0774b6028195b9f4519f50516`.
- Issue #815 closed automatically as completed by the merge.
- Source branch `repair/issue-815` was automatically deleted after merge.

## Exact-head validation

All pull-request workflows observed for final head `911837bed2daa57be59323395bf0552d67de05a1` passed:

- Branch Lifecycle `31186492049`: PASS;
- Agent Governance `31186491920`: PASS;
- CI `31186491768`: PASS;
- Edge Security Emulation `31186492703`: PASS;
- Game Auth Ticket Concurrency `31186491846`: PASS;
- Platform DB Outage Validation `31186491834`: PASS;
- Phase 7 Production-Like Validation `31186492156`: PASS.

The Branch Lifecycle validate job ran 28 focused tests and the live dry-run job passed. The reviewed-manifest apply job was correctly skipped on the pull-request event, so validation performed no branch deletion.

Repository-required `classify-changes` and `test` jobs both passed on the exact final head. Runtime tests were correctly path-skipped by repository CI routing.

The HEIGHTENED self-review was recorded on PR #822 for the exact final head and returned `PASS` with zero material findings, zero review threads and zero requested changes.

Deterministic coverage includes supported remote normalization, wrong-root rejection, foreign-origin rejection, ambiguous push-URL rejection and preservation of the prior force-with-lease last-instruction race semantics.

## E2E and safety

Production/staging product E2E is `NOT_APPLICABLE`: this repair changes repository-governance branch deletion safety only. The executable boundary was validated by deterministic mocked destructive-path tests, the non-destructive Branch Lifecycle live dry-run and exact-head repository workflows.

No production, staging, protected-environment, secret or external-repository mutation was performed. No reviewed deletion candidate set was activated by the repair PR.

## Rollback and compatibility

A squash revert of PR #822 restores the prior unbound git-root/remote behavior while leaving Issue #793 force-with-lease semantics available in history.

Existing supported `origin` usage remains compatible when its single push URL resolves to the configured GitHub repository. Supported HTTPS and SSH URL forms normalize case-insensitively for repository identity. The implementation fails closed when git is unavailable, the configured root is not the actual working-tree root, the remote is missing or ambiguous, its URL is unsupported, or its repository identity differs from `GitHubClient.repo`.

## Ownership release

This archival closeout removes the durable active-task lease for Issue #815. Once this archive change is merged, all Issue #815 implementation paths and lifecycle ownership are released for subsequent remediation work.
