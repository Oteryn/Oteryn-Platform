# OTERYN CI + Merge Queue Canary V2

Governing Issue: #1399.

Status: `PREPARED / LIVE_EXECUTION_NOT_STARTED`.

This runbook supersedes the V1 execution package from #1268/#1269. Historical V1 evidence remains historical only.

## Objective

Measure current GitHub behavior across exactly:

- `Oteryn/Oteryn` (META)
- `Oteryn/Oteryn-Platform`
- `Oteryn/Oteryn-Game`
- `Oteryn/Oteryn-Atlas`

V2 measures two independent outcomes:

1. CI routing/economy: expected routing, duplicate generations, supersession, loops, runner contention and disproportionate work.
2. Merge Queue autonomy: whether an already-authorized exact-head PR can be submitted without manual GitHub UI enqueue, receives a real `merge_group`, passes the required queue gates, and is then merged by GitHub without a second integration mutation.

## Merge Queue result model

Never collapse the integration lifecycle into a single `auto-merge` flag.

Record separately:

- `ELIGIBLE`: exact-head requirements and exact integration authorization are satisfied.
- `AUTO_ENQUEUE`: the coordinator successfully invokes a currently proven governed queue route without manual UI enqueue.
- `MERGE_GROUP_PROVEN`: GitHub creates the real synthetic queue candidate and the required/source workflows execute against the exact `merge_group` SHA.
- `AUTO_MERGE_AFTER_ENQUEUE`: after accepted enqueue no second integration mutation occurs and GitHub merges only after queue success, followed by protected-main readback.

`TASK_SELF_INTEGRATION=PASS` requires all four stages and proof that no direct merge, generic auto-merge, bypass, force/rebase, no-op/retrigger commit or protection weakening occurred.

A task is not allowed to merge itself without integration authority. V2 tests autonomous execution after exact authorization, not removal of the owner/governance boundary.

## Current governed route

The selected native route is exact-head `merge-async` with `merge_action=merge_queue` under the current META policy. A directly capable coordinator may use the native route. A coordinator without direct capability may use the protected META delegated executor only when current capability classification proves `DELEGATED_CAPABLE` for the exact canary-qualified protected META generation.

The executor receipt is non-terminal. Terminal proof still requires real `merge_group`, required aggregate/source workflow success, merged PR state and protected-main readback.

## Existing evidence reuse

Prefer recent exact attributable real evidence over new mergeable canaries.

Known current-generation evidence to revalidate before generating new MQ probes:

- META #199: real META Merge Queue integration.
- Platform #1392: accepted delegated `merge-async` receipt/UUID, real Platform `merge_group`, `platform-gate` success, merged state and protected-main readback on META `23b21e9b1b2d4b6c3a5cac3d4c7a18747804c090`.
- Game #606: merged current Game candidate with real Game `merge_group` gate success; bind exact governed enqueue/receipt causally before crediting `AUTO_ENQUEUE`.
- Atlas #494: merged current Atlas candidate with real merge-group selective-verification/protected-authority runs; bind exact governed enqueue/receipt causally before crediting `AUTO_ENQUEUE`.

Do not generate another mergeable canary merely to reproduce an already-proven stage for the same control-plane generation.

## Per-repository execution gate

Freeze each repository independently. A repository may enter live V2 only when all are true:

1. exact protected `main` SHA captured;
2. current protection/rulesets, required checks and Merge Queue settings captured;
3. complete PR-capable workflow inventory and trigger/downstream graph captured;
4. no open PR/canary is changing the CI/planner/ruleset/runner semantics being measured unless the owner explicitly freezes that exact generation as baseline;
5. no existing canary already exercises the same measurement surface;
6. exact `EXPECTED / ALLOWED_SKIP / FORBIDDEN` matrix generated from current workflow bytes;
7. selected probe cannot reach production writes, protected environments, destructive operations or ambiguous external effects;
8. any probe expected to integrate autonomously has fresh trusted capability state: `DIRECT_CAPABLE`, `DELEGATED_CAPABLE`, or a precise blocker.

Do not wait for all product work to stop. Freeze one repository at a time.

Preferred order after clean preflight: `META -> Platform -> Game -> Atlas`.

## Minimal probe policy

Do not recreate V1's nine probes automatically.

Per repository start with at most:

- `R0`: inert docs/control probe for global/lightweight routing;
- `R1`: one representative selective/code-path probe chosen from current workflow inventory.

Add `R2` heavy-lane probe only when static inventory plus existing real PR evidence cannot answer a material question.

Routing probes stay Draft, never merge, never deploy and are closed after evidence capture.

META currently needs only `R0` unless its workflow topology changes, because the current PR CI is global rather than path-selective.

## Phases

### A. Freeze and matrix

Record exact main, workflow blobs, PR event actions/path filters, reusable/downstream calls, runner labels/groups, concurrency/cancel semantics, environments/permissions, required checks, Merge Queue settings and overlapping control-plane work.

### B. Serial routing baseline

Run `R0`, wait for all caused runs terminal and persist evidence. Then run `R1`; run `R2` only if justified. Do not overlap repositories in the primary baseline.

### C. Supersession

On `R1`, publish two successor generations while supersedable work is active. Measure cancellation only where the frozen workflow contract declares supersession behavior.

### D. Metadata-only event

Edit only the `R0` PR body. Head SHA must remain unchanged. Any resulting workflow run is judged against the exact event-action contract.

### E. Controlled cross-repository burst

Only after serial evidence is terminal, update comparable `R1` probes in one bounded batch and compare queue delay/runtime against serial evidence.

### F. Merge Queue autonomy

First satisfy the four-stage MQ result model from existing real evidence. Only missing stages justify a new inert integration canary.

For any new MQ canary:

1. create the smallest repository-allowed inert change suitable for eventual merge;
2. qualify exact head normally;
3. obtain exact current integration authorization;
4. perform no manual UI enqueue or merge;
5. invoke exactly one currently proven governed queue route;
6. preserve request/receipt identity and UUID where supplied;
7. observe exact `merge_group` SHA and required/source workflow results;
8. after accepted enqueue perform no second integration mutation;
9. prove GitHub merged only after queue success;
10. read back protected `main` containing the candidate.

A queue receipt alone is not PASS. A merged PR without attributable governed enqueue plus real `merge_group` is not PASS.

## Authorization-hold negative control

At least one exact-head GREEN candidate in the campaign must remain without integration authorization for a bounded observation window.

Expected state: open + unqueued + unmerged.

Any unexpected enqueue, `merge_group`, merge or source-head mutation is immediate `FAIL_UNAUTHORIZED_INTEGRATION` and stops the affected repository campaign.

## Moving-base / anti-loop control

When protected `main` advances while a candidate remains open:

- PR head must remain unchanged unless there is a real task edit;
- no bot merge-up/rebase/no-op commit solely because `main` moved;
- no same-head synthetic retrigger loop;
- later authorized queue qualification must use the synthetic current-base `merge_group`, not source-branch chasing.

Unrelated main movement is `UPSTREAM_ADVANCED + MATRIX_VALID` unless it changes routing, workflow, runner, protection or relevant governance semantics.

## Evidence schema

For every intentional stimulus record repository, canary ID, PR, action, generation, exact head/base SHA and timestamp.

For every workflow run record workflow path/name, run ID/attempt, event, exact candidate SHA, status/conclusion, job IDs/names/conclusions, runner identity/labels when observable, timestamps, expected-state classification and causal stimulus.

For Merge Queue additionally record:

- authorization/freeze locator;
- capability decision/evidence generation;
- submission route (`DIRECT` or `DELEGATED`);
- exact requested head and PR;
- request/comment identity if delegated;
- accepted UUID/receipt and readback state where available;
- exact `merge_group` SHA;
- required/source workflow run/check IDs on that SHA;
- merged PR state and integration SHA;
- protected-main readback;
- manual integration actions after authorization, which must be zero for `TASK_SELF_INTEGRATION=PASS`.

## Verdicts

Per repository:

- `PASS`
- `PASS_WITH_OPTIMIZATION`
- `FAIL`
- `WAIT`
- `BLOCKED`
- `UNKNOWN`

Global conclusions are separate for:

- `CI_ROUTING_HEALTH`
- `AUTO_ENQUEUE`
- `MERGE_GROUP_INTEGRITY`
- `AUTO_MERGE_AFTER_ENQUEUE`
- `TASK_SELF_INTEGRATION`

Never convert insufficient evidence to PASS.

## Immediate abort

Stop new stimuli for the affected repository on unexpected production/protected-environment effects, self-sustaining amplification, unexplained duplicate generations consuming capacity, ambiguous runner ownership, relevant baseline invalidation, unexpected merge, unauthorized queueing, inability to bind evidence to exact SHA/run, or fallback around required protection.

## Current preparation snapshot

Informative only; refresh before execution:

- META `23b21e9b1b2d4b6c3a5cac3d4c7a18747804c090`
- Platform `aac113f2cc878847205e51e95c87ad86766ce818`
- Game `43f0128f7e11253cca19b40b5cc3703b46b35916`
- Atlas `670f2d9945191fb146e423660cfc3ae082142af2`

Current admission status at preparation time:

- META: `PREP_READY`.
- Platform: `WAIT` while #1398 changes `.github/workflows/codeql.yml`.
- Game: `WAIT` while Draft #592 changes workflow/control-plane behavior.
- Atlas: `WAIT` while #492/#493 occupy the verification/MQ canary surface.

No live V2 canary is authorized merely by this runbook.