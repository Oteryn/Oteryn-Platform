# Oteryn Platform Agent Governance live task-liveness audit

## Verdict

`AUDIT_COMPLETE_WITH_FINDING`

The fourth bounded continuous-audit package found one systemic high-confidence, high-risk governance defect: Agent Governance proves checkpoint syntax but does not prove that an active task's PR, branch, archive state, next action or ownership agree with live GitHub state.

No governance tool, workflow, stale task, retained branch, runtime, production system or external repository was changed.

## Scope inspected

- `.github/workflows/agent-governance.yml` triggers, permissions and commands;
- checkpoint parser, contract validation and unit fixtures;
- Control Room state derivation;
- live PR, branch and archive state for three representative active tasks;
- systemic duplicate and ownership search;
- workflow serialization constraints from current PR #542.

## Finding OPA-GOV-0003 — HIGH

**Title:** Agent Governance accepts terminal merged tasks as active.  
**Evidence state:** `PROVEN`  
**Confidence:** high  
**Systemic remediation owner:** Issue #558  
**Concrete Game Gateway symptom:** Issue #555

### Expected

An active task should pass governance only when both are true:

1. its repository checkpoint is structurally valid;
2. its declared PR, branch, archive lifecycle, next action and ownership agree with live GitHub and repository state.

Unavailable required liveness evidence should fail closed or produce an explicit blocked governance result. A retained branch alone must never prove active ownership.

### Actual

Agent Governance runs local tests and `checkpoint.py --tasks docs/agents/tasks/active --require-checkpoint`. The validator checks YAML structure and allowed values only. Control Room similarly normalizes local status and age without live reconciliation.

Three schema-valid active records contradict live state:

- Game Gateway MVP directs agents to merge already-merged PR #122;
- Announcements/Events directs agents to mark already-merged PR #157 ready;
- Download Center directs agents to review and merge already-merged PR #161.

All lack archive records and retain source branches. The current governance gate has no mechanism to reject them.

### Impact

This undermines the repository's collision-prevention model. A worker can obey an obsolete task and attempt invalid continuation, block a current owner, treat a retained branch as a live lease, or ignore apparent ownership because the authoritative surfaces conflict. The defect recurs for every terminal task that is not manually archived.

### Required acceptance

1. Define a versioned active-task liveness model, including the narrow valid pre-archive transition after merge.
2. Reconcile numeric PR identity and state through least-privilege GitHub reads.
3. Reconcile claimed branch existence and task/PR/branch agreement.
4. Reject duplicate active/archive identities.
5. Detect terminal PRs paired with obsolete next actions.
6. Report retained terminal branches for explicit classification without granting ownership.
7. Make API unavailability fail closed when live evidence is required.
8. Surface schema validity and live validity separately in Control Room.
9. Cover positive, negative, boundary, API-failure and prompt-injection fixtures.
10. Preserve existing valid active tasks without a broad format migration.

## Authorization and dependency

Issue #558 is implementation-authorized in principle but remains blocked. The repair changes `.github/workflows/agent-governance.yml` and governance tooling, while active PR #542 is a workflow-bearing package. Repository policy serializes workflow mutations; remediation must revalidate after PR #542 is terminal.

## Audit result

```yaml
audited_head: 968c8adc912beef0119da21a345b0afadc45a494
domain: agent-governance-live-task-liveness
findings:
  critical: 0
  high: 1
  medium: 0
  low: 0
  informational: 0
new_issue: 558
representative_false_active_tasks: 3
governance_repairs: 0
stale_task_repairs: 0
e2e: NOT_APPLICABLE_WITH_REASON
e2e_reason: documentation-only audit with no workflow, tool or task mutation
production_operations: none
external_writes: none
```

## Bounded conclusion

The sample is sufficient to prove the systemic gate defect but is not a complete stale-task remediation inventory. After this package closes, the continuous audit should create deduplicated concrete cleanup owners for additional false-active tasks or select another independent high-risk domain while Issue #558 waits for PR #542.
