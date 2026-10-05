---
task_id: OTERYN-20261005-php-coverage-enforcement
governing_issue: 1010
required_reads:
  - docs/agents/CI_COVERAGE_POLICY.json
search_first:
  - php coverage policy
optional_reads: []
---

# OTERYN-20261005 PHP coverage enforcement — terminal archive

## Terminal outcome

Issue #1010 implementation is complete.

PR #1447 was rebuilt on current protected main after stale-base PR #1445 was superseded. PR #1447 passed its exact-head pull-request checks, entered Merge Queue, and merged to protected main as `a29c0f990cbac725e47e9f0407a16fbc9eaf333c`.

The enforced coverage policy is now:

- mode: `enforce`;
- verified stable statement baseline: **82.69%**;
- enforced statement floor: **82.0%**;
- method coverage reference: **55.86%**;
- statement count: **20,669**;
- covered statements: **17,091**;
- ordinary pull requests do not run the expensive coverage lane;
- relevant Merge Queue candidates and relevant protected-main pushes do.

## Acceptance criteria

- [x] Record the verified stable baseline from qualifying protected-main coverage runs.
- [x] Switch the policy to enforce mode with a reviewed floor below, but close to, the stable baseline.
- [x] Run the coverage job on relevant Merge Queue candidates before integration and on relevant main pushes for durable evidence.
- [x] Do not run the expensive coverage job on ordinary pull_request events.
- [x] Preserve existing integration/E2E/security/concurrency gates as authoritative behavioral evidence.
- [x] Exact-head policy tests, workflow routing tests and required CI pass.

## Evidence

### Baseline evidence

Five consecutive qualifying protected-main runs from 2026-10-01 reported the same PHP coverage:

- runs `36869483285`, `36856010085`, `36854848269`, `36848883850`, `36848616932`;
- statement coverage **82.69%**;
- method coverage **55.86%**;
- statements **20,669**;
- covered statements **17,091**.

### Pull-request generation

Fresh-base PR #1447 exact head `32053184a65df40476996e0abdf62652ef020b94` passed:

- CI `37299179995`;
- Agent Governance `37299179989`;
- CodeQL `37299180089`;
- Platform DB Outage Validation `37299179982`;
- Game Auth Ticket Concurrency `37299180075`;
- Edge Security Emulation `37299180023`;
- Phase 7 Production-Like Validation `37299180162`.

On the ordinary pull-request event, `php-coverage-report` was intentionally **SKIPPED**, proving the expensive lane is not charged to normal PR validation.

### Merge Queue proof

Merge Queue candidate `a29c0f990cbac725e47e9f0407a16fbc9eaf333c` executed CI run `37299672530`.

The candidate passed:

- `runtime-tests`;
- `php-coverage-report`;
- required `test`;
- aggregate `platform-gate`.

Coverage evaluation reported:

- statement coverage **82.69%**;
- method coverage **55.86%**;
- enforced minimum **82.0%**;
- result: **statement coverage meets enforced floor**.

Durable artifact: `11340816108`, named `php-coverage-a29c0f990cbac725e47e9f0407a16fbc9eaf333c-1`.

### Protected-main proof

After Merge Queue integration, protected-main CI run `37300007155` executed the coverage lane again on `main@a29c0f990cbac725e47e9f0407a16fbc9eaf333c`.

It passed:

- `runtime-tests`;
- `php-coverage-report`;
- required `test`;
- aggregate `platform-gate`.

Post-merge coverage remained **82.69%**, above the **82.0%** floor. Durable artifact: `11342055808`.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-05T11:05:00Z
head: a29c0f990cbac725e47e9f0407a16fbc9eaf333c
branch: docs/issue-1010-php-coverage-closeout
pr: none
status: completed
terminal_pr_policy: archive_pending
context_routes:
  - ci
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20261005-php-coverage-enforcement.md
proven:
  - PR #1447 merged through Merge Queue as a29c0f990cbac725e47e9f0407a16fbc9eaf333c
  - Merge Queue CI run 37299672530 executed php-coverage-report and platform-gate successfully
  - Merge Queue coverage was 82.69 percent against enforced 82.0 percent floor
  - protected-main CI run 37300007155 repeated coverage and aggregate gating successfully
  - protected-main coverage remained 82.69 percent
  - stale-base PR #1445 was closed as superseded and its source branch was deleted by Terminal Branch Lifecycle
  - PR #1447 source branch was automatically deleted after merge
derived:
  - coverage regression enforcement now protects relevant integration candidates before main and records durable evidence after merge
unknown: []
conflicts: []
first_failure:
  marker: none-open
  evidence: all implementation and integration gates passed
rejected_hypotheses:
  - coverage observability was missing entirely
  - ordinary PRs must pay the full coverage runtime cost
  - the threshold should be set at or above the exact measured baseline
changed_paths:
  - .github/workflows/ci.yml
  - docs/agents/CI_COVERAGE_POLICY.json
  - tools/validation/php_coverage_policy.py
  - tools/validation/test_php_coverage_policy.py
  - tests/ci/test_workflow_trigger_economy.py
  - docs/agents/tasks/archive/OTERYN-20261005-php-coverage-enforcement.md
validation:
  - command: PR #1447 exact-head required and relevant checks
    result: PASS
    evidence: CI 37299179995; Agent Governance 37299179989; CodeQL 37299180089; DB Outage 37299179982; Game Auth Concurrency 37299180075; Edge Security 37299180023; Phase 7 37299180162
  - command: Merge Queue candidate coverage enforcement
    result: PASS
    evidence: CI 37299672530; 82.69 percent statement coverage; 82.0 percent enforced floor; artifact 11340816108
  - command: protected-main coverage enforcement
    result: PASS
    evidence: CI 37300007155; 82.69 percent statement coverage; platform-gate PASS; artifact 11342055808
blockers: []
next_action: merge the sequential archive-closeout PR, verify its source branch is absent, then close Issue #1010 as completed
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: lifecycle-only closeout branch has no retention or recovery purpose after archive integration
source_branch_evidence: pending closeout PR merge and post-merge branch absence verification
```
