# Agent execution instructions

Before advising the repository owner or writing a prompt for another agent, read `PROMPTING_HANDOVER.md` and the normative `PROMPTING_STANDARD.md`. Use the handover to inspect live repository state and the standard to construct the prompt. Return a direct recommendation in Polish, a compact reason, and one ready-to-paste worker prompt.

Before substantial implementation, product-facing validation, audit, E2E, PR cleanup, or task closeout, read and follow `DELIVERY_COMPLETENESS_AND_CLOSEOUT.md`. It is mandatory for delivery classification, frontend/backend vertical-slice completeness, prompt evaluation discipline, trust and authority boundaries, independent audit, real E2E, exact-head validation, related-PR terminal states, and final archival. A worker summary is not terminal evidence.

Before autonomous, long-running, retry-prone, CI-waiting, repair, continuation, or multi-task work, read and follow `ANTI_STALL_AND_EXECUTION_BUDGET.md`. Its runtime, no-progress, CI-check, retry, repair-cycle, context-reconstruction, command-timeout, and additional-task limits are mandatory. Budget exhaustion or unchanged pending state is a real stop condition even when another contract says to continue autonomously.

Before treating the absence of Codex or a local terminal as a blocker, read and follow `GITHUB_ONLY_EXECUTION.md`. Use the GitHub connection and GitHub Actions on a dedicated branch, select the smallest proving validation, inspect full failed-job logs, keep repairs bounded, preserve required artifacts, and report an exact technical blocker only after the contract's alternatives are exhausted. Autonomous merge or auto-merge of the current task's own PR is authorized only after every required gate in that contract and this repository passes; protected live operations remain unauthorized without separate authority.

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

Before creating, claiming, resuming, updating, handing off, or closing any task under this directory:

1. Read `EXECUTION_PROTOCOL.md`.
2. Read `PROJECT_LANES.json`.
3. Select or preserve the correct `project_lane`.
4. Treat the task record and Git or PR state as durable; treat the worker session as disposable.
5. Execute one bounded phase per session and persist a checkpoint before a long-running or failure-prone operation.
6. Record anti-stall timestamps and counters required by `ANTI_STALL_AND_EXECUTION_BUDGET.md`.
7. Do not remain active while waiting for CI, dependencies, external evidence, deployment, or a user reply.
8. On a blocker or exhausted budget, preserve coherent work, record checkpoint `status`, evidence, blocker and exactly one `next_action`, then end or rotate the session.
9. Record `execution_mode` and let the worker decide whether Chat/GitHub, Codex, or a permitted runner is appropriate.
10. At a synchronization barrier, run `python tools/agents/control_room.py --format markdown` and escalate only material decisions.
11. Do not mark user-facing work complete unless all required backend and frontend consumers are integrated and real E2E passes.
12. Before archival, perform a fresh independent audit, verify exact-head required CI, resolve review threads, and merge or intentionally close every related or superseded PR.
13. Start at most one additional task after the terminal entry task, only when at least 30 minutes of declared budget remains, no stall warning occurred, and the anti-stall gate permits it.

These rules supplement the repository root `AGENTS.md`. When rules overlap, follow the more restrictive safety requirement.