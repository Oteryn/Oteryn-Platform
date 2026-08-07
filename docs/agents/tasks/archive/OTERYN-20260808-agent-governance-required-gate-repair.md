---
task_id: OTERYN-20260808-agent-governance-required-gate-repair
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
search_first:
  - issue #858
  - PR #861
optional_reads: []
---

# OTERYN-20260808-agent-governance-required-gate-repair

## Result

`completed`

- Finding: Issue #858 — closed completed.
- Delivery PR: #861.
- Exact delivery head: `f7a76053b8afd1d5f5f5d30bf7752563fdcd642a`.
- Squash merge: `48bcff78b119308e7d0d93753f29e5927f511ef5`.
- Resulting-main Agent Governance: run `31226632635` — PASS.
- Resulting-main CI: run `31226632716` — PASS; `classify-changes`, `runtime-tests` and aggregate `test` all passed.
- Exact-head full-diff self-review: `PASS_ZERO_MATERIAL_FINDINGS`; review `4887233484`, zero unresolved review threads.
- E2E: `NOT_APPLICABLE` — governance/CI merge-enforcement only, with no executable user or integration journey.
- Runtime/product/schema/authentication/payment/deployment/production/external-repository changes: none.

## Repair delivered

Issue #858 proved that an invalid Markdown record under `docs/agents/tasks/active/` could make Agent Governance fail while the branch-protection-required CI contexts remained green. The protected branch required `classify-changes` and `test`, but neither validated the canonical active-task checkpoint contract.

PR #861 closes that gap by making required `classify-changes` run:

```text
python tools/agents/checkpoint.py --tasks docs/agents/tasks/active --require-checkpoint
```

before change classification. Because the aggregate `test` job already fails closed when classification fails, malformed active-task checkpoint structure is now covered by branch-protection-required CI rather than only by the separate Agent Governance workflow.

`tests/ci/test_required_test_gate.py` also pins this protected-workflow contract so removing the canonical checkpoint-validation command cannot pass the same required CI contract tests unnoticed.

The historical Native Character Portfolio task state that first exposed the gap was repaired and archived independently before final delivery. PR #861 did not re-open ADR 0030, change the accepted Option A architecture, or duplicate that task ownership.

## Validation

Exact PR #861 head `f7a76053b8afd1d5f5f5d30bf7752563fdcd642a`:

- Agent Governance run `31226465245`: PASS.
- CI run `31226465252`: PASS.
- `classify-changes`: PASS, including `Validate active task checkpoint contract`.
- `runtime-tests`: PASS.
- aggregate required `test`: PASS.
- full effective diff: exactly three bounded paths.
- exact-head self-review: `PASS_ZERO_MATERIAL_FINDINGS`.
- unresolved review threads: 0.

Resulting `main@48bcff78b119308e7d0d93753f29e5927f511ef5`:

- Agent Governance run `31226632635`: PASS.
- CI run `31226632716`: PASS.
- `classify-changes`: PASS, including the new checkpoint gate.
- `runtime-tests`: PASS.
- aggregate required `test`: PASS.
- Issue #858: closed completed.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-08T01:18:30+02:00
head: f7a76053b8afd1d5f5f5d30bf7752563fdcd642a
branch: fix/OTERYN-20260808-agent-governance-required-gate
pr: 861
status: completed
merge_sha: 48bcff78b119308e7d0d93753f29e5927f511ef5
issue: 858
context_routes:
  - agent-governance
  - ci-repair
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20260808-agent-governance-required-gate-repair.md
  - .github/workflows/ci.yml
  - tests/ci/test_required_test_gate.py
proven:
  - PR #861 exact head f7a76053b8afd1d5f5f5d30bf7752563fdcd642a passed Agent Governance run 31226465245 and CI run 31226465252.
  - The protected classify-changes job now validates every active Markdown task with the canonical checkpoint validator and --require-checkpoint.
  - CI contract regression coverage pins that command inside the protected classifier.
  - PR #861 merged as 48bcff78b119308e7d0d93753f29e5927f511ef5.
  - Resulting-main Agent Governance run 31226632635 passed.
  - Resulting-main CI run 31226632716 passed classify-changes, runtime-tests and aggregate test.
  - Issue #858 closed as completed.
derived:
  - The structural active-task checkpoint failure mode recorded by Issue #858 can no longer remain nominally mergeable while the existing protected required CI contexts are enforced.
unknown: []
conflicts: []
first_failure:
  marker: historical active task checkpoint rejected by Agent Governance while required CI remained green
  evidence: Issue #858; original main Agent Governance run 31223532847 versus successful normal CI recorded in the issue
rejected_hypotheses:
  - Requiring a new branch-protection context was necessary: the existing required classify-changes plus aggregate test can enforce the canonical checkpoint contract fail-closed.
  - Runtime or architecture changes were required: the defect was isolated to governance/task-state enforcement and CI contract coverage.
changed_paths:
  - .github/workflows/ci.yml
  - tests/ci/test_required_test_gate.py
  - docs/agents/tasks/archive/OTERYN-20260808-agent-governance-required-gate-repair.md
validation:
  - command: PR #861 exact-head Agent Governance run 31226465245
    result: PASS
    evidence: checkpoint, task-liveness and Control Room validation completed successfully.
  - command: PR #861 exact-head CI run 31226465252
    result: PASS
    evidence: classify-changes including canonical active-task checkpoint validation, runtime-tests and aggregate test all passed.
  - command: resulting-main Agent Governance run 31226632635
    result: PASS
    evidence: current merged repair task state is governance-valid.
  - command: resulting-main CI run 31226632716
    result: PASS
    evidence: classify-changes, runtime-tests and aggregate test all passed on merge SHA 48bcff78b119308e7d0d93753f29e5927f511ef5.
  - command: exact-head full-diff self-review
    result: PASS
    evidence: PASS_ZERO_MATERIAL_FINDINGS on f7a76053b8afd1d5f5f5d30bf7752563fdcd642a; review 4887233484; zero unresolved threads.
  - command: repair E2E
    result: NOT_APPLICABLE
    evidence: governance/task/CI-only repair creates no executable user or integration journey.
blockers: []
next_action: Archive complete; reopen only if required CI can again permit an invalid active-task checkpoint state or the canonical checkpoint contract changes.
```
