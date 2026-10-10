---
task_id: OTERYN-20261008-decanary-plan
governing_issue: 1474
required_reads:
  - docs/agents/programs/DECANARY_REMOVAL_PLAN.md
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
search_first:
  - app/GameAuth/Tickets
  - app/GameAuth/NativeLogin/NativeGameLoginTickets.php
  - deploy/synology/compose.yml
optional_reads:
  - docs/agents/tasks/active/OTERYN-20261008-decanary-login-default.md
---

# OTERYN-20261008 DECANARY-PLAN-1 removal plan

## Goal

Governing GitHub Issue: #1474 (native-only migration) — canonical lifecycle authority for this task. Plan v1 lives in Issue #1476; this task delivers plan v2 in PR #1480.

Write the plan-only document `docs/agents/programs/DECANARY_REMOVAL_PLAN.md` for the owner decision of 2026-10-08 (remove everything about Canary from Oteryn-Platform and re-point every capability at Oteryn-Game): inventory, native replacement per group, serial PR sequence with dependencies, history rule and owner questions. The task changes no code, schema, configuration, deployment or data and grants no runtime, DDL, production, Game-repository or merge authority.

## Acceptance criteria

- [x] Inventory, native replacement, PR sequence, history rule and owner questions are present.
- [x] Packet dependencies match the code at the inspected head (B1 after B7, D-4 after every Canary DB caller is off, D-8 keeps the native ticket model and issuance, C8 after the preference row inventory and Q-PREFS).
- [x] The plan lives outside the task globs of `docs/agents/PROJECT_LANES.json`.
- [ ] Exact-head CI, including checkpoint-validation, passes on PR #1480.

## Ownership

```yaml
owned_paths:
  - docs/agents/programs/DECANARY_REMOVAL_PLAN.md
  - docs/agents/tasks/active/OTERYN-20261008-decanary-plan.md
modules:
  - none (documentation only)
dependencies:
  - Issue #1474
  - Issue #1476 (plan v1)
blockers:
  - none
cross_repository_tasks:
  - none; the Game repository was not read and no Game write is authorized
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-10T21:30:00Z
status: validating
terminal_pr_policy: archive_pending
phase: pre_merge_validation
branch: claude/decanary-plan-1-20261008
head: 3215a788d4759735b2266363b8db31d29b69c9f8
pr: 1480
context_routes:
  - agent-governance
  - architecture
owned_paths:
  - docs/agents/programs/DECANARY_REMOVAL_PLAN.md
  - docs/agents/tasks/active/OTERYN-20261008-decanary-plan.md
proven:
  - PR #1480 changes documentation only.
  - IssueGameLoginTicket denies an Identity without a ready identity_canary_accounts binding.
  - NativeGameLoginTickets imports GameLoginTicket and GameLoginTicketSecrets from app/GameAuth/Tickets.
  - The Synology platform service reaches the Canary DB for public data, account overview, provisioning, creation and Bazaar transfer.
derived:
  - The plan file must stay outside docs/agents/tasks/*.md, otherwise control_room.py loads it as a task.
unknown:
  - Exact-head CI result for the head that carries this record.
conflicts: []
first_failure:
  marker: checkpoint-validation on 3215a78
  evidence: control_room.py exit 3, plan file loaded as a task and marked MISSING_FROM_REPORT
rejected_hypotheses:
  - Change tools/agents/control_room.py to ignore the plan file.
changed_paths:
  - docs/agents/programs/DECANARY_REMOVAL_PLAN.md
  - docs/agents/tasks/active/OTERYN-20261008-decanary-plan.md
validation:
  - command: python3 tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20261008-decanary-plan.md --require-checkpoint
    result: NOT_RUN
    evidence: validation is delegated to CI on the PR head
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: planning document only, no runtime behavior changes
blockers:
  - none
next_action: Run exact-head CI for PR #1480 and hand the review packet to the lead; archive this record when the PR ends the task.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```

## Notes

Owner questions Q-H, Q-ID, Q4, Q-PREFS and Q-CAT stay open in §6 of the plan. Jira sync for `KAN-41` is pending.
