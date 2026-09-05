# OTERYN-CI-CANARY-AUDIT-V1

**Recommended execution surface:** ChatGPT Work.

**Role:** autonomous cross-repository CI experiment coordinator.

You are executing the controlled Oteryn CI canary campaign defined by:

- `Oteryn/Oteryn-Platform#1268`;
- `docs/superpowers/specs/2026-08-25-organization-ci-canary-audit-design.md`;
- `docs/superpowers/plans/2026-08-25-organization-ci-canary-audit.md`.

The repositories are exactly:

- `Oteryn/Oteryn-Platform`;
- `Oteryn/Oteryn-Game`;
- `Oteryn/Oteryn-Atlas`.

The purpose is to observe real GitHub Actions behavior under controlled PR stimuli and identify unnecessary test/CI execution, wrong path routing, duplicate generations, supersession failures, runner contention, event amplification, or loops.

## Authority and mode

Run as one coordinator. You may delegate static read-only workflow inventory to at most three subagents, one per repository. Subagents must not mutate GitHub state. All branch/commit/PR mutations are owned by the coordinator, except that the coordinator may emit the three explicitly authorized generation-4 commits as one bounded cross-repository burst.

Do not ask for routine confirmation. Do not stop at audit/planning once the execution gate is satisfied. Continue through canary creation, evidence collection, terminal PR closure, cleanup verification, report, and durable closeout.

If the execution gate is not satisfied, do not create canary branches or PRs. Persist `WAITING` or `BLOCKED` with the exact reason and one concrete next action, then stop.

## Mandatory live preflight

Before any canary mutation:

1. Read the current governing `AGENTS.md`/bootstrap/routed testing instructions for each repository and obey the most restrictive applicable rule.
2. Resolve the live state of Platform Issue #1268 and any central coordination PR/task.
3. Fetch fresh exact 40-character protected `main` SHAs for Platform, Game, and Atlas. Do not reuse the preparation snapshot.
4. Inspect material overlapping work. In Atlas, verification/test-selection/runner-concurrency work such as preparation-time PRs #169 and #180 or their successors is material until terminal or explicitly frozen by the owner as a measured baseline boundary.
5. Inspect current runner activity sufficiently to avoid knowingly contaminating the serial baseline with an unrelated high-load campaign.
6. Inventory every PR-capable workflow at the exact baseline SHAs, including event actions, path/branch filters, downstream calls/runs, permissions, environments, runner groups/labels, concurrency groups, and cancellation behavior.
7. Revalidate all nine canary marker paths and freeze the exact expected/allowed-skip/forbidden workflow and job matrix.
8. Fail closed if any selected PR event can reach a production write, protected environment, destructive operation, unexpected secret-bearing external mutation, or ambiguous runner target.

Do not treat upstream `main` movement alone as invalidation. If `main` moves after freeze, inspect the delta. Refresh the affected matrix only when the delta can change applicable workflow routing, CI classification, runner behavior, test contracts, or safety/governance semantics. Otherwise record `UPSTREAM_ADVANCED + TEST_MATRIX_VALID`.

## Canary set

Use these canonical canaries unless the fresh trigger inventory proves a path no longer represents the intended class. Any substitution must be inert, minimal, recorded before execution, and followed by matrix regeneration.

### Platform

**P1**
- branch: `probe/ci-canary-v1-p1-docs`
- path: `docs/ci-canary/OTERYN_CI_CANARY_P1.md`
- title: `test(ci): probe docs-only routing P1`
- intent: docs-only control; no unrelated runtime/acceptance/heavy domain lane.

**P2**
- branch: `probe/ci-canary-v1-p2-core`
- path: `resources/js/ci-canary-probe.js`
- title: `test(ci): probe core routing P2`
- intent: core/frontend CI routing without Acceptance solely because of this path.

**P3**
- branch: `probe/ci-canary-v1-p3-acceptance`
- path: `public/ci-canary.txt`
- title: `test(ci): probe acceptance routing P3`
- intent: core CI plus product Acceptance path, without unrelated heavy lanes.

### Game

**G1**
- branch: `probe/ci-canary-v1-g1-docs`
- path: `docs/ci-canary/OTERYN_CI_CANARY_G1.md`
- title: `test(ci): probe docs-only routing G1`
- intent: global merge/governance controls without direct Rust workspace PR lane.

**G2**
- branch: `probe/ci-canary-v1-g2-rust-scope`
- path: `crates/ci-canary/README.md`
- title: `test(ci): probe Rust scope routing G2`
- intent: Rust classification in Merge Gate while the direct `rust.yml` PR lane remains absent if it still targets only `crates/simulation-determinism/**`.

**G3**
- branch: `probe/ci-canary-v1-g3-rust-sim`
- path: `crates/simulation-determinism/CI_CANARY.md`
- title: `test(ci): probe Rust SIM routing G3`
- intent: targeted direct Rust simulation-determinism PR validation.

### Atlas

**A1**
- branch: `probe/ci-canary-v1-a1-docs`
- path: `docs/ci-canary/OTERYN_CI_CANARY_A1.md`
- title: `test(ci): probe docs-only routing A1`
- intent: measure global Atlas PR CI/CodeQL cost while targeted Docker/search/live lanes remain absent unless fresh inventory proves otherwise.

**A2**
- branch: `probe/ci-canary-v1-a2-semantic-search`
- path: `web/semantic-search/CI_CANARY.txt`
- title: `test(ci): probe semantic-search routing A2`
- intent: global controls plus Semantic Search lane.

**A3**
- branch: `probe/ci-canary-v1-a3-docker-e2e`
- path: `e2e/CI_CANARY.md`
- title: `test(ci): probe Docker E2E routing A3`
- intent: global controls plus Docker E2E; main-only Synology live and nightly verification depth remain forbidden.

## Marker contents

Markdown/text markers begin at generation 1:

```text
OTERYN-CI-CANARY-V1
Canary-ID: <ID>
Generation: 1
Never-Merge: true
```

P2 is:

```javascript
// OTERYN-CI-CANARY-V1 P2 generation=1 never-merge
```

Later synchronize stimuli change only the generation number.

## PR contract

All canary PRs are same-repository branches targeting `main`, remain Draft for their complete lifetime, and are never merged.

Use this body, substituting only the canary ID:

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

Do not add labels, comments, Ready transitions, manual workflow dispatches, or manual reruns as part of the experiment.

## Phase A — serial baseline

Open P1, wait until all runs caused by its `opened` event are terminal, collect and classify complete evidence, then P2, then P3. Repeat G1→G2→G3, then A1→A2→A3.

Never open the next baseline canary while the previous canary's caused runs remain non-terminal.

For every intentional stimulus record:

- repository;
- canary ID;
- PR number;
- action;
- exact head SHA;
- exact base SHA;
- generation;
- stimulus timestamp.

For every observed workflow run record:

- workflow path/name;
- run ID and run attempt;
- event;
- exact head;
- status/conclusion;
- created/start/update timestamps;
- all jobs with ID/name/status/conclusion and runner identity/labels when observable;
- expected state: `EXPECTED`, `ALLOWED-SKIP`, or `FORBIDDEN`;
- verdict;
- causal stimulus ID or explicit `UNATTRIBUTED`.

## Phase B — supersession

Use only P2, G2, and A2, one repository at a time.

For P2:
1. change generation 1→2;
2. push and confirm supersedable work has begun;
3. change generation 2→3;
4. push;
5. observe whether older runs/jobs cancel exactly where the frozen concurrency contract says they should.

Wait for Platform to become terminal, then repeat on G2, then A2.

Do not classify a long run as a supersession defect unless its workflow contract declares it supersedable. Measure obsolete execution after the generation-3 stimulus.

## Phase C — metadata-only event routing

Without changing head SHAs, edit P1, G1, and A1 PR bodies by adding inside `## Validation`:

```text
Metadata-Event-Probe: 1
```

Observe which workflows react to the `edited` event. Compare strictly with the frozen event-action matrix. Prove the head SHA remained unchanged.

## Phase D — cross-repository burst

After every earlier canary-caused run is terminal and no known unrelated heavy runner campaign contaminates the measurement, update P2, G2, and A2 from generation 3→4 in one bounded coordinator batch.

Create no other mutation until all burst-caused work is terminal. Compare queue/runtime evidence with the corresponding serial phases only for comparable jobs/runners.

## Phase E — closeout

After all evidence is frozen:

1. close P1-P3 without merge and capture terminal lifecycle runs;
2. close G1-G3 without merge and capture terminal lifecycle runs;
3. close A1-A3 without merge and capture terminal lifecycle runs;
4. verify all nine PRs are closed/unmerged;
5. verify all `probe/ci-canary-v1-*` branches reach the repository-declared terminal disposition;
6. verify no unintended canary-owned container, deployment, environment, runner resource, or other temporary resource remains.

Use exact ownership-scoped cleanup only. Never use blanket prune/delete operations.

## Abort conditions

Immediately stop generating new stimuli for the affected repository when any condition is observed:

- PR-triggered execution reaches a production write, protected environment, destructive operation, or unexpected secret-bearing external mutation path;
- workflow generations continue without new intentional stimuli and appear self-sustaining;
- an unexplained duplicate run pattern materially consumes capacity;
- runner target/ownership is ambiguous;
- relevant baseline routing/safety changes land on `main`;
- a canary unexpectedly becomes Ready, receives product changes, or is merged;
- evidence cannot be bound to exact head/run identities.

Persist the evidence first. Do not attempt to repair CI inside the canary PRs.

## Findings taxonomy

Use only these primary finding classes:

- `KEEP`
- `OPTIMIZE`
- `REMOVE_OR_RESTRICT`
- `ROUTING_BUG`
- `SUPERSESSION_BUG`
- `DUPLICATE_GENERATION`
- `RUNNER_CONTENTION`
- `LOOP_RISK`
- `LOOP_PROVEN`
- `UNKNOWN`

`LOOP_PROVEN` requires both: no new intentional stimulus and a proven causal event chain that creates new workflow generations. Repeated work with a known stimulus is not a loop.

Do not convert `UNKNOWN` to PASS.

## Metrics

Calculate per canary and repository:

- workflow runs per intentional stimulus;
- jobs by conclusion;
- observed total job-runtime sum;
- observed wall-clock completion;
- queue delay where timestamps permit;
- obsolete runtime after superseding head arrival;
- targeted-heavy-lane count;
- forbidden/unexpected count;
- duplicate generation count;
- amplification factor = workflow runs / intentional stimuli;
- burst delta versus comparable serial evidence.

Do not call observed runtime GitHub billed minutes unless a billing API was directly queried.

## Durable outputs

Persist centrally in `Oteryn/Oteryn-Platform`:

- `docs/agents/evidence/OTERYN-CI-CANARY-AUDIT-V1.json`
- `docs/agents/reports/OTERYN-CI-CANARY-AUDIT-V1.md`

The report must include:

- exact three-repository baseline SHAs;
- workflow matrix identity and relevant workflow blob SHAs;
- all nine PR numbers and exact generation heads;
- run/job evidence and timing;
- routing/supersession/duplication/contention/loop conclusions;
- cleanup proof;
- a before/after section when a valid earlier Atlas/organization boundary exists;
- a ranked remediation backlog separated from the measurement campaign.

Do not modify CI to fix findings during this same measurement campaign. Create separate remediation Issues/branches only after the audit evidence is complete and the owner has authorized that follow-up phase.

## Terminal result

Finish only when either:

- `DONE`: all live phases executed, all nine canaries closed/unmerged, cleanup verified, evidence/report persisted, and the central lifecycle is reconciled; or
- `WAITING`/`BLOCKED`: the execution gate or a safety/authority condition prevents starting/continuing, exact evidence is persisted, and exactly one concrete next action is recorded.

Do not claim completion from planning, green individual runs, or partial cleanup.
