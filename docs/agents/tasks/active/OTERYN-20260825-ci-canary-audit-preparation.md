---
task_id: OTERYN-20260825-ci-canary-audit-preparation
governing_issue: 1268
required_reads:
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/superpowers/specs/2026-08-25-organization-ci-canary-audit-design.md
  - docs/superpowers/plans/2026-08-25-organization-ci-canary-audit.md
search_first:
  - .github/workflows/** pull_request paths concurrency cancel-in-progress
  - CI canary audit overlapping active tasks and PRs
optional_reads: []
---

# OTERYN-20260825-ci-canary-audit-preparation

## Goal

Governing GitHub Issue: #1268 — canonical lifecycle authority for this task.

Prepare and durably persist the exact design, execution plan, and Work prompt for a controlled CI canary audit across `Oteryn/Oteryn-Platform`, `Oteryn/Oteryn-Game`, and `Oteryn/Oteryn-Atlas`, without starting the live canary PR campaign before its execution gate is satisfied.

## Acceptance criteria

- [x] Preparation Issue #1268 exists and explicitly forbids starting live canary PRs before the gate.
- [x] Current preparation snapshot SHAs for Platform, Game, and Atlas are directly verified from GitHub and labeled non-authoritative for the later live baseline.
- [x] The design defines nine inert canaries, exact branch/PR identities, execution phases, evidence schema, metrics, abort conditions, and findings taxonomy.
- [x] The implementation plan defines exact serial baseline, supersession, metadata-event, cross-repository burst, cleanup, and reporting tasks.
- [x] A standalone Work prompt `OTERYN-CI-CANARY-AUDIT-V1` is prepared for later autonomous execution.
- [ ] Preparation documents are durably committed on the dedicated Platform task branch and the remote head is verified.
- [ ] The central coordination PR/task state is persisted without creating any live canary PR.

## Ownership

```yaml
owned_paths:
  - docs/superpowers/specs/2026-08-25-organization-ci-canary-audit-design.md
  - docs/superpowers/plans/2026-08-25-organization-ci-canary-audit.md
  - docs/agents/prompts/OTERYN-CI-CANARY-AUDIT-V1.md
  - docs/agents/tasks/active/OTERYN-20260825-ci-canary-audit-preparation.md
modules:
  - agent-governance
  - testing
  - ci-audit-preparation
dependencies:
  - Oteryn/Oteryn-Platform#1268
  - Atlas verification/E2E optimization terminal-or-explicitly-frozen boundary before live execution
blockers:
  - live canary campaign intentionally gated; preparation itself is not blocked
cross_repository_tasks:
  - read-only preparation evidence from Oteryn/Oteryn-Game
  - read-only preparation evidence from Oteryn/Oteryn-Atlas
  - live cross-repository canary writes are deferred until the execution gate and later Work invocation
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-25T16:15:29Z
head: UNKNOWN
branch: test/issue-1268-ci-canary-audit-preparation
pr: none
status: implementing
context_routes:
  - agent-governance
  - testing
owned_paths:
  - docs/superpowers/specs/2026-08-25-organization-ci-canary-audit-design.md
  - docs/superpowers/plans/2026-08-25-organization-ci-canary-audit.md
  - docs/agents/prompts/OTERYN-CI-CANARY-AUDIT-V1.md
  - docs/agents/tasks/active/OTERYN-20260825-ci-canary-audit-preparation.md
proven:
  - "Platform preparation main = 2ea92ba412fe2a6721b69b021ffb888e3b93d091"
  - "Game preparation main = 5211cb26b9424925cd353822dd6e6b39b7984f21"
  - "Atlas preparation main = f48edc9170708b341df06339cae6cc113985dce8"
  - "Platform Issue #1268 is open and scoped to preparation before live canaries"
  - "No open Platform PR was found owning docs/superpowers/** during preflight"
derived:
  - "A Work coordinator with read-only per-repository static analysis and one mutating actor best isolates experiment stimuli"
  - "Atlas selector/runner optimization is a material dependency for an unqualified final baseline"
unknown:
  - "Exact live baseline SHAs and complete trigger matrix at the future campaign start"
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - "Cached preparation SHAs are sufficient for the later live baseline"
  - "Static workflow review alone can prove absence of CI amplification or loops"
changed_paths: []
validation:
  - command: design/spec self-review
    result: PASS
    evidence: preparation design contains explicit scope, nine probes, safety gates, phases, evidence schema and terminal conditions
  - command: live canary execution
    result: NOT_APPLICABLE
    evidence: Issue #1268 preparation scope explicitly defers live canary PR creation until the execution gate
blockers:
  - none
next_action: Commit the four approved preparation documents as one coherent branch update, verify the remote head, then persist the central coordination PR/task state without creating canary PRs.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: central preparation/coordination branch remains active until the durable package and later execution handoff state are established
source_branch_evidence: pending
```

## Notes

Preparation intentionally separates measurement design from remediation. The later Work run must regenerate the full workflow matrix from fresh exact SHAs and must fail closed before any canary PR if the safety or Atlas-dependency gate is not satisfied.
