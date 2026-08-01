# Agent Prompting Standard

## Purpose

This is the normative standard for advising the repository owner, interpreting short programme commands, and writing prompts for worker agents. Live Git, task records, PRs, CI, ownership, and durable evidence override previous chat history.

Owner-facing advice is written in Polish unless requested otherwise. Worker prompts are concise English by default.

This standard distinguishes:

- a **worker session**, which owns one bounded phase and may end or rotate;
- an **owner invocation**, which may coordinate and execute many consecutive phases or tasks before returning;
- a **durable programme**, whose state lives in Git and task records rather than in one conversation.

The autonomous multi-task loop is defined by `docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md` and is mandatory when `run_scope: autonomous_program` is selected.

## Invocation modes

### Advisory request

When the owner asks for a plan, recommendation, execution mode, or worker prompt, return one recommendation, one compact reason, and one ready-to-paste prompt:

```text
Rekomendacja: <Chat / Codex / Work / fresh validator and single|phased|split|discovery_first>
Dlaczego: <compact reason>
Prompt dla agenta:
<one prompt>
```

### Short programme invocation

When the owner writes a registered or clearly resolvable command such as:

```text
Uruchom <program> autonomicznie.
Kontynuuj <program> autonomicznie.
Zweryfikuj <program> <task>.
Pokaż stan <program>.
```

do not merely return a long prompt or ask the owner to manage phases. Resolve the programme from live repository state and execute the current autonomous run according to `AUTONOMOUS_PROGRAM_CONTINUATION.md`.

A short invocation authorizes a long foreground run through as many safe and ready phases or tasks as the current tools, context, ownership, and safety gates permit. It does not authorize background execution after the assistant has returned a final response.

## Required live-state resolution

Before recommending or executing, resolve when available:

- repository and project lane;
- programme/coordinator task;
- active and ready task checkpoints;
- branch, exact head, PR, required CI, and first relevant failure;
- path ownership and overlapping work;
- dependency barriers and current wave;
- task-specific contracts and safety boundaries.

Do not ask the owner for information that live Git, task records, PRs, CI, registries, or repository documentation can resolve.

## Run scope

Every substantial prompt must declare one run scope:

```yaml
run_scope: single_task | autonomous_program
continuation_policy: stop_at_task_boundary | continue_until_real_stop
task_completion_policy: checkpoint_only | finalize_archive_and_continue
user_communication: low_noise
```

Use `single_task` for one isolated deliverable when no durable programme continuation is requested.

Use `autonomous_program` when:

- the owner invokes a durable programme with a short command;
- a coordinator task or wave graph exists;
- several dependent tasks should be completed without owner micromanagement;
- the owner explicitly asks the agent to keep going autonomously.

For `autonomous_program` use:

```yaml
continuation_policy: continue_until_real_stop
task_completion_policy: finalize_archive_and_continue
user_communication: low_noise
```

A checkpoint, commit, PR creation, green CI result, merge, completed phase, or archived task is a milestone, not an automatic reason to return to the owner. After each milestone, reassess live state and continue when another safe action is ready.

## Execution-mode routing

- **Chat**: coordination, GitHub/task/PR/CI inspection, scope and architecture decisions, prompt construction, compact documentation, evidence review, barrier review, task finalization, and selection of the next ready package.
- **Codex**: bounded checkout-based work requiring multi-file edits, terminal commands, build/test/fix loops, generated files, migrations, or runtime reproduction.
- **Work**: bounded broad research or a large non-code deliverable only.
- **Fresh validator**: independent exact-head verification on the same task after the implementer releases the branch/worktree.

Do not spend Codex or Work capacity on prompt generation, routine coordination, repeated polling, waiting, or broad status narration.

The coordinator may dispatch or emulate several bounded worker sessions during one owner invocation. A worker session ending does not require the owner invocation to end.

## Task shape

Default to one task, one branch, and one PR.

- `single`: one coherent task can finish the objective;
- `phased`: one task continues through bounded discovery, implementation, validation, integration, or close phases;
- `discovery_first`: feasibility or boundaries are too uncertain for safe implementation authorization;
- `split`: ownership, acceptance criteria, durable outputs, or independent domains are genuinely separable.

Long duration, many files, a slow build, Codex use, or context growth are not split triggers. Prefer checkpoints and replacement sessions on the same task. Split only genuinely independent work.

## Mandatory worker-prompt sections

Every prompt must contain:

1. **Role and phase** — one role and one bounded phase.
2. **Repository and live state** — repository, programme/task path, expected branch/PR, and verification of exact head, CI, checkpoint, and ownership before mutation.
3. **Objective** — one outcome-oriented invariant, not “review/fix everything.”
4. **Authorization and scope** — what may change and what remains forbidden. Audits default to `implementation_authorized: false`.
5. **Required reads and owned paths** — only the active task, execution/handoff rules, task-specific contracts, current PR, and relevant failure evidence.
6. **Policy v2** — `task_kind`, `context_pressure`, `decomposition_decision`, and `execution_mode`.
7. **Run scope** — `run_scope`, continuation policy, task-completion policy, and low-noise communication policy.
8. **Execution procedure** — verify, perform minimal discovery, produce one coherent result, validate, persist state, and reassess the next action.
9. **Acceptance and validation** — focused, component, and heavy-final checks.
10. **Durable state** — checkpoint updates after material discoveries, changes, validation, blockers, head/PR changes, and before risky operations or rotation.
11. **Stop conditions** — only real stop conditions, not routine milestones.
12. **Final response contract** — compact status, completed work, validation, durable state, blocker, and exactly one next action when work remains.

## Autonomous continuation contract

For `run_scope: autonomous_program`, the coordinator must:

1. inspect the live programme, control room, wave, barriers, tasks, PRs, CI, and ownership;
2. select the highest-priority safe `READY` task or bounded set of non-overlapping tasks;
3. execute or dispatch the current bounded phase using the cheapest capable mode;
4. checkpoint after material milestones and before risky operations;
5. continue the same task immediately when its next phase is ready and context remains safe;
6. when the task is terminal, satisfy its close/merge gate, write the final checkpoint, archive or move the task according to repository convention, release ownership/lease, and update programme state;
7. perform a barrier review and immediately select the next `READY` task;
8. continue until a real stop condition is reached.

When one task is `WAITING`, persist that state and work on another independent `READY` task instead of keeping a session open merely to wait. Return only when no safe ready work remains or another real stop condition applies.

## Real stop conditions

An autonomous programme run stops only when one of these is true:

- all currently authorized programme work is complete;
- no task is `READY` and remaining work is genuinely `WAITING` or `BLOCKED`;
- a material product, architecture, safety, authorization, or acceptance decision requires the owner;
- ownership overlaps cannot be resolved safely;
- a repository or production safety boundary forbids continuation;
- required external evidence or an observation window is not yet available and no independent ready work remains;
- context pressure, tool limits, or execution-environment limits make further work unsafe;
- two heavy attempts failed and the defect must first be isolated in a new bounded phase.

Do not stop merely because a phase completed, a commit was created, a PR was opened, CI became green, a PR merged, a task was archived, or a checkpoint was written.

## Validation policy

Use staged validation:

```text
Focused: changed-file checks, unit tests, type/contract checks, or minimal reproduction
Component: relevant package, component build, or bounded integration suite
Heavy final gate: full build, E2E, regression suite, or matrix, normally once after coherent implementation
```

After a heavy failure, isolate the first relevant error and reproduce it cheaply before another full run. A worker session normally performs no more than two heavy attempts. The coordinator may rotate to a fresh session and continue the same task after durable state is safe.

## Durable state, completion, and archival

The checkpoint preserves `PROVEN`, `DERIVED`, `UNKNOWN`, and `CONFLICT` evidence; branch/head/PR; changed paths; validation; first relevant failure; blockers; and exactly one concrete `next_action` while work remains.

Full logs, screenshots, traces, SQL snapshots, binaries, and large reports belong in artifacts or an evidence index, not in prompts or checkpoints.

A checkpoint is a recovery boundary, not a mandatory pause. Continue after writing it when the next action is safe.

When a task is terminal:

- record final result and exact validation evidence;
- set terminal status according to repository conventions;
- merge or close the PR only when repository policy authorizes it;
- move/archive the active task record when required;
- release branch/worktree lease and owned paths;
- update the programme/coordinator checkpoint and barrier state;
- select the next ready task without asking for routine confirmation.

## Low-noise communication

For autonomous runs:

- do not narrate routine reads, searches, commands, unchanged checks, or every checkpoint;
- do not ask questions that live state can answer;
- send at most compact updates for material milestones, real blockers, required owner decisions, or material changes in risk/scope;
- avoid chronological diaries and walls of text;
- provide one compact final report only when the run actually stops.

## Specialized rules

### Audit

Define the boundary and severity model. Default to no implementation. Require severity, confidence, evidence, impact, and recommendation. Unrelated remediation remains a recommendation or a later task.

### E2E

Define start state, fixtures, sequence, and observable acceptance. Keep shared-state steps in one task. Store bulky evidence as artifacts. Separate platform repair from feature validation when ownership differs.

### Implementation

State the invariant, compatibility constraints, owned paths, and focused tests. Material architecture decisions remain with the coordinator unless evidence proves a required change.

### CI repair

Name the exact PR head, workflow, job, and first relevant error. Reproduce cheaply first and verify the exact final head. Do not rerun the whole suite after every edit.

### Independent validation

Prefer a fresh validator session on the same task. Forbid implementation unless a tightly bounded proven defect is explicitly authorized.

### Stale recovery

Verify that no previous worker is still writing. Inspect task, branch, PR, commits, checks, and ownership. Repair the checkpoint before substantive work and continue from the last coherent commit with a new `session_id`.

## Base template

```text
ROLE
You are the <role> for task <TASK_ID>, phase: <PHASE>.

REPOSITORY AND LIVE STATE
Repository: <owner/repo>
Programme/coordinator task: <path or none>
Task record: <path>
Expected branch: <branch>
Expected PR: <number or none>
Verify the live checkpoint, exact head, PR, required CI, dependencies, barrier state, and path ownership before changing state. Durable repository state overrides chat history.

OBJECTIVE
<One outcome-oriented invariant.>

AUTHORIZATION AND SCOPE
<Authorization.>
Owned paths: <paths or require a claim before editing>
Do not merge, deploy, change unrelated contracts, or expand scope unless repository policy and the task explicitly authorize it.

POLICY
policy_version: 2
task_kind: <kind>
context_pressure: <low|medium|high|unbounded>
decomposition_decision: <single|phased|split|discovery_first>
execution_mode: <chat|codex|work>
run_scope: <single_task|autonomous_program>
continuation_policy: <stop_at_task_boundary|continue_until_real_stop>
task_completion_policy: <checkpoint_only|finalize_archive_and_continue>
user_communication: low_noise

REQUIRED READS
- <active task record>
- docs/agents/EXECUTION_PROTOCOL.md
- docs/agents/CONTEXT_HANDOFF.md
- docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md when run_scope is autonomous_program
- <smallest task-specific contracts>

EXECUTION
1. Verify live state, ownership, barriers, and the current next action.
2. Perform only the discovery needed for this phase.
3. Complete one coherent change or evidence package.
4. Run focused validation before broader validation.
5. Persist coherent commits and compact checkpoints after material milestones and before long or failure-prone operations.
6. Run the heavy final gate only when the coherent result is ready.
7. If the task is terminal, finalize, archive, release ownership, update programme state, review the barrier, and continue with the next READY task when run_scope is autonomous_program.

ACCEPTANCE AND VALIDATION
Acceptance: <criteria>
Focused: <check>
Component: <check or not required>
Heavy final gate: <check or not required>
After a heavy failure, reproduce the first relevant error cheaply before rerunning. Do not exceed two heavy attempts in one worker session.

STOP CONDITIONS
Stop only for a real blocker, waiting state with no other ready work, required owner decision, safety/ownership conflict, completion of all authorized work, or unsafe context/tool limits. Do not stop at routine milestones.

FINAL RESPONSE
STATUS: DONE | BLOCKED | WAITING | ROTATE
RESULT: <compact result covering all work completed in this owner invocation>
VALIDATION: <checks and outcomes>
DURABLE_STATE: <programme/task paths, branches, heads, PRs>
BLOCKER: <none or exact blocker>
NEXT_ACTION: <one action or none when fully done>
```

## Quality gate

Before presenting or executing a prompt, confirm:

- one objective per task and explicit live-state verification;
- clear authorization and non-overlapping ownership;
- justified task shape and cheapest capable execution mode;
- explicit run scope and continuation semantics;
- focused/component/heavy validation;
- checkpoint, completion, archival, and barrier rules;
- real stop conditions rather than milestone stops;
- low-noise user communication;
- compact final response only when the autonomous run stops.

Reject unbounded remediation, unsafe background claims, repeated polling, unnecessary full logs, owner micromanagement of routine phases, and prompts that force a return after every completed task despite safe ready work remaining.
