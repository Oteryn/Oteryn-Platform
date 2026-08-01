# Agent Prompting Standard

## Purpose

This is the normative standard for advising the repository owner and writing prompts for worker agents. Live Git, task records, PRs, CI, and durable evidence override previous chat history.

Owner-facing advice is written in Polish unless requested otherwise. Worker prompts are concise English by default.

## Required owner-facing result

Return one recommendation, one compact reason, and one ready-to-paste prompt:

```text
Rekomendacja: <Chat / Codex / Work / fresh validator and single|phased|split|discovery_first>
Dlaczego: <compact reason>
Prompt dla agenta:
<one prompt>
```

Before recommending, resolve the repository, project lane, active task/checkpoint, branch, exact head, PR, required CI, first relevant failure, path ownership, overlapping work, and task-specific contracts from live state when available.

## Execution-mode routing

- **Chat**: coordination, GitHub/task/PR/CI inspection, scope and architecture decisions, prompt writing, compact documentation, and evidence review without a checkout.
- **Codex**: bounded checkout-based work requiring multi-file edits, terminal commands, build/test/fix loops, generated files, migrations, or runtime reproduction.
- **Work**: bounded broad research or a large non-code deliverable only.

Do not use Codex or Work for waiting, repeated polling, prompt generation, routine coordination, or broad status narration.

## Task shape

Default to one task, one branch, and one PR.

- `single`: one coherent worker can finish the objective.
- `phased`: one task continues through bounded discovery, implementation, and validation sessions.
- `discovery_first`: feasibility or boundaries are too uncertain for safe implementation authorization.
- `split`: ownership, acceptance criteria, durable outputs, or independent domains are genuinely separable.

Long duration, many files, a slow build, Codex use, or context growth are not split triggers. Prefer a compact checkpoint and a fresh session on the same task.

## Mandatory worker-prompt sections

Every prompt must contain:

1. **Role and phase** — one role, one bounded phase.
2. **Repository and live state** — repository, task path, expected branch/PR, and an instruction to verify exact head, CI, checkpoint, and ownership before mutation.
3. **Objective** — one outcome-oriented invariant, not “review/fix everything.”
4. **Authorization and scope** — what may be changed and what is forbidden. Audits default to `implementation_authorized: false`.
5. **Required reads and owned paths** — only the active task, execution/handoff rules, task-specific contracts, current PR, and relevant failure evidence.
6. **Policy v2** — `task_kind`, `context_pressure`, `decomposition_decision`, and `execution_mode`.
7. **Execution procedure** — verify, perform minimal discovery, produce one coherent result, validate, persist checkpoint/commit, then run the final gate.
8. **Acceptance and validation** — focused, component, and heavy-final checks.
9. **Durable state** — checkpoint updates after material discoveries, changes, validation, blockers, head/PR changes, and before session rotation.
10. **Stop conditions** — complete, blocked, waiting, ownership conflict, authorization required, unsafe context pressure, or two failed heavy attempts.
11. **Final response contract** — compact status, result, validation, durable state, blocker, and one next action.

## Validation policy

Use staged validation:

```text
Focused: changed-file checks, unit tests, type/contract checks, or minimal reproduction
Component: relevant package, component build, or bounded integration suite
Heavy final gate: full build, E2E, regression suite, or matrix, normally once after coherent implementation
```

After a heavy failure, isolate the first relevant error and reproduce it cheaply before another full run. A session normally performs no more than two heavy attempts.

## Durable state and evidence

The checkpoint preserves `PROVEN`, `DERIVED`, `UNKNOWN`, and `CONFLICT` evidence; branch/head/PR; changed paths; validation; first relevant failure; blockers; and exactly one concrete `next_action`.

Full logs, screenshots, traces, SQL snapshots, binaries, and large reports belong in artifacts or an evidence index, not in prompts or checkpoints.

A worker must not remain active merely to wait for CI, another task, deployment, an observation window, or a user reply. It records `waiting` or `blocked`, leaves one next action, and exits.

## Specialized rules

### Audit

Define the boundary and severity model. Default to no implementation. Require each finding to include severity, confidence, evidence, impact, and recommendation. Record unrelated remediation as recommendations.

### E2E

Define start state, fixtures, sequence, and observable acceptance criteria. Keep shared-state steps in one task. Store logs, screenshots, traces, SQL snapshots, and binaries as artifacts. Separate platform repair from feature validation when ownership differs.

### Implementation

State the invariant, compatibility constraints, owned paths, and focused tests. Use Codex only when local execution is necessary. Material architecture decisions remain with the coordinator unless evidence proves a required change.

### CI repair

Name the exact PR head, workflow, job, and first relevant error. Require the cheapest reproduction first. Do not rerun the full suite after every edit. Verify the exact final head.

### Independent validation

Prefer a fresh validator session on the same task. Forbid implementation unless a tightly bounded proven defect is authorized. Verify the exact candidate head and report evidence.

### Stale recovery

Verify that no previous worker is still writing. Inspect task, branch, PR, commits, checks, and ownership. Repair the checkpoint before substantive work and continue from the last coherent commit with a new `session_id`.

## Base template

```text
ROLE
You are the <role> for task <TASK_ID>, phase: <PHASE>.

REPOSITORY AND LIVE STATE
Repository: <owner/repo>
Task record: <path>
Expected branch: <branch>
Expected PR: <number or none>
Verify the live checkpoint, exact head, PR, required CI, and path ownership before changing state. Durable repository state overrides chat history.

OBJECTIVE
<One outcome-oriented invariant.>

AUTHORIZATION AND SCOPE
<Authorization.>
Owned paths: <paths or require a claim before editing>
Do not merge, deploy, change unrelated contracts, or expand scope unless explicitly authorized.

POLICY
policy_version: 2
task_kind: <kind>
context_pressure: <low|medium|high|unbounded>
decomposition_decision: <single|phased|split|discovery_first>
execution_mode: <chat|codex|work>

REQUIRED READS
- <active task record>
- docs/agents/EXECUTION_PROTOCOL.md
- docs/agents/CONTEXT_HANDOFF.md
- <smallest task-specific contracts>

EXECUTION
1. Verify live state, ownership, and the current next action.
2. Perform only the discovery needed for this phase.
3. Complete one coherent change or evidence package.
4. Run focused validation before broader validation.
5. Persist a coherent commit and compact checkpoint after a milestone and before long or failure-prone operations.
6. Run the heavy final gate only when the coherent result is ready.

ACCEPTANCE AND VALIDATION
Acceptance: <criteria>
Focused: <check>
Component: <check or not required>
Heavy final gate: <check or not required>
After a heavy failure, reproduce the first relevant error cheaply before rerunning. Do not exceed two heavy attempts in one session.

STOP CONDITIONS
Stop and checkpoint when complete, blocked, waiting, ownership conflicts, owner authorization is required, or session rotation is safer. Never remain open merely to poll or wait.

FINAL RESPONSE
STATUS: DONE | BLOCKED | WAITING | ROTATE
RESULT: <compact result>
VALIDATION: <checks and outcomes>
DURABLE_STATE: <task path, branch, head, PR>
BLOCKER: <none or exact blocker>
NEXT_ACTION: <one action or none>
```

## Quality gate

Before presenting a prompt, confirm one objective, explicit live-state verification, clear authorization and ownership, justified task shape, cheapest capable mode, explicit acceptance/validation, checkpoint/evidence rules, stop conditions, and compact final response. Reject unbounded remediation, background supervision, repeated polling, and unnecessary full logs.