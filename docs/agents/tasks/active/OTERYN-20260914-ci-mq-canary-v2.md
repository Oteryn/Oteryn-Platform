---
task_id: OTERYN-20260914-ci-mq-canary-v2
governing_issue: 1399
required_reads:
  - docs/maintenance/OTERYN_CI_MQ_CANARY_V2.md
search_first:
  - Oteryn/Oteryn#196
optional_reads:
  - none
---

# OTERYN-20260914-ci-mq-canary-v2

## Goal

Governing GitHub Issue: #1399.

Prepare the current four-repository CI + Merge Queue canary V2 and preserve one exact live execution gate per repository without launching live V2 probes during preparation.

## Acceptance criteria

- [x] V1 Issue/PR are terminally superseded and cannot be mistaken for current execution authority.
- [x] V2 covers META, Platform, Game and Atlas.
- [x] Merge Queue autonomy is split into eligibility, autonomous enqueue, real merge-group proof and automatic merge-after-enqueue.
- [x] Negative control proves a GREEN but unauthorized candidate remains unqueued/unmerged.
- [x] Existing current-generation real MQ evidence is reused before creating redundant mergeable canaries.
- [x] Minimal R0/R1/R2 anti-waste probe policy is defined.
- [x] Per-repository live admission gate is defined.
- [ ] Every repository reaches a clean/frozen live execution generation.
- [ ] Exact live trigger matrices are frozen immediately before V2 execution.
- [ ] Live V2 evidence and final report are produced in a later authorized execution stage.

## Ownership

```yaml
owned_paths:
  - docs/maintenance/OTERYN_CI_MQ_CANARY_V2.md
  - docs/agents/tasks/active/OTERYN-20260914-ci-mq-canary-v2.md
modules:
  - CI canary coordination
  - Merge Queue autonomy validation
dependencies:
  - Oteryn/Oteryn#196 governed Merge Queue control surface
blockers:
  - Platform #1398 modifies .github/workflows/codeql.yml
  - Game #592 modifies workflow/control-plane behavior
  - Atlas #492/#493 occupy verification/MQ canary surface
cross_repository_tasks:
  - META/Game/Atlas are read-only during preparation; future live probes require their own exact current authority
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-23T18:34:00Z
head: efd0ef80734d7e0c3e6c85ffdccace66e95f2914
branch: docs/issue-1399-ci-mq-canary-v2
pr: 1400
status: validating
terminal_pr_policy: archive_pending
context_routes:
  - CI validation
  - Merge Queue integration capability
owned_paths:
  - docs/maintenance/OTERYN_CI_MQ_CANARY_V2.md
  - docs/agents/tasks/active/OTERYN-20260914-ci-mq-canary-v2.md
proven:
  - V1 Issue #1268 closed not_planned as superseded by #1399
  - V1 PR #1269 closed unmerged and source branch removed
  - V2 preparation Draft PR #1400 exists on the dedicated task branch
  - Platform #1392 terminally proved delegated exact-head enqueue -> real merge_group -> platform-gate -> automatic protected merge on META 23b21e9
  - current META main is 23b21e9b1b2d4b6c3a5cac3d4c7a18747804c090
  - current META PR CI is global META CI with PR concurrency cancel-in-progress=true
  - META governed executor is issue_comment-only and terminal branch lifecycle handles PR close separately
derived:
  - META needs only one routing probe unless its workflow topology changes
  - Platform matrix must not freeze while #1398 is an open workflow-changing PR
unknown:
  - exact attributable governed enqueue receipt chain for Game #606 and Atlas #494 until fully rebound from live executor evidence
  - final exact trigger matrices for Platform/Game/Atlas after blockers move
conflicts: []
first_failure:
  marker: preparation branch creation initially unavailable
  evidence: later authorized retry succeeded and branch/PR now exist
rejected_hypotheses:
  - historical V1 nine-probe matrix remains current
  - queue receipt alone proves integration
  - generic GitHub auto-merge is equivalent to governed exact-head queue submission
changed_paths:
  - docs/maintenance/OTERYN_CI_MQ_CANARY_V2.md
  - docs/agents/tasks/active/OTERYN-20260914-ci-mq-canary-v2.md
validation:
  - command: live GitHub readback of Issue #1399, V1 terminal state, current META/Game/Platform/Atlas heads, current MQ policy/evidence and material overlapping PRs
    result: PASS
    evidence: GitHub live connector readback on 2026-09-14
  - command: live V2 canary execution
    result: NOT_APPLICABLE
    evidence: preparation task explicitly does not launch live probes
  - command: exact-head repository CI for preparation PR #1400
    result: NOT_RUN
    evidence: wait for current exact-head checks after this checkpoint update
blockers:
  - Platform #1398
  - Game #592
  - Atlas #492/#493
next_action: archive this terminal preparation packet; continue Issue #1399 live V2 execution only under a fresh bounded task and branch after current repository preflight is re-established
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: V2 preparation is active and awaiting clean live generations before later execution
source_branch_evidence: pending
```

## Notes

Do not convert this preparation branch into a live canary. Live measurement remains a separate stage controlled by #1399 and fresh repository-specific preflight.