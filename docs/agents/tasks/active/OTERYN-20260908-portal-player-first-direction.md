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
- [ ] Pass applicable exact-head documentation/governance validation and protected integration requirements.

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
updated_at: 2026-09-08T13:55:30Z
head: fc5f423bc4cebf29bfccf8d6dcbb6b427dcc8c14
branch: docs/1356-portal-player-first-direction
pr: 1357
status: validating
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
  - PR #1357 contains exactly the new player-experience companion direction, its delivery-plan reference and this task packet at the pre-checkpoint candidate fc5f423bc4cebf29bfccf8d6dcbb6b427dcc8c14.
derived:
  - The direction is durable and discoverable from the existing Portal Completion delivery hierarchy without creating a parallel queue.
unknown:
  - Exact-head documentation/governance and required-check result for the final PR head after this checkpoint update.
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
  - command: exact-head documentation/governance validation
    result: NOT_RUN
    evidence: Final PR head checks are pending after the checkpoint update.
blockers:
  - none
next_action: Verify the exact final PR #1357 head, inspect its complete three-file diff, and confirm applicable required/documentation governance checks before protected integration.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: documentation task is validating in PR #1357
source_branch_evidence: Issue #1356; PR #1357; branch docs/1356-portal-player-first-direction
```

## Notes

No personal identifiers, secrets, runtime credentials, production claims or invented gameplay facts belong in this record.
