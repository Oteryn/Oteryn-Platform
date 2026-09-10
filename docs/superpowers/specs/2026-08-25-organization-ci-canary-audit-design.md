# Organization CI Canary Audit Design

Status: approved preparation design for Issue #1268. This document defines the experiment but does not authorize starting the live canary pull requests before the execution gate is satisfied.

## Goal

Measure the real GitHub Actions behavior of `Oteryn/Oteryn-Platform`, `Oteryn/Oteryn-Game`, and `Oteryn/Oteryn-Atlas` under controlled pull-request stimuli and detect unnecessary workflow fan-out, redundant jobs, wrong path routing, supersession failures, runner contention, duplicate generations, workflow/event amplification, or loops.

The audit must test observable execution, not only static YAML. Static workflow analysis is the expected-behavior oracle; live canary runs are the acceptance evidence.

## Preparation snapshot

These SHAs are informative only and MUST NOT be reused as the live experiment baseline without a fresh preflight:

- Platform: `2ea92ba412fe2a6721b69b021ffb888e3b93d091`
- Game: `5211cb26b9424925cd353822dd6e6b39b7984f21`
- Atlas: `f48edc9170708b341df06339cae6cc113985dce8`

The live Work run must capture fresh protected `main` SHAs and regenerate the workflow/trigger matrix from those exact revisions immediately before creating any canary branch.

## Non-goals

- Do not prove product correctness with canary marker files.
- Do not change branch protection, required checks, workflow permissions, secrets, environments, runners, deployment configuration, or production state.
- Do not fix CI during the measurement phase. Remediation is a separate follow-up so the experiment is not confounded.
- Do not merge canary PRs.
- Do not use workflow dispatch, labels, comments, Ready-for-review transitions, or manual reruns as stimuli unless a later reviewed version of this design explicitly adds them.

## Coordination model

Use one Work coordinator as the only mutating actor. It may use up to three read-only subagents, one per repository, for static workflow inventory and expected-trigger classification. Subagents must not create branches, commits, PRs, comments, labels, reruns, or workflow dispatches.

The coordinator serializes all mutations except the single intentional cross-repository burst phase. This avoids making agent concurrency itself an uncontrolled variable.

All canary PRs remain Draft for their entire lifetime. This avoids advisory Ready-only AI review and prevents accidental merge readiness.

## Execution gate

The live campaign may begin only when all conditions are true:

1. Organization parallel-agent Git concurrency rollout is terminal in Platform, Game, and Atlas.
2. Atlas test-selection/runner-concurrency work that materially changes verification behavior is terminal, or the owner has explicitly declared a specific revision boundary to measure as a baseline. At preparation time PRs #169 and #180 are material examples and therefore block an unqualified final baseline.
3. Fresh `main` SHAs and complete workflow inventories are captured for all three repositories.
4. Every PR-capable workflow is classified by event types, path filters, concurrency group, cancel policy, permissions, environment, runner, and downstream `workflow_run`/`workflow_call` relationship.
5. The expected/forbidden matrix for all nine canaries is regenerated from the fresh SHAs.
6. No unrelated high-load CI campaign is knowingly occupying the same self-hosted runner capacity during the serial baseline.
7. No workflow selected by a canary is expected to perform a production write, protected-environment mutation, secret-bearing external operation, or destructive action on a PR event.

If any condition is false or `UNKNOWN`, do not create canary PRs. Persist the exact blocker and stop as `WAITING` or `BLOCKED`.

## Canary inventory

Each canary changes exactly one inert marker path. The coordinator must revalidate the path against the fresh workflow inventory before use. If a path no longer represents the intended class, select the narrowest equivalent inert path, record the substitution, and regenerate the expected matrix before execution.

| ID | Repository | Canonical marker path | Intended routing class | Anchor expectation |
|---|---|---|---|---|
| P1 | Platform | `docs/ci-canary/OTERYN_CI_CANARY_P1.md` | docs-only control | central/global lightweight controls only; no runtime/acceptance/heavy domain lane |
| P2 | Platform | `resources/js/ci-canary-probe.js` | frontend, core-CI-only probe | Platform classifier `ci=true`; Acceptance must not be triggered solely by this path |
| P3 | Platform | `public/ci-canary.txt` | frontend + product acceptance probe | core CI plus product acceptance selected by `public/**`; unrelated heavy lanes remain forbidden |
| G1 | Game | `docs/ci-canary/OTERYN_CI_CANARY_G1.md` | docs-only control | global merge/governance controls; no direct Rust workspace PR workflow |
| G2 | Game | `crates/ci-canary/README.md` | Rust-classified merge-gate probe | Merge Gate classifies Rust scope; direct `rust.yml` PR trigger remains absent if it still targets only simulation determinism |
| G3 | Game | `crates/simulation-determinism/CI_CANARY.md` | targeted Rust SIM probe | Merge/governance controls plus the direct PR simulation-determinism Rust lane |
| A1 | Atlas | `docs/ci-canary/OTERYN_CI_CANARY_A1.md` | docs-only control | current global Atlas CI/CodeQL behavior is measured; targeted Docker/search/live lanes forbidden unless fresh inventory proves otherwise |
| A2 | Atlas | `web/semantic-search/CI_CANARY.txt` | semantic-search targeted probe | global controls plus Semantic Search lane; Docker E2E forbidden unless independently selected |
| A3 | Atlas | `e2e/CI_CANARY.md` | Docker E2E targeted probe | global controls plus Docker E2E harness; Synology live/main-only and nightly depth forbidden |

The anchor expectations above are not a substitute for the frozen exact-SHA matrix. The matrix generated immediately before execution is authoritative for pass/fail classification.

## Exact marker contents

Use deterministic inert contents so later `synchronize` commits can change only a generation number without altering routing class.

For Markdown/text markers:

```text
OTERYN-CI-CANARY-V1
Canary-ID: <ID>
Generation: 1
Never-Merge: true
```

For `resources/js/ci-canary-probe.js`:

```javascript
// OTERYN-CI-CANARY-V1 P2 generation=1 never-merge
```

Subsequent controlled commits change only `Generation: N` or `generation=N`.

## Branch and PR identity

Branches:

- `probe/ci-canary-v1-p1-docs`
- `probe/ci-canary-v1-p2-core`
- `probe/ci-canary-v1-p3-acceptance`
- `probe/ci-canary-v1-g1-docs`
- `probe/ci-canary-v1-g2-rust-scope`
- `probe/ci-canary-v1-g3-rust-sim`
- `probe/ci-canary-v1-a1-docs`
- `probe/ci-canary-v1-a2-semantic-search`
- `probe/ci-canary-v1-a3-docker-e2e`

PR titles must follow each repository's current metadata contract and remain at most 72 characters. Canonical titles:

- `test(ci): probe docs-only routing P1`
- `test(ci): probe core routing P2`
- `test(ci): probe acceptance routing P3`
- `test(ci): probe docs-only routing G1`
- `test(ci): probe Rust scope routing G2`
- `test(ci): probe Rust SIM routing G3`
- `test(ci): probe docs-only routing A1`
- `test(ci): probe semantic-search routing A2`
- `test(ci): probe Docker E2E routing A3`

Every PR body contains:

```markdown
## Summary
Controlled CI canary `<ID>` for OTERYN-CI-CANARY-AUDIT-V1. This PR exists only to measure GitHub Actions routing and must never be merged.

## Scope
Exactly one inert canary marker path. No product behavior, dependency, workflow, runner, deployment, secret, environment, protection, or production mutation.

## Validation
Expected and forbidden workflow/run behavior is frozen in the central Issue #1268 audit matrix before this PR is opened. Evidence is collected by exact head SHA and run ID.

Branch-Disposition: delete
Branch-Disposition-Reason: ephemeral CI canary; never merge.
```

## Experiment phases

### Phase 0 — freeze

For each repository record:

- exact protected `main` SHA;
- workflow file inventory and blob SHAs;
- PR event types and path filters;
- downstream reusable/workflow-run relationships;
- workflow/job permissions and environments;
- runner labels/groups;
- concurrency group and `cancel-in-progress` semantics;
- current open PRs or runner activity that can materially affect timing.

Generate a per-canary set of `EXPECTED`, `ALLOWED-SKIP`, and `FORBIDDEN` workflows/jobs. Freeze the matrix before the first PR event.

If `main` advances during the experiment, classify the delta. A workflow, CI router, runner contract, applicable agent instruction, or canary-target routing change invalidates the affected repository's frozen matrix and requires refresh before further stimuli. Unrelated upstream movement is recorded as `UPSTREAM_ADVANCED + TEST_MATRIX_VALID` and does not by itself invalidate completed evidence.

### Phase 1 — serial baseline

Open the nine Draft PRs one at a time. Do not open the next canary until every run caused by the previous `opened` event is terminal and the observation snapshot is persisted.

Record all workflow runs/jobs associated with the exact canary head, including skipped/cancelled jobs, runner placement, queue delay, runtime, run attempt, and conclusion.

### Phase 2 — supersession

Use P2, G2, and A2 only. For each repository independently:

1. change generation 1 to 2 and push;
2. while supersedable PR work is active, change generation 2 to 3 and push;
3. observe whether the older generation is cancelled where the frozen workflow contract declares `cancel-in-progress`;
4. classify any continued obsolete execution as expected non-supersedable work or waste.

Do not overlap repositories in this phase.

### Phase 3 — PR metadata event routing

Edit the PR body of P1, G1, and A1 without changing the head SHA. Record exactly which workflows react to the `edited` event. A workflow is a finding only when its frozen event contract says the event should not have produced a run, or when the work performed is disproportionate to a metadata-only event.

Do not change Draft/Ready state and do not add labels/comments.

### Phase 4 — controlled cross-repository burst

After all previous runs are terminal, update P2, G2, and A2 from generation 3 to generation 4 in one bounded coordinator batch. Record queueing and runtime on shared/self-hosted resources and compare with the serial baseline.

This phase is the only intentional cross-repository concurrent mutation.

### Phase 5 — terminal closeout

Close all nine PRs without merge. Observe close-event lifecycle workflows, then verify each ephemeral source branch reaches the repository-declared terminal disposition. Do not use blanket cleanup and do not delete unrelated refs/resources.

## Evidence schema

Persist a central machine-readable evidence object with one record per intentional stimulus and one record per observed workflow run.

Required stimulus fields:

- `stimulus_id`
- `timestamp`
- `repository`
- `canary_id`
- `pr_number`
- `action` (`opened`, `synchronize`, `edited`, `closed`)
- `head_sha`
- `base_sha`
- `generation`

Required run fields:

- `repository`
- `canary_id`
- `pr_number`
- `head_sha`
- `workflow_path`
- `workflow_name`
- `run_id`
- `run_attempt`
- `event`
- `status`
- `conclusion`
- `created_at`
- `run_started_at`
- `updated_at`
- `jobs[]` with job ID/name/status/conclusion/runner labels or runner name when observable
- `expected_state` (`EXPECTED`, `ALLOWED-SKIP`, `FORBIDDEN`)
- `verdict`
- `causal_stimulus_id` or explicit `UNATTRIBUTED`

Timing derived from timestamps must be labeled observed wall/queue/runtime, not GitHub billing minutes.

## Findings taxonomy

- `KEEP` — necessary and correctly routed.
- `OPTIMIZE` — valid lane but disproportionate cost/fan-out.
- `REMOVE_OR_RESTRICT` — no unique proof for this stimulus and safely avoidable subject to separate remediation review.
- `ROUTING_BUG` — expected lane missing or forbidden lane executed.
- `SUPERSESSION_BUG` — obsolete supersedable work continues contrary to declared concurrency policy.
- `DUPLICATE_GENERATION` — same workflow/event/head executes more than once without an explicit retry or separately justified trigger.
- `RUNNER_CONTENTION` — burst materially degrades queueing compared with serial evidence on a shared constrained runner.
- `LOOP_RISK` — static graph permits a recurrence but live self-sustaining recurrence is not proven.
- `LOOP_PROVEN` — new workflow generations continue without a new intentional stimulus and the event chain proves recurrence.
- `UNKNOWN` — evidence is insufficient; never convert this to PASS.

## Metrics

Report per canary and per repository:

- workflow run count per stimulus;
- job count by conclusion;
- observed total job-runtime sum;
- observed wall-clock completion time;
- queue delay distribution where timestamps permit it;
- obsolete runtime after superseding head arrival;
- targeted-heavy-lane count;
- unexpected/forbidden run count;
- duplicate generation count;
- amplification factor = observed workflow runs / intentional stimuli;
- cross-repository burst delta versus comparable serial runs.

## Immediate abort conditions

Stop generating new stimuli for the affected repository and persist evidence when any of these occurs:

- a PR event reaches a production write, protected environment, destructive operation, or unexpected secret-bearing external mutation path;
- workflow generations continue without new intentional stimuli and appear self-sustaining;
- an unexplained duplicate run pattern can materially consume runner capacity;
- runner ownership or target identity is ambiguous;
- the frozen workflow matrix becomes invalid because relevant `main` changes land;
- a canary branch/PR is unexpectedly merged or receives product changes;
- evidence cannot be tied to an exact head/run identity.

## Final outputs

The live Work run produces on the central Platform coordination task:

- `docs/agents/evidence/OTERYN-CI-CANARY-AUDIT-V1.json`
- `docs/agents/reports/OTERYN-CI-CANARY-AUDIT-V1.md`

The report must contain the frozen baseline SHAs, exact canary PR numbers, run/job evidence, findings, before/after comparison when available, and a separate remediation backlog. No CI remediation is mixed into the measurement PRs.
