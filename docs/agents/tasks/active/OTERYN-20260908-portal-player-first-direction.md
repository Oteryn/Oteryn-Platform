---
task_id: OTERYN-20260908-portal-player-first-direction
governing_issue: 1356
required_reads:
  - docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
search_first:
  - live Issue #1356, current protected main, active Portal tasks and open Portal PRs
optional_reads: []
---

# OTERYN-20260908-portal-player-first-direction

## Goal

Governing GitHub Issue: #1356 — canonical lifecycle authority for this task.

Persist the owner-approved player-first Portal completion direction as a durable companion to the existing Portal Completion hierarchy, without resurrecting terminal `PORTAL-POLISH`, creating a second selector, or changing runtime behavior.

## Acceptance criteria

- [x] Add a durable player-facing completion direction document covering product promise, truthful release/service state, Oteryn/Tibia-oriented art direction, homepage composition, page-family completion, EN/PL editorial quality, player journey, empty-state policy, implementation boundary and release acceptance.
- [x] Preserve explicit unknowns and prohibit invented gameplay/release/runtime claims.
- [x] Link the direction from `docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md` without changing selector authority or delivery ordering.
- [x] Keep the change documentation/governance only; no runtime, production, auth, payment, game-server or deployment behavior changes.
- [x] Pass applicable exact-head documentation/governance validation and protected integration requirements.

## Ownership

```yaml
owned_paths:
  - docs/architecture/PORTAL_PLAYER_EXPERIENCE_COMPLETION_DIRECTION.md
  - docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
  - docs/agents/tasks/active/OTERYN-20260908-portal-player-first-direction.md
modules:
  - web-cms
  - portal-completion
dependencies:
  - OTERYN_PORTAL_COMPLETION accepted architecture and live selector
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-08T13:58:30Z
head: 3e31a1ae06524e2af09d2a402432cea61b85e5d6
branch: docs/1356-portal-player-first-direction
pr: 1357
status: ready
context_routes:
  - portal-completion
owned_paths:
  - docs/architecture/PORTAL_PLAYER_EXPERIENCE_COMPLETION_DIRECTION.md
  - docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
  - docs/agents/tasks/active/OTERYN-20260908-portal-player-first-direction.md
proven:
  - Owner requested that the comprehensive player-first Portal completion solution agreed in the current session be persisted to the repository.
  - Issue #1356 owns this documentation-only record and explicitly does not resurrect PORTAL-POLISH or replace the live Portal Completion selector.
  - Protected main at task admission is 2acb8b548b413b9ce522ff5c507b4f10c9d0ff0e.
  - PR #1357 changes exactly the new player-experience companion direction, its one-line delivery-plan reference and this task packet.
  - PR head 3e31a1ae06524e2af09d2a402432cea61b85e5d6 passed CI run 34234974571 including platform-gate, Agent Governance 34234974487, Edge Security 34234974615, Game Auth Ticket Concurrency 34234974431, Native protocol contract audits 34234974543, Native protocol contract 34234974889, Phase 7 Production-Like Validation 34234974928 and Platform DB Outage Validation 34234974752.
derived:
  - The direction is durable and discoverable from the existing Portal Completion delivery hierarchy without creating a parallel queue.
  - The implementation content is ready for the repository's protected integration path; the checkpoint-only readiness update does not add runtime/product scope.
unknown:
  - Resulting protected-main SHA after PR #1357 integration.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Reopening the completed PORTAL-POLISH task is required to preserve the new owner direction.
  - The player-first direction requires a runtime change in the same PR.
changed_paths:
  - docs/architecture/PORTAL_PLAYER_EXPERIENCE_COMPLETION_DIRECTION.md
  - docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
  - docs/agents/tasks/active/OTERYN-20260908-portal-player-first-direction.md
validation:
  - command: product/browser E2E
    result: NOT_APPLICABLE
    evidence: This task records owner-approved documentation direction only and changes no executable product path.
  - command: CI 34234974571 / platform-gate
    result: PASS
    evidence: Exact PR head 3e31a1ae passed change classification, checkpoint validation, required test gate and platform-gate.
  - command: Agent Governance 34234974487
    result: PASS
    evidence: Exact PR head 3e31a1ae passed repository governance validation.
  - command: ancillary exact-head workflows
    result: PASS
    evidence: Runs 34234974615, 34234974431, 34234974543, 34234974889, 34234974928 and 34234974752 all completed successfully on 3e31a1ae.
blockers:
  - none
next_action: Enqueue PR #1357 through the repository's configured protected Merge Queue; after verified integration, archive this task, close Issue #1356 and verify source-branch deletion.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: player-first direction and validation evidence are durable in PR #1357; delete the source branch after verified protected integration
source_branch_evidence: Issue #1356; PR #1357; validated implementation head 3e31a1ae06524e2af09d2a402432cea61b85e5d6
```

## Notes

No personal identifiers, secrets, runtime credentials, production claims or invented gameplay facts belong in this record.
