---
task_id: OTERYN-20260909-merge-queue-enqueue-executor
governing_issue: 1363
required_reads: []
search_first:
  - protected Platform main and terminal PR #1384
  - protected META policy
optional_reads: []
---

# OTERYN-20260909 merge-async bridge retirement

## Terminal result

The repository-local Platform Merge Queue bridge is retired. Protected META policy owns the native exact-head submission semantics, and the already-authorized authenticated Codex execution surface can invoke GitHub REST `merge-async` directly without creating a custom Oteryn GitHub App or repository PAT bridge.

PR #1384 integrated through the real protected Merge Queue. Its final effective change deleted the bridge workflow, script, focused bridge test and obsolete runbook, moved the workflow to the retired lifecycle inventory, and reduced the active workflow budget from 54 to 53.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-09T22:17:50Z
head: 4663f931502ae32c0f7bd4b3519886372c78a210
branch: fix/187-native-merge-async-no-app-auth
pr: 1384
status: completed
context_routes:
  - ci-repair
  - integration-control-plane
owned_paths:
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - docs/agents/tasks/archive/OTERYN-20260909-merge-queue-enqueue-executor.md
proven:
  - final PR #1384 candidate had all seven exact-head Platform workflows successful and a clean terminal independent review
  - PR #1384 integrated through native exact-head submission and the real protected Merge Queue
  - merge_group CI run 34411221385 completed successfully with aggregate platform-gate job 102666441905 successful
  - protected Platform main readback is a09f8dfa65d5af7b8a25201b7d551d5849debc4b
  - protected main no longer contains the repository-local merge-queue-enqueue workflow/script/test/runbook
  - no custom Oteryn GitHub App or repository PAT bridge is required for organization-native submission
derived:
  - native submission authority now remains centralized in protected META rather than duplicated in Platform
unknown:
  - final organization-wide provider repins and the dedicated receipt UUID/causal-readback canary tracked by Oteryn/Oteryn#187
conflicts: []
first_failure:
  marker: none
  evidence: terminal Platform bridge-retirement scope is complete
rejected_hypotheses:
  - A custom Oteryn GitHub App is required for native merge-async.
  - A repository PAT bridge is required when an authorized direct native execution surface exists.
  - Direct or generic automatic integration is an acceptable fallback.
changed_paths:
  - .github/workflows/merge-queue-enqueue.yml
  - scripts/github/merge_queue_enqueue.py
  - tests/ci/test_merge_queue_enqueue.py
  - docs/operations/MERGE_QUEUE_EXECUTOR.md
  - docs/agents/CI_WORKFLOW_LIFECYCLE.json
  - docs/agents/tasks/archive/OTERYN-20260909-merge-queue-enqueue-executor.md
validation:
  - command: final exact-head Platform PR workflows
    result: PASS
    evidence: Agent Governance 34409778542; CodeQL 34409778619; Game Auth 34409778546; Edge Security 34409778560; DB Outage 34409778541; Phase 7 34409778612; CI 34409778620
  - command: protected queue integration
    result: PASS
    evidence: merge_group CI 34411221385 and platform-gate 102666441905 succeeded; protected main a09f8dfa65d5af7b8a25201b7d551d5849debc4b
blockers: []
next_action: none
```

## Notes

Historical #1382/#1383 evidence remains as provenance. The failed App-token canary explains why the redundant local bridge was retired; it is not an active execution dependency.
