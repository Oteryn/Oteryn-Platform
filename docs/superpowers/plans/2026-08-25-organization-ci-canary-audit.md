# Organization CI Canary Audit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Execute a controlled nine-PR cross-repository canary campaign that measures real GitHub Actions routing, cost, supersession, contention, duplication, and loop behavior without merging probes or touching production.

**Architecture:** One Work coordinator owns every mutating event and may delegate only read-only static inventory to at most three repository-scoped subagents. A frozen exact-SHA workflow matrix is built before execution, then the coordinator runs serial baseline, supersession, metadata-event, and bounded cross-repository burst phases before closing every canary and producing central machine-readable evidence plus a human report.

**Tech Stack:** GitHub repositories and pull requests, GitHub Actions workflow/run/job APIs, repository Markdown/JSON evidence, existing repository branch-lifecycle/governance contracts.

**Spec:** `docs/superpowers/specs/2026-08-25-organization-ci-canary-audit-design.md`

## Global Constraints

- Governing preparation/live coordination Issue: `Oteryn/Oteryn-Platform#1268` unless live GitHub state explicitly supersedes it.
- Repositories are exactly `Oteryn/Oteryn-Platform`, `Oteryn/Oteryn-Game`, and `Oteryn/Oteryn-Atlas`.
- The preparation snapshot SHAs in the spec are not execution authority; fresh `main` SHAs are mandatory immediately before the live campaign.
- Actual canary PRs remain Draft and are never merged.
- No Ready transition, label stimulus, issue comment stimulus, manual rerun, workflow dispatch, branch-protection change, secret use, protected-environment approval, production write, or destructive cleanup.
- One coordinator is the only mutating actor. Read-only parallel analysis is allowed; parallel mutation is allowed only in the explicit cross-repository burst task.
- If Atlas test-selection/runner-concurrency changes are not terminal or explicitly frozen, return `WAITING` before creating any canary branch.
- Any unexpected production/write-capable execution, self-sustaining run recurrence, ambiguous runner identity, or relevant baseline drift stops new stimuli for the affected repository.
- Evidence from an old head never satisfies an exact-head conclusion.
- Upstream movement alone is `UPSTREAM_ADVANCED`, not task invalidation. Refresh the matrix only when the upstream delta can change the experiment's routing/safety semantics.

---

### Task 1: Resolve live authority and execution gate

**Files:**
- Read: root and routed agent instructions in all three repositories.
- Read: `docs/superpowers/specs/2026-08-25-organization-ci-canary-audit-design.md`.
- Read/update: `docs/agents/tasks/active/OTERYN-20260825-ci-canary-audit-preparation.md` or its live successor.
- No canary file is created in this task.

**Interfaces:**
- Consumes: live GitHub Issue #1268, live repository `main` refs, open PR/task state.
- Produces: `execution_gate = PASS | WAITING | BLOCKED` plus exact reason and fresh repository SHAs.

- [ ] **Step 1: Verify the central task**

Fetch Issue #1268 and the live central coordination PR/task record if present. Confirm the issue is open and the current task has not been superseded. If superseded, follow the newer live authority and record the transition.

- [ ] **Step 2: Resolve fresh repository SHAs**

Capture the full 40-character `main` SHA for Platform, Game, and Atlas from GitHub. Record them as `baseline_candidate_sha` values; do not use cached/local refs.

- [ ] **Step 3: Resolve material Atlas overlap**

Inspect open/merged state for Atlas verification/E2E optimization work, including the successors of preparation-time PRs #169 and #180 when applicable. Classify each as terminal, non-material to CI routing, or material-and-active.

Expected: no live canary work starts while material Atlas selector/runner work remains active unless the owner has explicitly frozen a revision boundary for a baseline.

- [ ] **Step 4: Check for materially competing load**

Inspect current Actions activity and open PRs that use the same self-hosted runner groups. If a known high-load campaign would contaminate timing, persist `WAITING` with the exact runs/PRs and stop before mutations.

- [ ] **Step 5: Persist gate result**

If every gate is satisfied, record `execution_gate=PASS` with the three exact SHAs and continue. Otherwise update the durable checkpoint with exactly one next action and stop.

---

### Task 2: Freeze the exact workflow/trigger matrix

**Files:**
- Read: `.github/workflows/**` at each fresh exact baseline SHA.
- Read: Platform `scripts/ci/classify_changes.py` and applicable routing contracts at the exact baseline.
- Create during live execution: central evidence staging data for the frozen matrix.

**Interfaces:**
- Consumes: exact baseline SHAs from Task 1.
- Produces: one versioned matrix mapping each canary to `EXPECTED`, `ALLOWED-SKIP`, or `FORBIDDEN` workflows/jobs.

- [ ] **Step 1: Inventory every workflow**

For every workflow file, record path, blob SHA, workflow name, events/actions, branch filters, path filters, `workflow_run`/`workflow_call` relationships, concurrency group, `cancel-in-progress`, permissions, environments, and runner selector.

Up to three read-only subagents may do this in parallel, one repository each. They may not mutate GitHub state.

- [ ] **Step 2: Validate canonical canary paths**

Re-evaluate the nine canonical paths from the spec against the fresh trigger inventory. Verify that each path still represents the intended class. If not, choose the narrowest inert substitute path that preserves the intended trigger class and record the substitution before any PR is opened.

- [ ] **Step 3: Apply anchor assertions**

At minimum verify these design anchors still hold or explicitly classify the drift:

```text
P1: docs-only => no Platform runtime/acceptance/heavy domain lane
P2: resources/js marker => core CI, not Acceptance solely from that path
P3: public marker => core CI + product Acceptance path
G1: docs-only => no direct Rust workspace PR workflow
G2: crates marker => Merge Gate Rust scope; direct rust.yml absent if its PR path is still simulation-determinism-only
G3: simulation-determinism marker => direct Rust SIM PR lane
A1: docs-only => global Atlas PR controls measured; targeted Docker/search/live lanes absent
A2: semantic-search marker => Semantic Search lane selected
A3: e2e marker => Docker E2E selected; main-only Synology live and nightly depth absent
```

- [ ] **Step 4: Freeze per-canary expected sets**

For each ID P1..A3 create exact workflow/job sets with reason strings. Include global control-plane workflows even when they do almost no work. Record `ALLOWED-SKIP` separately from `EXPECTED` so a correctly skipped job is not misclassified as missing proof.

- [ ] **Step 5: Safety review selected PR paths**

For every workflow that can start from any canary, inspect permissions/environment/write-capable steps. If a PR event can reach production, protected-environment mutation, secret-bearing external operation, or destructive action, set `execution_gate=BLOCKED` and stop before creating branches.

- [ ] **Step 6: Freeze matrix identity**

Hash or otherwise deterministically identify the matrix together with the three baseline SHAs and workflow blob SHAs. Persist this identity in the central task checkpoint/evidence staging data.

---

### Task 3: Create nine Draft canary PRs and capture serial baseline

**Files:**
- Platform markers:
  - `docs/ci-canary/OTERYN_CI_CANARY_P1.md`
  - `resources/js/ci-canary-probe.js`
  - `public/ci-canary.txt`
- Game markers:
  - `docs/ci-canary/OTERYN_CI_CANARY_G1.md`
  - `crates/ci-canary/README.md`
  - `crates/simulation-determinism/CI_CANARY.md`
- Atlas markers:
  - `docs/ci-canary/OTERYN_CI_CANARY_A1.md`
  - `web/semantic-search/CI_CANARY.txt`
  - `e2e/CI_CANARY.md`

**Interfaces:**
- Consumes: frozen matrix and exact baselines from Tasks 1-2.
- Produces: nine Draft PR numbers, generation-1 head SHAs, and terminal serial-baseline run/job evidence.

- [ ] **Step 1: Create P1 from exact Platform baseline**

Create branch `probe/ci-canary-v1-p1-docs` from the frozen Platform baseline and add exactly:

```text
OTERYN-CI-CANARY-V1
Canary-ID: P1
Generation: 1
Never-Merge: true
```

Open Draft PR titled `test(ci): probe docs-only routing P1` with the exact body template from the spec.

- [ ] **Step 2: Observe P1 to terminal state**

Persist every workflow run/job attributable to the `opened` event and classify it against the frozen matrix before creating P2.

- [ ] **Step 3: Repeat serially for P2 and P3**

P2 JavaScript marker content:

```javascript
// OTERYN-CI-CANARY-V1 P2 generation=1 never-merge
```

P3 uses the standard four-line text marker. Open and fully observe P2, then P3. Never have two new Platform baseline `opened` stimuli in flight at once.

- [ ] **Step 4: Repeat serially for G1, G2, G3**

Use the standard text marker, exact branch/title identities from the spec, and the mandatory `## Summary`, `## Scope`, `## Validation` PR body headings. Wait for every caused run to become terminal before the next Game canary.

- [ ] **Step 5: Repeat serially for A1, A2, A3**

Use the standard text marker and exact branch/title identities from the spec. Keep all Atlas canaries Draft. Record global CI/CodeQL behavior separately from targeted Semantic Search/Docker E2E behavior.

- [ ] **Step 6: Baseline integrity check**

Confirm there are exactly nine live canary PRs, each changes exactly one marker path, no canary is merged/Ready, and every generation-1 head is bound to a complete observation snapshot.

---

### Task 4: Measure supersession behavior

**Files:**
- Modify only the existing marker on P2, G2, and A2 branches.

**Interfaces:**
- Consumes: generation-1 baseline and frozen concurrency contracts.
- Produces: generation-2/3 cancellation evidence and obsolete-work measurements.

- [ ] **Step 1: Run P2 supersession**

Change P2 marker from `generation=1` to `generation=2` and push. After supersedable PR work has actually started, change only `generation=2` to `generation=3` and push.

Record the exact arrival time/head SHA for both synchronize events and whether older runs/jobs cancel where the frozen workflow says `cancel-in-progress`.

- [ ] **Step 2: Classify P2 obsolete work**

For each job still running after generation 3 arrives, classify it as intentionally non-supersedable or `SUPERSESSION_BUG`/`OPTIMIZE` using the frozen contract. Do not infer a bug from duration alone.

- [ ] **Step 3: Repeat for G2**

Perform the same two-commit sequence only after Platform supersession evidence is terminal.

- [ ] **Step 4: Repeat for A2**

Perform the same two-commit sequence only after Game supersession evidence is terminal.

- [ ] **Step 5: Verify no cross-repo overlap occurred**

Confirm Tasks 4.1-4.4 were serialized. If unrelated external runs overlapped, mark timing comparison `CONTAMINATED` rather than fabricating a clean baseline.

---

### Task 5: Measure metadata-only PR event routing

**Files:**
- No repository file changes.
- Edit only PR body text on P1, G1, and A1.

**Interfaces:**
- Consumes: frozen event-action contracts.
- Produces: exact `edited`-event run evidence on unchanged head SHAs.

- [ ] **Step 1: Edit P1 body without changing semantic scope**

Append one line inside `## Validation`:

```text
Metadata-Event-Probe: 1
```

Record whether any workflow starts and compare it with the frozen `pull_request` action contract.

- [ ] **Step 2: Repeat for G1**

Use the same line. In particular, verify workflows that explicitly include `edited` behave as declared while unrelated code/test workflows remain absent.

- [ ] **Step 3: Repeat for A1**

Use the same line and observe whether Atlas workflows using default pull-request action behavior correctly remain idle on `edited` where applicable.

- [ ] **Step 4: Persist unchanged-head proof**

For all three PRs prove the repository head SHA did not change during the metadata event. Any run must therefore be attributed to the metadata stimulus rather than a synchronize commit.

---

### Task 6: Measure controlled cross-repository contention

**Files:**
- Modify only P2, G2, and A2 markers from generation 3 to generation 4.

**Interfaces:**
- Consumes: serial baseline timings and runner identities.
- Produces: bounded burst queue/runtime evidence.

- [ ] **Step 1: Ensure a clean start**

Wait until every prior canary-caused run is terminal and verify no known unrelated heavy run already occupies the measured self-hosted runner capacity.

- [ ] **Step 2: Emit the three generation-4 commits as one coordinator batch**

Change only the generation marker on P2, G2, and A2. Record the three stimulus timestamps and head SHAs. Do not generate any other mutation until the burst is terminal.

- [ ] **Step 3: Compare runner queueing**

For jobs that use comparable shared/self-hosted resources, calculate observed queue delay and runtime versus their own serial phase. Do not compare unrelated job types as if they were equivalent.

- [ ] **Step 4: Classify contention**

Use `RUNNER_CONTENTION` only when evidence shows material shared-resource queue degradation attributable to the burst. Otherwise record `KEEP`, `OPTIMIZE`, or `UNKNOWN` as supported.

---

### Task 7: Close all canaries without merge and verify cleanup

**Files:**
- No new canary content.

**Interfaces:**
- Consumes: nine open Draft canary PRs.
- Produces: terminal PR states, close-event evidence, verified branch dispositions, no unintended task-owned resources.

- [ ] **Step 1: Freeze final measurement evidence**

Before closure, verify all open-canary run records are persisted and every active run is terminal or explicitly classified as an abort finding.

- [ ] **Step 2: Close PRs one repository at a time**

Close P1-P3, then G1-G3, then A1-A3 without merge. Observe any close-event/terminal branch-lifecycle workflows and capture run IDs.

- [ ] **Step 3: Verify source-branch disposition**

Confirm each ephemeral `probe/ci-canary-v1-*` ref is deleted by the repository's intended lifecycle or perform only the narrowly authorized exact cleanup path. Never use blanket branch/resource cleanup.

- [ ] **Step 4: Verify no canary resource remains**

Confirm no canary PR is open, no canary branch remains unintentionally, no canary was merged, and no temporary runner/container/deployment resource was created or left behind by the audit itself.

---

### Task 8: Build machine-readable evidence and final audit report

**Files:**
- Create/update: `docs/agents/evidence/OTERYN-CI-CANARY-AUDIT-V1.json`
- Create/update: `docs/agents/reports/OTERYN-CI-CANARY-AUDIT-V1.md`
- Update: central task checkpoint.

**Interfaces:**
- Consumes: all frozen-matrix, stimulus, workflow-run, job, timing, and cleanup evidence.
- Produces: reproducible audit conclusions and remediation backlog.

- [ ] **Step 1: Materialize the evidence JSON**

Write the exact schema from the design. Every run must have an exact repository/head/run identity and `causal_stimulus_id` or explicit `UNATTRIBUTED`. Validate JSON syntax and uniqueness of `(repository, run_id, run_attempt)`.

- [ ] **Step 2: Calculate metrics**

Per canary and per repository calculate workflow runs/stimulus, jobs by conclusion, observed job-runtime sum, wall time, available queue delay, obsolete superseded runtime, heavy-lane count, forbidden count, duplicate count, amplification factor, and burst delta.

- [ ] **Step 3: Apply findings taxonomy**

Classify every material finding using only:

```text
KEEP
OPTIMIZE
REMOVE_OR_RESTRICT
ROUTING_BUG
SUPERSESSION_BUG
DUPLICATE_GENERATION
RUNNER_CONTENTION
LOOP_RISK
LOOP_PROVEN
UNKNOWN
```

A loop may be called `LOOP_PROVEN` only when workflow generations continue without a new intentional stimulus and the causal event chain proves recurrence.

- [ ] **Step 4: Write the report**

Include exact baseline SHAs and workflow matrix identity, all nine PR numbers/heads, phase results, before/after comparison when a valid earlier boundary exists, top waste sources, routing defects, contention, loop conclusions, cleanup proof, and an ordered remediation backlog.

Do not claim GitHub billed minutes unless a billing API was actually queried; label summed job durations as observed runtime.

- [ ] **Step 5: Validate report/evidence consistency**

Check every reported run ID exists in the evidence JSON, every reported finding has supporting evidence, every canary has terminal closure evidence, and all `UNKNOWN` items remain explicit.

---

### Task 9: Handoff remediation without contaminating the experiment

**Files:**
- Update: central task checkpoint/Issue #1268.
- No CI workflow remediation in this task unless the owner starts a separate authorized remediation phase.

**Interfaces:**
- Consumes: final report.
- Produces: one ranked follow-up list and terminal/next-phase state.

- [ ] **Step 1: Separate measurement from remediation**

Group proposed fixes by repository and root cause, but do not modify workflow files on the canary branches or retroactively rerun the experiment with unreviewed changes.

- [ ] **Step 2: Rank follow-ups**

Use severity order: proven unsafe/write behavior or loop, routing correctness, duplicate/supersession waste, self-hosted contention, then general optimization.

- [ ] **Step 3: Decide next lifecycle state from evidence**

If the audit is complete and cleanup verified, persist the final report/evidence and close or transition the central Issue according to the repository's live lifecycle. If remediation is authorized, create separate narrow Issues/branches per independently reviewable root cause.

- [ ] **Step 4: Leave one concrete next action**

The durable checkpoint must contain exactly one next action, such as the first approved remediation Issue or terminal archive/merge closeout. Do not leave an ambiguous continuation instruction.
