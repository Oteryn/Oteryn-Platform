# Terminal-Only Autonomous Communication Contract

```yaml
terminal_only_communication_policy_version: 1
```

## Purpose

Long autonomous runs must preserve context for execution rather than spend it on a running commentary. Durable evidence belongs in Git, task records, PRs, Issues, workflow artifacts and checkpoints. The owner should normally receive one compact terminal report when the invocation reaches a real stop condition.

This contract is the controlling specialization for user-facing communication during autonomous start, resume, continuation, scheduled audit and scheduled repair runs. It is more restrictive than broader `low_noise` or material-milestone wording elsewhere in repository governance.

## Default mode

```yaml
user_communication: terminal_only
intermediate_progress_messages: forbidden_by_default
final_reports_per_invocation: 1
```

Treat `user_communication: low_noise` as `terminal_only` unless the owner explicitly requests live progress updates in the current invocation.

## No intermediate narration

During safe autonomous execution, do not send user-facing messages for:

- repository or task preflight;
- reading files, Issues, PRs, logs or artifacts;
- searches, tool calls, commands or test selection;
- branch, task, commit or draft-PR creation;
- checkpoint writes or `next_action` changes;
- phase transitions between implementation, validation, audit, E2E, CI, merge and archival;
- successful or pending CI observations;
- ordinary CI failures that the agent can diagnose and repair autonomously;
- merge completion, archive-PR creation, task archival or selection of the next safe `READY` task;
- restating exact SHAs, run IDs, route counts, screen counts or module counts already persisted durably.

Continue executing silently and write the detailed evidence to the durable project state.

Do not emit messages such as `Kontynuuję`, `Sprawdzam`, `Teraz przechodzę`, `PR został scalony`, `Rozpoczynam kolejny etap` or chronological descriptions of completed tool activity while another safe action remains.

## Permitted intermediate interruption

An intermediate user-facing message is allowed only when execution cannot safely continue without owner involvement because of at least one of these conditions:

- a specific product, architecture, legal or business decision is required;
- new repository, production, payment, protected-environment, credential or cross-repository authorization is required;
- a real safety or data-exposure concern requires immediate owner attention;
- two active owners claim overlapping paths and repository evidence cannot resolve the conflict;
- the requested scope must materially expand and the expansion requires approval;
- a tool or permission failure leaves no safe executable alternative and requires an owner action.

The message must contain only the decision or action required, the smallest supporting evidence and the safe default while waiting. Use at most two short sentences unless the owner explicitly asks for detail. Do not send repeated reminders while the required state is unchanged.

CI pending, normal repair work, a merge, a completed phase, a new finding that can be recorded as an Issue, or a context checkpoint is not an owner-interruption condition.

## Final report

Send one final response only when a real stop condition is reached. Use the canonical terminal response from `ANTI_STALL_AND_EXECUTION_BUDGET.md`, but keep it compact:

- summarize the observable result rather than the chronology;
- cite exact durable task, PR, Issue, head and run identifiers only where needed;
- avoid repeating evidence already stored in the task or PR;
- report exactly one `NEXT_ACTION` when not `DONE`;
- do not include hidden reasoning, a tool diary or a paragraph for every phase.

## Context economy

- Persist detailed progress once in the authoritative task checkpoint or PR; do not duplicate the same narrative in chat, Issue comments and PR text.
- Update durable state after material changes, not after every read or command.
- Prefer identifiers and result classifications over copied logs.
- Reuse the existing checkpoint instead of reconstructing history in user-facing messages.
- A worker or role handoff remains internal and durable; it is not a reason to notify the owner.

## Conflict rule

When this contract conflicts with a general request to keep the owner informed, `terminal_only` controls for autonomous runs unless the owner explicitly requests live updates in that invocation. System, developer, safety and explicit current-owner instructions remain higher authority.