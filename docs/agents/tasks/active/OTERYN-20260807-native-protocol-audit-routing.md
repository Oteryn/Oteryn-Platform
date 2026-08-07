---
task_id: OTERYN-20260807-native-protocol-audit-routing
issue: 829
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: true
status: implementing
risk: high
validation_intensity: HEIGHTENED
execution_mode: github_only
branch: repair/issue-829
base_branch: main
pr: pending
production_activation_authorized: false
cross_repository_mutation_authorized: false
owned_paths:
  - .github/workflows/native-protocol-contract-audits.yml
  - scripts/validate_native_protocol_change_boundary.py
  - scripts/test_native_protocol_change_boundary.py
  - docs/agents/tasks/active/OTERYN-20260807-native-protocol-audit-routing.md
modules:
  - ci-native-protocol-audit-routing
coordination_key: ci:native-protocol-audit-routing
blockers: []
cross_repository_tasks: []
---

# OTERYN-20260807 native protocol audit routing

## Goal

Repair Issue #829 so the native-protocol architecture boundary audit does not reject unrelated runtime changes merely because an unrelated file under `docs/contracts/**` or `docs/architecture/**` changed, while true native-protocol producer corrections remain fail-closed.

## Acceptance criteria

- [x] Unrelated contract + unrelated runtime changes do not invoke native-protocol producer ownership enforcement.
- [ ] A native-protocol producer correction with runtime changes still requires the canonical active producer task record for every producer-path routing case.
- [ ] Native-protocol producer runtime changes outside the existing allowlist still fail closed when the canonical producer task is the producer signal.
- [ ] Focused deterministic regression fixtures cover unrelated-change PASS, missing-task FAIL, escaped-runtime FAIL, native-doc-only PASS, producer-task-only valid/escaped cases, and valid producer PASS.
- [x] Existing architecture, security/downgrade, parser/schema, Canary regression, rollout and rollback audits remain present and unchanged in meaning.
- [ ] Required exact-head CI and Agent Governance pass; workflow-specific validation passes.
- [x] E2E is `NOT_APPLICABLE`: this task changes CI routing/validation code only and no product/runtime user journey.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T18:45:00+02:00
head: 04faba107218fba7aa43325270ccb19226358171
branch: repair/issue-829
pr: pending
status: implementing
context_routes:
  - agent-governance
  - testing
  - ci-repair
owned_paths:
  - .github/workflows/native-protocol-contract-audits.yml
  - scripts/validate_native_protocol_change_boundary.py
  - scripts/test_native_protocol_change_boundary.py
  - docs/agents/tasks/active/OTERYN-20260807-native-protocol-audit-routing.md
proven:
  - Issue #829 originally reproduced because generic docs/contracts or docs/architecture triggering caused Audit 1 to classify every runtime path in the PR as a native-protocol producer correction.
  - PR #832 merged as 04faba107218fba7aa43325270ccb19226358171 and fixed the proven unrelated-contract/runtime false positive while preserving Audits 2-5.
  - Post-merge acceptance review found that the canonical producer task path itself is not included in the merged producer-signal classifier and is not a workflow path trigger.
  - A change set containing the canonical producer task plus governed or escaped producer runtime can therefore be classified NOT_APPLICABLE unless another enumerated native contract/architecture signal happens to be present.
  - The canonical producer task is itself a producer governance path and must route into the existing task/runtime allowlist checks without making generic runtime a producer signal.
  - Main branch protection requires classify-changes and test.
derived:
  - Adding the exact canonical producer task path as both a workflow trigger and an explicit producer signal closes the discovered route without recreating the Issue #829 false positive for unrelated contracts/runtime.
unknown: []
conflicts: []
first_failure:
  marker: post-merge-acceptance-review-04faba
  evidence: merged is_native_producer_signal() enumerates native docs/architecture/producer operation/prompt/fixtures/validator but excludes PRODUCER_TASK, while the workflow paths also exclude PRODUCER_TASK.
rejected_hypotheses:
  - Removing the producer ownership check would fix the false positive; rejected because it would weaken a real native-protocol governance boundary.
  - Narrowing the whole workflow trigger to only native files is sufficient; rejected because Audits 2-5 intentionally validate broader contract and architecture changes and must retain their existing trigger surface.
  - Treating every allowed runtime path as a producer signal is safe; rejected because broad GameAuth/gateway test/runtime prefixes can contain unrelated work and would recreate noisy false positives.
changed_paths:
  - .github/workflows/native-protocol-contract-audits.yml
  - scripts/validate_native_protocol_change_boundary.py
  - scripts/test_native_protocol_change_boundary.py
  - docs/agents/tasks/active/OTERYN-20260807-native-protocol-audit-routing.md
validation:
  - command: Native protocol contract audits / run 31198529483 on PR #832 head 114f0c4ff59c83a86277a895609ccd44aa5226b8
    result: PASS
    evidence: all five existing audit lanes passed before the post-merge acceptance gap was identified.
  - command: Agent Governance / run 31198527556 on PR #832 head 114f0c4ff59c83a86277a895609ccd44aa5226b8
    result: PASS
    evidence: governance passed for the first repair candidate.
  - command: post-merge acceptance review
    result: FAIL
    evidence: canonical producer task is neither a producer signal nor a workflow trigger on merged main 04faba107218fba7aa43325270ccb19226358171.
blockers: []
next_action: commit the narrow producer-task trigger/signal regression repair, open the single follow-up PR for reopened Issue #829, validate exact-head Native protocol audits, CI and Agent Governance, then merge only after self-review PASS
```

## Design

Audit 1 separates two concerns:

1. canonical native-protocol architecture invariants always run when the workflow is triggered;
2. producer runtime ownership enforcement runs only when the PR changes a specific native-protocol producer signal such as the canonical gameplay contract/proto/migration, native protocol ADR/threat/rollout documents, producer operation/prompt, native fixtures, canonical contract validator, or the canonical active producer task record.

The generic `docs/contracts/**` and `docs/architecture/**` workflow triggers remain intact for Audits 2-5. The exact canonical producer task path is also a trigger so producer-task-led corrections cannot bypass Audit 1. Generic runtime paths are not promoted to producer signals.

## Safety

Repository-only CI/governance repair. No production deployment, protected-environment operation, secret access, runtime product behavior, Canary mutation or external-repository mutation is authorized.
