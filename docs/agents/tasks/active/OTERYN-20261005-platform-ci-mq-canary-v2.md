---
task_id: OTERYN-20261005-platform-ci-mq-canary-v2
governing_issue: 1399
required_reads:
  - docs/maintenance/OTERYN_CI_MQ_CANARY_V2.md
search_first:
  - Oteryn/Oteryn#196
  - Platform Merge Queue current generation
optional_reads: []
---

# OTERYN-20261005 Platform CI + Merge Queue canary V2

## Goal

Continue Issue #1399 under a fresh bounded current-generation packet without colliding with Oteryn-Game or Atlas work.

This packet owns only:

1. read-only META capability/preflight evidence;
2. current Oteryn-Platform CI/Merge Queue inventory and exact trigger matrix;
3. the smallest Platform-only live probe required after evidence reuse;
4. durable V2 Platform result recording.

No Oteryn-Game or Oteryn-Atlas mutation is authorized by this task.

## Acceptance criteria

- [x] Historical preparation packet is terminalized and archived.
- [x] Current META protected main is frozen for capability evidence.
- [x] Current Platform protected main is frozen for preflight.
- [x] Current META governed executor is proven operational on the frozen META generation.
- [x] Current open Platform PRs are inspected for overlap with the measured CI/MQ surface.
- [x] Exact current Platform PR-capable workflow and downstream trigger matrix is frozen.
- [x] Existing recent Platform evidence is classified as reusable or insufficient for each V2 state.
- [ ] If evidence is insufficient, create only the smallest bounded Platform canary needed.
- [ ] Prove or classify ELIGIBLE, AUTO_ENQUEUE, MERGE_GROUP_PROVEN and AUTO_MERGE_AFTER_ENQUEUE separately.
- [ ] Persist final Platform V2 evidence without claiming Game/Atlas completion.

## Ownership

```yaml
owned_paths:
  - docs/maintenance/OTERYN_CI_MQ_CANARY_V2.md
  - docs/agents/tasks/active/OTERYN-20261005-platform-ci-mq-canary-v2.md
modules:
  - Platform CI routing
  - Platform Merge Queue autonomy validation
dependencies:
  - Oteryn/Oteryn#196 read-only control-plane evidence
blockers:
  - none for Platform preflight
cross_repository_tasks:
  - META is read-only except a future exact governed queue request if and only if the Platform live probe reaches authorized AUTO_ENQUEUE measurement
  - Oteryn-Game: no mutation
  - Oteryn-Atlas: no mutation
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-05T11:58:00Z
head: 1a3302b451ed8d5c67f85c6316386f846144fbb0
branch: test/issue-1399-platform-ci-mq-v2-20261005
pr: none
status: validating
terminal_pr_policy: active
context_routes:
  - ci
  - merge-queue
owned_paths:
  - docs/maintenance/OTERYN_CI_MQ_CANARY_V2.md
  - docs/agents/tasks/active/OTERYN-20261005-platform-ci-mq-canary-v2.md
proven:
  - Platform protected main is b143cff5dd19f622990791d2f9b93d2c483020f8 after Issue #1010 closeout
  - META protected main is 56419b6e463d28bf5ea71a92303a75ab6bf5207c
  - META governed executor run 37118716936 executed from exact META main 56419b6e463d28bf5ea71a92303a75ab6bf5207c with OTERYN_MQ_FINE_GRAINED_PAT provisioned and returned REQUEST_ACCEPTED_NON_TERMINAL with a provider UUID
  - old Platform blocker #1398 is closed
  - old Game blocker #592 is closed
  - Atlas #492/#493 remain open, but #1399 permits repository-by-repository measurement
  - current Platform open PRs are #1439, #1294, #1211, #1120 and #1116
  - #1439 changes composer.lock only
  - #1294, #1120 and #1116 change documentation/control-plane records only
  - #1211 changes repair-synology-autostart.yml, which has push-main/workflow_dispatch triggers only and no pull_request or merge_group trigger
derived:
  - no current open Platform PR is actively changing the PR/Merge Queue routing surface measured by this packet
  - current META executor capability is operational, but target-specific Platform AUTO_ENQUEUE must still be proven or explicitly classified from exact current evidence
  - recent PR #1447 proves a real current-generation Platform merge_group and automatic protected merge but does not by itself prove governed exact-head AUTO_ENQUEUE because generic auto-merge transport is not equivalent to the #196 executor contract
unknown:
  - exact governed AUTO_ENQUEUE result for the fresh delivery PR
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - all four repositories must be idle before any V2 measurement
  - Atlas #492/#493 block Platform-only preflight
  - PR #1447 generic auto-merge alone proves governed AUTO_ENQUEUE
changed_paths:
  - docs/agents/tasks/archive/OTERYN-20260914-ci-mq-canary-v2.md
  - docs/agents/tasks/active/OTERYN-20261005-platform-ci-mq-canary-v2.md
matrix:
  r0_docs_agents_only:
    expected_pull_request:
      - CI
      - Agent Governance
    opened_trusted_base:
      - Historical Branch Audit
    close_housekeeping:
      - GitHub Actions Storage Hygiene
      - Terminal Branch Lifecycle
    forbidden_or_not_applicable:
      - CodeQL
      - Edge Security Emulation
      - Platform DB Outage Validation
      - Game Auth Ticket Concurrency
      - Phase 7 Production-Like Validation
      - php-coverage-report
      - runtime-tests
    observed_reference:
      pr: 1448
      exact_head: 989c4f207fe1c25324fc82026d6a20a0fd35c3a8
      ci: 37305682464
      agent_governance: 37305682341
      historical_branch_audit: 37305574333
      storage_hygiene_close: 37305850971
      terminal_branch_close: 37305850961
  merge_queue_reference:
    pr: 1448
    merge_group_run: 37305783477
    merge_group_sha: b143cff5dd19f622990791d2f9b93d2c483020f8
    protected_merge: b143cff5dd19f622990791d2f9b93d2c483020f8
  evidence_classification:
    ELIGIBLE: fresh delivery PR exact-head green state still required
    AUTO_ENQUEUE: INSUFFICIENT from #1447/#1448 because generic auto-merge transport is not the governed #196 executor contract
    MERGE_GROUP_PROVEN: REUSABLE from #1448 current-generation merge_group run 37305783477
    AUTO_MERGE_AFTER_ENQUEUE: REUSABLE from #1448 protected merge b143cff5dd19f622990791d2f9b93d2c483020f8
validation:
  - command: live GitHub readback of META main and governed executor run 37118716936
    result: PASS
    evidence: exact META main 56419b6e463d28bf5ea71a92303a75ab6bf5207c; accepted request returned provider UUID
  - command: live Platform open-PR overlap inspection
    result: PASS
    evidence: no open PR changes current pull_request/merge_group CI routing surface; #1211 workflow is push-main/workflow_dispatch only
blockers: []
next_action: open this docs-only delivery PR as the single R0 probe; after exact-head gates pass, issue exactly one governed META #196 request for that exact head to prove AUTO_ENQUEUE, then observe provider merge_group and protected merge without any second integration mutation
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository V2 preflight/execution branch; no retention purpose after terminal delivery
source_branch_evidence: pending
```
