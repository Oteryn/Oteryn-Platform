---
task_id: OTERYN-20260805-platform-agent-scope-lock
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
search_first:
  - OTERYN_PLATFORM_CONTINUOUS_AUDIT
  - OTERYN_PLATFORM_REMEDIATION
  - OTERYN_PLATFORM_ARCHITECTURE_REVIEW
optional_reads: []
---

# OTERYN-20260805-platform-agent-scope-lock

## Goal

Make the three durable Oteryn Platform programmes permanently repository-scoped so that no invocation, Issue, comment, task, PR, programme state or later owner wording can redirect those programme identities to another repository or product area.

## Acceptance criteria

- [x] One canonical immutable scope contract names `blakinio/Oteryn-Platform` as the sole execution and write repository.
- [x] External repositories and systems may be inspected only read-only when directly necessary to verify a Platform-owned boundary.
- [x] The three programme identities cannot accept cross-repository write authorization; such work requires a different programme/task identity.
- [x] The short-command registry and all three programme states require the scope contract.
- [x] Documentation/governance validation and exact-head CI pass.
- [x] The task is archived and ownership released after merge of this archive record.

## Ownership

```yaml
owned_paths: []
released_paths:
  - docs/agents/OTERYN_PLATFORM_PROGRAM_SCOPE.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/evidence/OTERYN-20260805-platform-agent-scope-lock/prompt-eval.md
  - docs/agents/tasks/active/OTERYN-20260805-platform-agent-scope-lock.md
  - docs/agents/tasks/archive/OTERYN-20260805-platform-agent-scope-lock.md
modules:
  - agent-governance
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-05T14:36:00Z
head: 80ee224de156e0e614a117b98d0a4a44e5b5f483
branch: docs/archive-platform-agent-scope-lock-20260805
pr: 545
status: completed
context_routes:
  - agent-governance
owned_paths: []
proven:
  - PR 545 merged the immutable programme scope contract and all required bindings into main.
  - The sole execution and write repository for the three programme IDs is blakinio/Oteryn-Platform.
  - Otheryn, OTClient, Canary, Freqtrade, Quant Platform, GitHub Projects control and every other repository/product area are excluded from these programme identities.
  - Narrow external inspection remains read-only and is permitted only to verify a Platform-owned boundary.
  - Static adversarial evaluation passed 15 of 15 candidate cases.
  - Exact-head workflows Agent Governance 31015678676, CI 31015678624, Game Auth Ticket Concurrency 31015678607, Platform DB Outage Validation 31015678621, Edge Security Emulation 31015678523 and Phase 7 Production-Like Validation 31015678671 all completed successfully on 5fa3defb121c41f460ee0d83d8a8d5cb377e9286.
  - PR 545 had no review threads and merged as commit 80ee224de156e0e614a117b98d0a4a44e5b5f483.
derived:
  - Separately authorized work for another repository requires a distinct programme or task identity and cannot reuse these three Platform programme IDs.
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Existing prompt wording alone was not fully immutable because it contained a separate-authorisation exception.
  - Labels, names and repository metadata alone were insufficient as an authority boundary.
changed_paths:
  - docs/agents/OTERYN_PLATFORM_PROGRAM_SCOPE.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/agents/programs/OTERYN_PLATFORM_REMEDIATION.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/evidence/OTERYN-20260805-platform-agent-scope-lock/prompt-eval.md
  - docs/agents/tasks/archive/OTERYN-20260805-platform-agent-scope-lock.md
validation:
  - command: static adversarial scope evaluation
    result: PASS
    evidence: docs/agents/evidence/OTERYN-20260805-platform-agent-scope-lock/prompt-eval.md; 15/15 candidate cases pass
  - command: fresh exact-diff review
    result: PASS
    evidence: PR 545 contained only declared agent-governance paths and had no material findings or review threads
  - command: runtime E2E
    result: NOT_APPLICABLE
    evidence: agent-governance documentation only; no runtime product behavior changed
  - command: exact-head GitHub Actions
    result: PASS
    evidence: all six workflows succeeded on 5fa3defb121c41f460ee0d83d8a8d5cb377e9286
blockers:
  - none
next_action: none
```

## Closeout

```yaml
implementation_pr: 545
implementation_merge_commit: 80ee224de156e0e614a117b98d0a4a44e5b5f483
archive_delivery: this record is authoritative only after the archive PR containing it is merged
ownership_released: true
related_open_prs: []
runtime_e2e: NOT_APPLICABLE
final_result: DONE
```
