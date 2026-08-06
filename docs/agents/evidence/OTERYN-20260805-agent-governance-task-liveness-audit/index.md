# Agent Governance live task-liveness audit evidence

## Identity

- Programme: `OTERYN_PLATFORM_CONTINUOUS_AUDIT`
- Task: `OTERYN-20260805-agent-governance-task-liveness-audit`
- Repository: `blakinio/Oteryn-Platform`
- Audited main: `968c8adc912beef0119da21a345b0afadc45a494`
- Finding: `OPA-GOV-0003`
- Finding Issue: #558

## Enforced governance path

| Source | Proven behavior |
|---|---|
| `.github/workflows/agent-governance.yml` | Checks out contents, runs local checkpoint unit tests, then validates every file under `docs/agents/tasks/active`; no GitHub liveness query exists. |
| Workflow permissions | `contents: read` only; no declared pull-request read permission or live-state reconciliation step. |
| `tools/agents/checkpoint.py` | Parses local fenced YAML and validates required fields, allowed statuses/results, non-empty values and evidence-list separation. It does not resolve PR, branch, archive, lease or ownership state. |
| `tools/agents/test_checkpoint.py` | Uses local temporary-file fixtures only and explicitly accepts `status: completed` in an active task-shaped file. |
| `tools/agents/control_room.py` | Derives READY/RUNNING/WAITING/STALE/DONE from local status and age. It does not compare the task with live PR or branch state. |

## Proven false-active outcomes

| Active task | Recorded action | Live PR truth | Archive | Branch |
|---|---|---|---|---|
| `OTERYN-20260722-game-gateway-mvp` | verify and merge PR #122 | merged as `8006534108d835474dadd208b0ec934e4a12528b` | missing | retained |
| `OTERYN-20260724-announcements-events` | mark PR #157 ready | merged as `82a415c5de5727d15186cf0d0d79744fb498e187` | missing | retained |
| `OTERYN-20260724-download-center` | review and merge PR #161 | merged as `79858de3949e8d5969207357e6fb92bfaada481f` | missing | retained |

The first concrete symptom is separately owned by Issue #555. The latter two prove the defect is not isolated to one task.

## Why current checks remain green

```yaml
schema_validity:
  required_fields_present: true
  allowed_status_ready: true
  next_action_non_empty: true
  evidence_lists_well_formed: true
live_validity:
  recorded_pr_open: false
  recorded_next_action_executable: false
  archive_lifecycle_complete: false
  retained_branch_classified: false
current_gate_result: passes_schema_without_live_truth
```

## Duplicate and ownership search

- Open and closed Issue searches for live checkpoint validation, merged-PR active tasks, active/archive contradictions and Agent Governance liveness found no systemic root-cause owner.
- Issue #555 is one concrete task cleanup and explicitly does not modify governance tooling.
- Active PR #542 remains a workflow-bearing owner; Issue #558 is blocked until that serialized workflow package is terminal.

## Evidence classification

```yaml
finding_id: OPA-GOV-0003
severity: high
confidence: high
evidence_state: PROVEN
sampled_false_active_tasks: 3
systemic_gate_gap: true
governance_tool_mutation_by_audit: none
stale_task_mutation_by_audit: none
production_mutation: none
external_repository_write: none
```

## Validation boundary

This package proves the missing enforcement layer and representative outcomes. It does not claim an exhaustive stale-task inventory and does not repair any affected task. Remediation must add fail-closed live-state validation without broad rewriting of valid active records.
