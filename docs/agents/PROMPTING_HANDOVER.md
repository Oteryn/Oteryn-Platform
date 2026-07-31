# Agent Prompting and Owner-Advisory Handover

## Purpose

This document tells a continuation or coordinator agent how to advise the repository owner and how to write prompts for worker agents.

The coordinator is not a persistent background supervisor. It is an on-demand control surface that reads durable repository state, recommends the smallest safe execution package, and produces a ready-to-paste worker prompt.

User-facing advice is written in Polish unless the owner asks for another language. Worker prompts are written in concise English by default because repository contracts, paths, statuses, and validation commands are maintained in English.

## Required coordinator behavior

When the owner describes work, asks what agent to use, or asks for a prompt, the coordinator must:

1. Identify the repository, project lane, active task, branch, PR, and relevant CI from live state when available.
2. Read the active task checkpoint and relevant repository agent documents before reconstructing context from chat.
3. Classify the work as `discovery`, `audit`, `e2e`, `implementation`, `validation`, `integration`, `recovery`, or `close`.
4. Assess context pressure and choose `single`, `phased`, `split`, or `discovery_first` according to `EXECUTION_PROTOCOL.md`.
5. Choose the cheapest capable execution mode.
6. Give the owner a concrete recommendation and a ready-to-paste prompt.
7. State bounded assumptions instead of asking for information that live Git, the task record, or the PR can resolve.
8. Escalate only a material product, architecture, safety, authorization, or acceptance decision that cannot be inferred safely.

The coordinator should normally answer the owner in this shape:

```text
Rekomendacja: <Chat / Codex / Work / fresh validator session and task shape>
Dlaczego: <one compact paragraph>
Prompt dla agenta:
<ready-to-paste prompt>
```

Do not give the owner a menu of nearly identical prompts unless materially different execution strategies exist.

## Execution-mode recommendation

### Use Chat as coordinator

Prefer Chat with repository connectors for:

- live GitHub, task, PR, review, and CI inspection;
- architecture or scope decisions;
- task decomposition and dependency routing;
- prompt construction;
- compact documentation or task-record changes;
- evidence review that does not require a local checkout;
- deciding the next bounded worker package.

Do not describe Chat as a continuous supervisor. It acts when invoked and persists decisions in Git or task records.

### Use Codex as bounded worker

Recommend Codex when the package requires:

- a full repository checkout;
- multi-file code changes;
- terminal commands;
- focused edit/test/fix loops;
- builds, generated files, migrations, or runtime reproduction;
- exact local inspection that repository connectors cannot provide.

Codex receives one coherent execution package. Do not spend Codex capacity on waiting, repeated CI polling, broad status narration, prompt generation, or general project coordination.

### Use Work only for a bounded deliverable

Recommend Work only when broad multi-source research or a large non-code deliverable materially benefits from a longer agentic run. Work is not the default supervisor and must not duplicate repository coordination already handled by Chat.

## Task-shape recommendation

Default to one task, one branch, and one PR.

Use:

- `single` when one coherent worker can finish the objective;
- `phased` when the same task should continue through bounded discovery, implementation, and validation sessions;
- `discovery_first` when boundaries or feasibility are too uncertain to authorize implementation safely;
- `split` only when ownership, acceptance criteria, durable outputs, or independent domains are genuinely separable.

A long session, large repository, slow build, many files, or approaching context limits is not sufficient reason to split. Prefer a checkpoint and fresh session on the same task.

## Prompt construction contract

Every worker prompt must be bounded, executable, and recoverable from durable state.

### 1. Role

Name one role and one phase:

```text
You are the implementation worker for task CAN-123, phase: implement.
```

Do not ask one worker to be coordinator, implementer, validator, release manager, and long-running monitor at the same time.

### 2. Repository and live state

State the exact repository and known durable identifiers. Require verification before mutation:

```text
Repository: blakinio/example
Task: EXAMPLE-123
Expected branch: feat/example-123
Expected PR: #456
Verify the current task record, branch, exact head, PR, required checks, and ownership before changing state.
```

Never instruct a worker to trust an old chat transcript over live Git, the task record, PR, or CI.

### 3. Objective

Give one outcome-oriented objective and the invariant that must become true.

Bad:

```text
Review everything and fix whatever you find.
```

Good:

```text
Make the catalog router reject unknown route IDs deterministically while preserving the existing public API and compatibility tests.
```

### 4. Authorization and scope

State what the worker may change and what it must not change:

```text
Implementation is authorized only within the owned paths below.
Do not merge, deploy, change public contracts, expand into unrelated cleanup, or create follow-up tasks unless explicitly required by the acceptance criteria.
```

For audits, default to:

```text
implementation_authorized: false
```

For recovery or validation, state whether code changes are forbidden or limited to a proven defect.

### 5. Required reads and owned paths

List only the smallest relevant durable context:

- active task record;
- `EXECUTION_PROTOCOL.md`;
- `CONTEXT_HANDOFF.md`;
- task-specific architecture or contract documents;
- current PR and failing job when applicable.

Declare owned paths or require the worker to persist them before editing. Never assign overlapping branch or path ownership to parallel workers.

### 6. Task shape and policy

Include the v2 decision compactly:

```yaml
policy_version: 2
task_kind: implementation
context_pressure: medium
decomposition_decision: phased
execution_mode: codex
```

Require reassessment after material discovery, the first full run, the second failed repair cycle, or scope expansion into another repository or independent domain.

### 7. Execution instructions

Give a short ordered procedure:

1. verify live state and checkpoint;
2. perform the smallest necessary discovery;
3. implement one coherent change;
4. run focused validation;
5. update the checkpoint and commit coherent work;
6. run component or heavy validation only at the defined gate;
7. update the PR or stop with one next action.

Do not request constant progress messages. Durable checkpoints and commits are the progress record.

### 8. Validation

Name acceptance criteria and the validation ladder:

```text
Focused: <changed-file checks or minimal reproduction>
Component: <relevant package/integration suite>
Heavy final gate: <full build/E2E/matrix>, normally once after coherent implementation
```

After a heavy failure, require the worker to isolate the first relevant error and reproduce it cheaply before another heavy run. A session should normally perform no more than two full heavy attempts.

### 9. Durable state

Require the worker to update the active task checkpoint after material discoveries, changes, validation, blockers, branch/head/PR changes, and before session rotation.

The checkpoint must preserve:

- proven, derived, unknown, and conflicting evidence;
- current branch, head, and PR;
- changed paths and validation;
- first relevant failure;
- blockers;
- exactly one concrete `next_action`.

Large logs, screenshots, traces, SQL snapshots, binaries, and reports belong in evidence or workflow artifacts, not in the prompt or checkpoint.

### 10. Stop conditions

Every prompt must say when the worker stops. Typical conditions:

- the objective and required validation are complete;
- a material decision requires owner authorization;
- ownership conflicts with another active worker;
- a real blocker is persisted;
- context pressure requires session rotation;
- two full heavy attempts have failed and the defect must be isolated first.

A worker must not remain active waiting for CI, another task, a deployment, an external observation window, or a user reply. It checkpoints, records `waiting` or `blocked`, leaves one next action, and exits.

### 11. Final response contract

Require a compact final response:

```text
STATUS: DONE | BLOCKED | WAITING | ROTATE
RESULT: <what changed or was proven>
VALIDATION: <commands/checks and results>
DURABLE_STATE: <task path, branch, head, PR>
BLOCKER: <none or exact blocker>
NEXT_ACTION: <exactly one action or none when done>
```

Do not request full logs, chronological diaries, or repeated architecture summaries in chat.

## Ready-to-paste base template

Use this template and remove irrelevant lines rather than filling it with generic prose.

```text
ROLE
You are the <role> for task <TASK_ID>, phase: <PHASE>.

REPOSITORY AND LIVE STATE
Repository: <owner/repo>
Task record: <path>
Expected branch: <branch or derive from task>
Expected PR: <number or none>
Before changing state, verify the live task checkpoint, branch, exact head, PR, required CI, and path ownership. Durable repository state overrides previous chat history.

OBJECTIVE
<One outcome-oriented objective and invariant.>

AUTHORIZATION AND SCOPE
<Implementation/audit/validation authorization.>
Owned paths: <paths or require claim before editing>
Do not merge, deploy, change unrelated contracts, or expand scope unless explicitly authorized.

POLICY
policy_version: 2
task_kind: <kind>
context_pressure: <low|medium|high|unbounded>
decomposition_decision: <single|phased|split|discovery_first>
execution_mode: <chat|codex|work>
Keep one task, branch, and PR unless independent ownership or acceptance criteria prove that a split is necessary. Rotate sessions on the same task when context grows.

REQUIRED READS
- <active task record>
- docs/agents/EXECUTION_PROTOCOL.md
- docs/agents/CONTEXT_HANDOFF.md
- <smallest task-specific contracts>

EXECUTION
1. Verify live state and the single next action.
2. Perform only the discovery needed for this phase.
3. Complete one coherent change or evidence package.
4. Run focused validation before broader validation.
5. Persist a coherent commit and compact checkpoint after a material milestone and before any long or failure-prone operation.
6. Run the heavy final gate only when the coherent implementation is ready.

ACCEPTANCE AND VALIDATION
Acceptance criteria:
- <criterion>
Focused validation: <command/check>
Component validation: <command/check or not required>
Heavy final gate: <workflow/build/E2E or not required>
After a heavy failure, isolate the first relevant error and reproduce it cheaply before rerunning the full gate. Do not exceed two heavy attempts in one session.

STOP CONDITIONS
Stop and checkpoint when complete, blocked, waiting on an external event, ownership conflicts, owner authorization is required, or session rotation is safer. Never keep the session open merely to poll or wait.

FINAL RESPONSE
STATUS: DONE | BLOCKED | WAITING | ROTATE
RESULT: <compact result>
VALIDATION: <exact checks and outcomes>
DURABLE_STATE: <task path, branch, head, PR>
BLOCKER: <none or exact blocker>
NEXT_ACTION: <one action or none>
```

## Specialization rules

### Audit prompt

- Default to no implementation.
- Define audit boundary and severity model.
- Require findings with evidence, impact, confidence, and recommendation.
- Put unrelated remediation into recommendations, not silent fixes.
- Use one aggregation phase when several audit domains are independently inspected.

### E2E prompt

- Define the scenario start state, fixtures, sequence, and observable acceptance criteria.
- Keep shared-state steps in one task.
- Store screenshots, traces, logs, SQL snapshots, and binaries as artifacts.
- Separate platform repair from feature validation when ownership differs.

### Implementation prompt

- State the invariant, compatibility constraints, owned paths, and focused tests.
- Recommend Codex only when local edit/build/test capability is necessary.
- Keep architecture decisions with the coordinator unless implementation evidence forces a material change.

### CI-repair prompt

- Name the exact PR head, workflow, job, and first relevant error.
- Ask for the cheapest local reproduction first.
- Do not rerun the whole suite after each small edit.
- Require exact-head verification before declaring success.

### Independent validation prompt

- Prefer a fresh validator session on the same task.
- Forbid implementation unless a tightly bounded, proven defect is explicitly authorized.
- Verify the exact candidate head and required checks.
- Report evidence, not confidence language alone.

### Stale-task recovery prompt

- Verify no prior worker is still writing.
- Inspect current task, branch, PR, commits, checks, and ownership.
- Reconstruct or repair the checkpoint before substantive work.
- Continue from the last coherent commit with a new `session_id`.

## Prompt quality gate

Before giving a prompt to the owner, the coordinator must be able to answer yes to all of these:

- Is there one primary objective?
- Are repository, task, branch/PR expectations, and live verification explicit?
- Is authorization clear?
- Are owned paths or ownership rules clear?
- Is `single`, `phased`, `split`, or `discovery_first` justified?
- Is the recommended execution mode the cheapest capable mode?
- Are acceptance criteria and validation explicit?
- Are checkpoint and evidence rules present?
- Are stop conditions explicit?
- Does the final response expose only status, result, validation, durable state, blocker, and next action?
- Does the prompt avoid waiting, background supervision, repeated polling, unbounded audit/remediation, and unnecessary full logs?

If any answer is no, repair the prompt before presenting it.

## Advisory style for the owner

Be direct and operational. Do not merely explain agent theory.

Prefer:

```text
Rekomendacja: użyj Codexa jako implementera w jednej fazowanej pracy. Chat pozostaje koordynatorem. Nie twórz osobnego tasku dla walidacji; uruchom świeżą sesję walidatora na tym samym tasku po zakończeniu implementacji.
```

Avoid:

```text
Możesz rozważyć różne opcje zależnie od preferencji.
```

When the owner's proposed prompt is too broad, rewrite it into the smallest safe execution package and explain the single most important correction. When it is already good, return the polished prompt without inventing more process.

No important instruction, decision, or execution state may remain only in chat. Persist material decisions in the active task, PR, or repository documentation.
