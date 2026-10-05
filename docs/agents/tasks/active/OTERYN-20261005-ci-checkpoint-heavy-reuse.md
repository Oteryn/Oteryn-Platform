---
task_id: OTERYN-20261005-ci-checkpoint-heavy-reuse
governing_issue: 1012
required_reads:
  - docs/agents/BUILD_TEST_MATRIX.md
  - scripts/ci/classify_changes.py
  - tests/ci/test_classify_changes.py
  - tests/ci/test_workflow_trigger_economy.py
search_first:
  - checkpoint-only heavy workflow reuse
  - PR 992
optional_reads: []
---

# OTERYN-20261005 CI checkpoint-only heavy-workflow reuse

## Goal

Continue Issue #1012 from current protected Platform state without redoing already-completed Platform Lead work.

Implement a fail-closed CI evidence-reuse mechanism so a later task/checkpoint/docs-only commit does not rerun unchanged heavy validation when the material tree for that workflow is provably identical to a previously successful material generation in the same PR.

Do not weaken exact-final-head governance, security/auth/migration/deployment validation, branch protection, Merge Queue gating, or protected-main validation.

## Verified continuation snapshot

### Completed before this task

- Issue #1010 is completed. PHP statement coverage enforcement is merged and protected:
  - Merge Queue implementation integration: `a29c0f990cbac725e47e9f0407a16fbc9eaf333c`.
  - Enforced floor: **82.0%** against verified stable **82.69%** statement coverage.
  - Closeout PR #1448 merged as `b143cff5dd19f622990791d2f9b93d2c483020f8`.
- Platform lane of Issue #1399 is complete with `TASK_SELF_INTEGRATION=PASS`:
  - exact delivery PR #1449 head: `84232c82271182b4373336b2c55a9679b5e1b251`;
  - governed META #196 request comment: `5994026696`;
  - executor run: `37306816432`;
  - provider UUID: `d8c8be6e-5146-4e00-afa6-d6e29e0e8399`;
  - real Platform merge-group CI: `37306867655`;
  - protected Platform integration: `05eeb6c6b8fbba762c9e783b173e2e30ca0e53e6`;
  - Platform V2 closeout #1450 merged as `b402fdf769a78f30ee6c54574900a37b2530b4cf`.
- Issue #1399 remains open for Game/Atlas only. Platform must not mutate those repositories from this task.
- Current protected Platform `main` at handoff creation is `91d70e7075697d11cf3f3e53e19dd2dc2c012438`.

### Issue #1012 ownership/readback

- Issue #1012 is open.
- No PR related to #1012 existed at handoff inspection time.
- No branch matching #1012 existed at handoff inspection time.
- No active task packet for this #1012 implementation existed before this file.
- Therefore this task is the current Platform ownership anchor for #1012.

### Current implementation evidence already inspected

The repository already has one canonical classifier: `scripts/ci/classify_changes.py`. Do not introduce a parallel classifier unless current source proves the existing one cannot express the required invariant.

Current behavior observed from protected main:

- `.github/workflows/ci.yml` classifies a pull request from PR base SHA to PR head SHA, so accumulated PR diff remains visible after a checkpoint-only successor commit.
- Heavy workflows including:
  - `.github/workflows/edge-security-emulation.yml`;
  - `.github/workflows/platform-db-outage-validation.yml`;
  - `.github/workflows/game-auth-ticket-concurrency.yml`;
  - `.github/workflows/phase7-production-like-validation.yml`
  also classify PR base-to-head and therefore cannot by themselves distinguish a checkpoint-only successor generation from the earlier material generation.
- `scripts/ci/classify_changes.py` already fails closed for:
  - central CI workflow changes;
  - classifier changes under `scripts/ci/**`;
  - classifier/CI contract tests under `tests/ci/**`;
  - empty/ambiguous path sets.
- Agent-governance-only paths already map to no runtime gates; the remaining #1012 gap is generation/evidence reuse when the accumulated PR diff still contains earlier material changes.

## Acceptance criteria

- [ ] Define a fail-closed distinction between latest-commit scope and accumulated PR delivery scope.
- [ ] Reuse prior heavy evidence only when the relevant material tree/evidence is cryptographically or otherwise deterministically proven unchanged.
- [ ] Any later runtime/build/workflow/classifier/material change invalidates prior relevant heavy evidence and reruns the affected heavy lane.
- [ ] Ambiguous git state, missing prior evidence, missing material identity, or unverifiable ancestry fails closed to heavy validation.
- [ ] Lightweight exact-final-head governance/metadata validation continues to run.
- [ ] Add deterministic tests covering:
  - material change -> heavy run;
  - checkpoint-only successor -> reuse/no redundant heavy run;
  - material change after checkpoint -> heavy run again;
  - ambiguous/missing state -> fail closed to heavy run.
- [ ] Persist reused material head and originating successful run/check identifiers in auditable task/PR evidence.
- [ ] Do not modify Oteryn-Game, Oteryn-Atlas, production, secrets, or protected environments.
- [ ] Merge only through protected Merge Queue after exact-head checks pass; verify protected-main readback after merge.

## Ownership

```yaml
owned_paths:
  - scripts/ci/**
  - tests/ci/**
  - .github/workflows/ci.yml
  - .github/workflows/edge-security-emulation.yml
  - .github/workflows/platform-db-outage-validation.yml
  - .github/workflows/game-auth-ticket-concurrency.yml
  - .github/workflows/phase7-production-like-validation.yml
  - docs/agents/tasks/active/OTERYN-20261005-ci-checkpoint-heavy-reuse.md
modules:
  - Platform CI routing
  - heavy-workflow evidence reuse
dependencies:
  - GitHub PR commit ancestry and check/run evidence
blockers:
  - none identified at handoff
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-05T20:03:00+02:00
head: 91d70e7075697d11cf3f3e53e19dd2dc2c012438
branch: docs/issue-1012-checkpoint-heavy-reuse-handoff
pr: none
status: ready
terminal_pr_policy: active
context_routes:
  - ci
  - merge-queue
owned_paths:
  - scripts/ci/**
  - tests/ci/**
  - .github/workflows/ci.yml
  - .github/workflows/edge-security-emulation.yml
  - .github/workflows/platform-db-outage-validation.yml
  - .github/workflows/game-auth-ticket-concurrency.yml
  - .github/workflows/phase7-production-like-validation.yml
  - docs/agents/tasks/active/OTERYN-20261005-ci-checkpoint-heavy-reuse.md
proven:
  - Issue #1012 is open
  - no #1012 PR or branch existed before this task packet
  - current canonical classifier is scripts/ci/classify_changes.py
  - current heavy PR workflows classify accumulated base-to-head PR diff
  - classifier control-plane changes already fail closed to all heavy gates
  - Platform coverage enforcement #1010 is complete
  - Platform V2 lane of #1399 is complete; Game and Atlas remain outside this task
derived:
  - the next implementation should extend current CI routing/evidence semantics rather than create a second unrelated classifier
  - safe optimization requires material-generation identity plus successful prior evidence, not merely inspection of the latest commit path list
unknown:
  - exact minimal representation for per-heavy-lane material identity on the current workflow generation
  - exact GitHub evidence lookup contract that is simplest while remaining fail closed
conflicts: []
first_failure:
  marker: none
  evidence: implementation has not started
rejected_hypotheses:
  - latest commit paths alone are sufficient proof to skip heavy validation
  - accumulated PR diff alone can distinguish checkpoint-only successor generations
  - generic successful prior CI is enough without binding evidence to the unchanged relevant material generation
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261005-ci-checkpoint-heavy-reuse.md
validation:
  - command: live GitHub ownership inspection for Issue #1012
    result: PASS
    evidence: issue open; no related PR; no matching branch before this task
  - command: protected-main source inspection
    result: PASS
    evidence: current classifier and four heavy workflow base-to-head PR classification paths inspected
blockers: []
next_action: From current protected main, design the smallest fail-closed material-generation identity/evidence-reuse contract around the existing classifier and heavy workflows, write deterministic positive/negative fixtures first, then implement and validate Issue #1012 without touching Game or Atlas.
```

## New-chat resume instruction

Start from this file and live GitHub state. Re-read current `main`, Issue #1012 and any new PR/branch before editing because repository state may have advanced after this snapshot. Do not repeat completed #1010 or Platform #1399 work unless live evidence contradicts the recorded terminal state.

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: docs-only continuation checkpoint; no retention purpose after protected integration
source_branch_evidence: pending
```
