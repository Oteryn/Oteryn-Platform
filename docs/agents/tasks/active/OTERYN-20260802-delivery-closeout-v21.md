---
task_id: OTERYN-20260802-delivery-closeout-v21
required_reads:
  - AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
search_first:
  - delivery completeness
  - vertical slice
  - pull request hygiene
optional_reads: []
---

# Delivery completeness and closeout v2.1

## Goal

Require prompt eval discipline, trust boundaries, complete frontend/backend vertical slices, independent audit, real E2E and terminal related-PR state before task completion.

## Policy

```yaml
policy_version: 2
task_kind: integration
implementation_authorized: false
context_pressure: low
context_growth: stable
decomposition_decision: single
execution_mode: chat
run_scope: autonomous_program
continuation_policy: continue_until_real_stop
task_completion_policy: finalize_archive_and_continue
user_communication: low_noise
```

## Scope

Owned paths:

- `docs/agents/AGENTS.md`
- `docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md`
- `docs/agents/tasks/active/OTERYN-20260802-delivery-closeout-v21.md`

No application, authentication, database, payment, production, Canary or deployment mutation.

## Acceptance

- [x] Backend-only evidence cannot close a user-facing feature when frontend is required.
- [x] Independent audit, real E2E and exact-head CI are mandatory closeout gates.
- [x] Related and superseded PRs must reach terminal states.
- [x] Prompt changes gain eval and trust-boundary discipline.
- [ ] Pass governance and required CI.
- [ ] Merge and archive this task.
