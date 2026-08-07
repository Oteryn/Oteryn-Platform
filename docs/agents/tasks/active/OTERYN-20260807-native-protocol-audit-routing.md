---
task_id: OTERYN-20260807-native-protocol-audit-routing
issue: 829
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: true
status: validating
risk: high
validation_intensity: HEIGHTENED
execution_mode: github_only
branch: repair/issue-829
base_branch: main
pr: 834
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
- [x] A native-protocol producer correction with runtime changes still requires the canonical active producer task record for every producer-path routing case.
- [x] Native-protocol producer runtime changes outside the existing allowlist still fail closed when the canonical producer task is the producer signal.
- [x] Focused deterministic regression fixtures cover unrelated-change PASS, missing-task FAIL, escaped-runtime FAIL, native-doc-only PASS, producer-task-only valid/escaped cases, and valid producer PASS.
- [x] Existing architecture, security/downgrade, parser/schema, Canary regression, rollout and rollback audits remain present and unchanged in meaning.
- [ ] Required exact-head CI and Agent Governance pass; workflow-specific validation passes.
- [x] E2E is `NOT_APPLICABLE`: this task changes CI routing/validation code only and no product/runtime user journey.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T18:53:00+02:00
head: ac47473d2a6bea5cd82abb4121d5f0aa14566392
branch: repair/issue-829
pr: 834
status: validating
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
  - Post-merge acceptance review found that the canonical producer task path itself was omitted from the merged producer-signal classifier and workflow path triggers.
  - Follow-up candidate ca7674cb9aa1111907111c2a30240fbec32a9e18 adds the exact canonical producer task as both a workflow trigger and producer signal without promoting generic runtime paths.
  - Focused local regressions pass for unrelated contract/runtime, unrelated architecture/runtime, documentation-only, missing task, missing task file, task-led escaped runtime, task-led governed runtime, and native-document-led governed runtime.
  - Native protocol contract audits run 31199222578 passed all five lanes on the exact code candidate, including Audit 1 change-boundary regressions.
  - Main branch protection requires classify-changes and test.
derived:
  - The narrow producer-task trigger/signal closes the post-merge gap without recreating the Issue #829 false positive for unrelated contracts/runtime.
unknown: []
conflicts: []
first_failure:
  marker: post-merge-acceptance-review-04faba
  evidence: merged is_native_producer_signal() enumerated native docs/architecture/producer operation/prompt/fixtures/validator but excluded PRODUCER_TASK, while the workflow paths also excluded PRODUCER_TASK.
rejected_hypotheses:
  - Removing the producer ownership check would fix the false positive; rejected because it would weaken a real native-protocol governance boundary.
  - Narrowing the whole workflow trigger to only native files is sufficient; rejected because Audits 2-5 intentionally validate broader contract and architecture changes and must retain their existing trigger surface.
  - Treating every allowed runtime path as a producer signal is safe; rejected because broad GameAuth/gateway test/runtime prefixes can contain unrelated work and would recreate noisy false positives.
changed_paths:
  - .github/workflows/native-protocol-contract-audits.yml
  - scripts/validate_native_protocol_change_boundary.py
  - scripts/test_native_protocol_change_boundary.py
  - docs/agents/tasks/active/OTERYN-20260807-native-protocol-audit-routing.md
  - docs/agents/tasks/archive/OTERYN-20260807-native-protocol-audit-routing.md
validation:
  - command: focused native-protocol change-boundary fixtures
    result: PASS
    evidence: 9 deterministic unittest cases passed locally before publishing the candidate.
  - command: Native protocol contract audits / run 31199222578
    result: PASS
    evidence: exact code candidate ca7674cb9aa1111907111c2a30240fbec32a9e18; Audits 1 through 5 all passed and Audit 1 ran the focused change-boundary regressions successfully.
  - command: Agent Governance / run 31199335627
    result: FAIL
    evidence: checkpoint validation failed only because a temporary validation item used unsupported result IN_PROGRESS; this checkpoint replaces it with the now-terminal PASS evidence.
blockers: []
next_action: validate the current PR #834 head, perform exact-head self-review, mark ready and merge only after required CI, Agent Governance and Native protocol audit evidence pass
```

## Design

Audit 1 separates two concerns:

1. canonical native-protocol architecture invariants always run when the workflow is triggered;
2. producer runtime ownership enforcement runs only when the PR changes a specific native-protocol producer signal such as the canonical gameplay contract/proto/migration, native protocol ADR/threat/rollout documents, producer operation/prompt, native fixtures, canonical contract validator, or the canonical active producer task record.

The generic `docs/contracts/**` and `docs/architecture/**` workflow triggers remain intact for Audits 2-5. The exact canonical producer task path is also a trigger so producer-task-led corrections cannot bypass Audit 1. Generic runtime paths are not promoted to producer signals.

## Safety

Repository-only CI/governance repair. No production deployment, protected-environment operation, secret access, runtime product behavior, Canary mutation or external-repository mutation is authorized.
