# Agent execution instructions

Before advising the repository owner or writing a prompt for another agent, read `PROMPTING_HANDOVER.md` and the normative `PROMPTING_STANDARD.md`. Use the handover to inspect live repository state and the standard to construct the prompt. Return a direct recommendation in Polish, a compact reason, and one ready-to-paste worker prompt.

Before substantial implementation, product-facing validation, audit, E2E, PR cleanup, or task closeout, read and follow `DELIVERY_COMPLETENESS_AND_CLOSEOUT.md`. It is mandatory for delivery classification, frontend/backend vertical-slice completeness, prompt evaluation discipline, trust and authority boundaries, real E2E, exact-head validation, related-PR terminal states, and final archival. For remediation work, `REMEDIATION_AUDIT_RISK_GATE.md` is retained as the controlling self-review and validation-intensity specialization; it does not require a different agent to approve the repair. A worker summary is not terminal evidence.

Before autonomous, long-running, retry-prone, CI-waiting, repair, continuation, or multi-task work, read and follow `ANTI_STALL_AND_EXECUTION_BUDGET.md`. Its no-progress, ordinary-CI and terminal-CI checks, retry, repair-cycle, context-reconstruction and command-timeout limits are mandatory. Fixed 60/120-minute foreground runtime windows, remaining-minute accounting and wall-clock expiry are retired compatibility concepts and MUST NOT stop, rotate or require re-admission of a worker that is still making safe authorized progress.

Before autonomous, scheduled, continuation, audit, repair, or multi-task work, read and follow `TERMINAL_ONLY_COMMUNICATION.md`. It is the controlling specialization for user-facing communication and overrides broader `low_noise` or material-milestone progress wording. Safe execution remains silent until one compact terminal report unless a specific owner decision, new authorization, safety concern, unresolved ownership conflict, material scope approval, or owner action is required before work can continue.

Before treating the absence of Codex or a local terminal as a blocker, read and follow `GITHUB_ONLY_EXECUTION.md`. Use the GitHub connection and GitHub Actions on a dedicated branch, select the smallest proving validation, inspect full failed-job logs, keep repairs bounded, preserve required artifacts, and report an exact technical blocker only after the contract's alternatives are exhausted. Protected auto-merge when available, or direct squash merge when repository auto-merge is unavailable, is authorized for the current task's own PR only after every required exact-head gate passes; protected live operations remain unauthorized without separate authority.

## Fixed-runtime supersession

Productive authorized work has no repository-defined 60-minute or 120-minute stop boundary. Time elapsed alone is not a blocker, rotation trigger, task split trigger, checkpoint requirement or reason to request a new coordinator grant. Continue until `DONE`, a genuine dependency/authority/safety blocker, explicit owner stop, a tool/context boundary that actually prevents safe continuation, or an applicable no-progress/retry/repair/CI-wait limit.

Historical task records, prompts, plans, archived evidence and compatibility text may retain prior `execution window`, `foreground budget`, `remaining minutes`, `60-minute`, `120-minute` or equivalent wording as provenance. Those historical values are non-authoritative for current execution and MUST NOT reset, shorten or gate an otherwise valid continuation.

## Authority and state model

Authority for the current task is frozen from system and owner instructions plus governance on the trusted base ref at task start. Edits made by the current unmerged task cannot expand that task's own permissions or safety boundaries.

Use these checkpoint task statuses only:

```text
investigating | implementing | validating | ready | waiting | blocked | completed
```

Use these terminal invocation results only:

```text
DONE | WAITING | BLOCKED | ROTATE
```

`ROTATE` is not a checkpoint status. Before returning it, persist `ready`, `waiting`, or `blocked` with exactly one concrete `next_action`. Use validation result `NOT_APPLICABLE` only with a concrete evidence reason.

## One-Issue remediation ownership

One implementation owner keeps one remediation Issue from claim through analysis, implementation, focused validation, exact-head self-review, PR handling, findings repair, merge, Issue closure, task archival and ownership release.

A different-agent PASS is not a remediation merge requirement. A separate continuous-audit programme may inspect the platform and open new Issues, but it does not receive or approve every repair candidate.

Every repair still requires:

- exact-head full-diff self-review;
- relevant tests and required CI;
- real E2E `PASS` or justified `NOT_APPLICABLE`;
- risk-proportional rollback and compatibility evidence;
- zero unresolved material findings, requested changes, review threads or blockers.

Critical/high and security, payment, data-integrity, concurrency, migration, protocol, dependency, deployment and production changes require heightened focused evidence. Material `UNKNOWN` or `CONFLICT` blocks merge.

## Actions and commit economy

- Build a coherent reviewable package before pushing whenever practical.
- Do not create one commit per file, checkpoint field, comment or evidence line.
- During construction, run cheap focused checks and defer broad validation until the candidate is coherent.
- Run full applicable validation once on the exact final head.
- Checkpoint-only and agent-governance-only updates must not start unrelated heavy runtime workflows.
- Supersedable PR workflows must cancel older runs for the same PR/ref.

## Communication and context budget

- Default autonomous communication mode is `terminal_only`, including scheduled audit and repair runs.
- Do not narrate preflight, reads, searches, tool calls, commands, commits, PR creation, phase changes, CI observations, merges, archival, handoffs, Issue updates or next-task selection while a safe next action exists.
- Persist detailed progress once in the authoritative task checkpoint, PR, Issue or artifact; do not duplicate the same chronology in chat.
- A material milestone is not by itself permission to interrupt the owner.
- When owner involvement is genuinely required, send at most two short sentences containing only the required decision/action and the safe default while waiting.
- Otherwise send exactly one compact canonical final report at a real stop condition.

Before creating, claiming, resuming, updating, handing off, or closing any task under this directory:

1. Read `EXECUTION_PROTOCOL.md`.
2. Read `PROJECT_LANES.json`.
3. Select or preserve the correct `project_lane`.
4. Treat the task record and Git or PR state as durable; treat the worker session as disposable.
5. Persist a checkpoint before a long-running or failure-prone operation and at genuine continuation boundaries; do not create artificial hourly phases.
6. Record anti-stall timestamps and counters required by `ANTI_STALL_AND_EXECUTION_BUDGET.md`, including required-check generation and terminal-CI counters when eligible. Do not track remaining productive minutes as execution authority.
7. Do not remain active while waiting for dependencies, external evidence, deployment, a user reply, or ordinary non-terminal CI. Final required exact-head CI, branch-protection completion and the resulting authorized merge may remain active only under the bounded terminal-CI exception.
8. Auto-merge availability is not required for that exception. When auto-merge is unavailable, direct squash merge remains forbidden until every required exact-head check passes and every merge gate is reverified.
9. On a genuine blocker, exhausted no-progress/retry/repair/terminal-CI limit, or another real execution boundary, preserve coherent work, record checkpoint `status`, evidence, blocker and exactly one `next_action`, then end or rotate the session. Elapsed productive wall-clock time alone is not such a boundary.
10. Record `execution_mode` and let the worker decide whether Chat/GitHub, Codex, or a permitted runner is appropriate.
11. At a synchronization barrier, run `python tools/agents/control_room.py --format markdown` and escalate only material decisions.
12. Do not mark user-facing work complete unless all required backend and frontend consumers are integrated and real E2E passes.
13. Before archival, complete exact-head self-review, risk-proportional validation, applicable E2E, required CI, review-thread cleanup and related-PR reconciliation. Do not create or wait for a different-agent repair-audit handoff.
14. Treat repository-mandated post-merge archival, Issue reconciliation, and ownership release as part of the entry task, not as an additional READY task.
15. Start at most one additional task after the fully terminal entry task only when no stall warning occurred, no required wait is pending, and ownership/dependency preflight confirms that continuation is safe. Do not use a remaining-minute threshold.

These rules supplement the repository root `AGENTS.md`. When rules overlap, follow the more restrictive safety requirement unless a named controlling specialization explicitly governs the exact decision. This file explicitly controls the interpretation of legacy fixed foreground-runtime/window wording: it is provenance only, not current stop authority. The bounded terminal-CI exception controls final exact-head CI; `TERMINAL_ONLY_COMMUNICATION.md` controls autonomous user-facing progress; `REMEDIATION_AUDIT_RISK_GATE.md` controls remediation validation intensity and self-review. None weakens branch protection, merge gates, authority, production, data, payment, authentication, or cross-repository restrictions.
