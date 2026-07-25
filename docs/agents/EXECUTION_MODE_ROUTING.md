# Oteryn Platform Execution Mode Routing

## Purpose

This document selects the least expensive execution mode that can complete the next concrete action safely and with verifiable evidence.

Mode selection is an execution-planning decision only. It does not grant repository access, infrastructure access, production authorization, permission to modify another repository or permission to weaken a required control.

The repository allowlist, `AGENTS.md`, the active task checkpoint, security boundaries and mandatory stop conditions always take precedence.

## Supported modes

### `CHAT`

Use `CHAT` when the next action can be completed through available read or connector-backed operations without a local repository build/runtime environment.

Typical work:

- inspect repository files, commits, issues, pull requests and workflow evidence;
- reconcile task state against trusted Git and GitHub state;
- create or update bounded documentation, task records, issues and pull-request metadata when the connector supports the required write;
- review diffs, comments, CI status and workflow logs;
- coordinate already-separated child tasks and pull requests;
- perform repository-backed planning that does not substitute for required implementation;
- merge a pull request only after the repository merge gate is independently proven.

`CHAT` must not claim:

- local commands were executed when no local checkout exists;
- application tests, migrations, browser acceptance or container builds passed without exact external evidence;
- staging or production deployment occurred merely because a workflow exists;
- an unavailable write or workflow action will be performed later in the background.

### `CODEX`

Use `CODEX` when the next action requires a writable local checkout or direct repository execution.

Typical work:

- modify application code, routes, migrations, tests, assets or workflows across several related files;
- inspect the working tree, branches, worktrees and generated diffs locally;
- install dependencies using repository-approved commands;
- run formatting, static analysis, focused tests and complete test suites;
- execute migration upgrade and rollback validation;
- run browser acceptance, visual checks or local runtime smoke;
- diagnose failures that require reproduction or iterative code changes;
- build images or deployment artifacts when the task and environment authorize them.

Select `CODEX` only when the required local execution capability is actually available. If it is unavailable, record the operation as `BLOCKED`; do not represent connector-only inspection as local execution.

### `WORK`

Use `WORK` for a broad deliverable that requires sustained coordination across many independent evidence sources or bounded workstreams and cannot be handled efficiently as one repository-execution slice.

Typical work:

- large multi-source research or architecture reconciliation;
- programme-level coordination across multiple child tasks and pull requests;
- a large document or evidence packet assembled from several independent systems;
- cross-repository compatibility analysis where all external repositories remain within their explicit read/write authorization boundaries;
- release or programme closure requiring several separately validated implementation packages.

`WORK` does not override task isolation. Runtime changes must still be delivered through bounded branches, task records and pull requests. When a concrete child action requires local repository execution, route that action to `CODEX` rather than treating `WORK` as a substitute for a writable checkout.

## Routing fields

Task prompts and handoffs may declare:

```text
PROGRAM: <programme name or none>
RECOMMENDED_MODE: CHAT | CODEX | WORK
BUDGET_POLICY: <execution-cost preference>
MODE_CONFIDENCE: high | medium | low
MODE_REASON: <why the recommended mode fits the next action>
MODE_ESCALATION: <the exact condition requiring a higher mode>
MODE_RETURN: <the condition for returning to the lower mode>
```

Field rules:

- `PROGRAM` identifies coordination context; it does not expand scope.
- `RECOMMENDED_MODE` is advisory until current capabilities and the next action are verified.
- `BUDGET_POLICY` may prefer a cheaper mode but must not skip required execution or validation.
- `MODE_CONFIDENCE` describes routing confidence, not implementation confidence.
- `MODE_REASON` must reference the actual operation required next.
- `MODE_ESCALATION` must name a concrete capability boundary, not a vague preference.
- `MODE_RETURN` must identify when the higher-cost capability is no longer needed.

## Deterministic selection procedure

Apply these steps in order.

### 1. Verify authorization and stop conditions

Before selecting a mode:

- verify the repository write allowlist;
- verify active task ownership and overlapping pull requests;
- verify production, infrastructure and external-repository authorization;
- identify mandatory reads and unresolved security or compatibility assumptions;
- stop if a mandatory repository rule requires a decision or evidence that is unavailable.

A mode must never be selected to bypass a stop condition.

### 2. Identify exactly one next action

Use the active checkpoint when one exists. The next action must be concrete enough to classify, for example:

- inspect PR status;
- create a task record;
- edit Laravel routes and tests;
- reproduce a migration failure;
- coordinate three already-bounded child PRs;
- dispatch an existing reviewed deployment workflow.

Do not route an entire programme as one indivisible implementation action when bounded child tasks are required.

### 3. Select the lowest sufficient mode

Use this order:

1. `CHAT` when available connector/read operations can complete the next action and produce sufficient evidence.
2. `CODEX` when local editing, building, testing, migration, browser or runtime execution is necessary.
3. `WORK` when the deliverable is primarily broad coordination or multi-source synthesis beyond one bounded repository slice.

A higher mode is not automatically better. Use the lowest mode that can truthfully complete the action.

### 4. Verify capability availability

Before acting, confirm that the selected mode has the required capabilities now.

Examples:

- no mounted checkout or shell access means local test execution is unavailable;
- no workflow-dispatch operation means deployment dispatch is unavailable even if the workflow file can be read;
- no authorized environment access means staging or production smoke cannot be claimed;
- connector write support may allow a documentation file or pull request to be created in `CHAT`.

Unavailable capability produces `BLOCKED`, not simulated success.

### 5. Execute and record evidence

Record exact evidence appropriate to the action:

- branch, head and PR number;
- changed paths;
- command and result;
- workflow, run and job identifiers;
- deployment SHA and sanitized health evidence;
- `PROVEN`, `DERIVED`, `UNKNOWN` and `CONFLICT` facts.

Mode labels are never evidence by themselves.

## Escalation rules

Escalate only when the current next action cannot be completed truthfully in the current mode.

### `CHAT` to `CODEX`

Escalate when any of these becomes necessary:

- local multi-file implementation;
- dependency or lockfile changes;
- migration creation or rollback testing;
- local formatter, static-analysis or test execution;
- browser, visual or runtime reproduction;
- container/image construction;
- failure diagnosis requiring iterative local patches.

Do not escalate solely because a task is important or large.

### `CHAT` or `CODEX` to `WORK`

Escalate when the primary remaining problem is broad coordination or synthesis across several bounded tasks, repositories or evidence systems.

Do not use `WORK` to avoid creating child tasks or to combine conflicting path ownership into one branch.

### No escalation past authorization

Escalation cannot authorize:

- writes outside the current repository allowlist;
- production changes without explicit authorization;
- router, DSM, Internet-exposure or secret-management changes outside scope;
- weakening CSRF, MFA, audit, validation, static analysis or tests;
- destructive migrations without the required rollback and data-impact evidence.

Stop and report the blocker instead.

## Return rules

Return to the lower sufficient mode as soon as the higher capability is no longer required.

Examples:

- after a `CODEX` implementation is pushed and only PR/CI inspection remains, return to `CHAT`;
- after `WORK` decomposes a programme into bounded child tasks, execute each repository slice in `CHAT` or `CODEX` as appropriate;
- after CI identifies a code defect, escalate to `CODEX`; after the fix is pushed, return to `CHAT` for workflow observation;
- after all child PRs merge, use `WORK` only for genuine programme reconciliation, then return to `CHAT` for issue and PR metadata operations.

## Repository-specific examples

| Next action | Recommended mode | Reason |
|---|---|---|
| Check the state of a PR or workflow | `CHAT` | GitHub evidence is sufficient. |
| Reconcile an issue checklist against current source | `CHAT` | Repository reads and exact merged evidence are sufficient. |
| Create a bounded task record and draft PR through the connector | `CHAT` | No local build/runtime is required. |
| Implement public Wiki controllers, routes, migrations, views and tests | `CODEX` | Multi-file local implementation and validation are required. |
| Diagnose a failing PHPStan or browser acceptance job | `CODEX` | Reproduction and iterative patches are required. |
| Coordinate several independent public-website child PRs | `WORK` | The primary action is programme-level coordination. |
| Dispatch a reviewed staging workflow | `CHAT` when the dispatch operation and authorization are available; otherwise `BLOCKED` | Reading the workflow alone does not prove dispatch capability. |
| Verify a live staging website | `CODEX` or another explicitly available runtime/browser execution environment | Live requests and browser evidence are required. |

## Budget policy

When a prompt specifies `BUDGET_POLICY: minimize_agentic_usage`:

- prefer `CHAT` for discovery, reconciliation, documentation, PR and CI work;
- escalate to `CODEX` only at the first action that genuinely requires local edit/build/test/runtime execution;
- escalate to `WORK` only for broad multi-source coordination or a large deliverable that cannot be reduced to one bounded slice;
- return to `CHAT` immediately after higher-mode execution is complete.

Budget policy must never suppress a required test, security check, migration validation or live deployment proof.

## Failure and uncertainty handling

- Use `I do not know` or `UNKNOWN` when evidence is absent.
- Use `CONFLICT` when authoritative sources disagree.
- Use `BLOCKED` when the selected capability, permission, secret or mandatory decision is unavailable.
- Do not replace missing evidence with an assumption.
- Do not claim asynchronous or background execution.
- Do not claim completion from a plan, task record, draft PR, mode selection or workflow definition alone.

## Relationship to other governance documents

- `AGENTS.md` defines repository allowlists, delivery policy, security and merge gates.
- `CONTEXT_ROUTING.md` determines which repository context must be loaded for the task domain.
- `BUILD_TEST_MATRIX.md` determines proportional validation for changed paths.
- `CONTEXT_HANDOFF.md` defines durable checkpoint and evidence-state rules.
- This document determines which execution capability is appropriate for the single next action.

When these documents appear to conflict, follow the more restrictive safety and evidence requirement and record the conflict in the active checkpoint.