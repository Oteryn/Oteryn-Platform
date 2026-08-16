# OTERYN Historical Work Reconciliation — terminal prompt tombstone

```yaml
status: TERMINAL_DO_NOT_RUN
issue: 1072
terminal_task: docs/agents/tasks/archive/OTERYN-20260815-historical-work-reconciliation.md
terminal_registry: docs/agents/HISTORICAL_WORK_RECONCILIATION_REGISTRY.json
durable_policy:
  - docs/architecture/adr/0037-terminal-source-branch-lifecycle.md
  - docs/architecture/adr/0039-historical-work-canonicalization-and-managed-recovery.md
prompt_contract:
  version: 2.0-terminal
  changed_surfaces:
    - worker_template
    - repository_routing
    - continuation_rule
  objective: prevent the completed one-time Issue 1072 historical reconciliation from being restarted while preserving durable provenance and routing future branch hygiene through steady-state governance
  baseline_version: historical_work_reconciliation_execution_v1
  eval_suite: docs/agents/evals/prompt-contract-v1.json
  rollback_version: git_history_only_do_not_reactivate_without_owner_decision
```

## Terminal status

**Issue #1072 is terminal. Do not execute this prompt as a worker instruction.**

The one-time Historical Work Reconciliation completed after exact review, merge, trusted-main Git-ref lifecycle E2E, task archival and source-branch closeout. The terminal registry records the reviewed historical refs and their applied dispositions. The old execution prompt is retained at this path only so historical references resolve to an explicit tombstone instead of a stale runnable instruction.

The former active task path and former execution branch are historical identifiers and provide no current ownership, permission or continuation authority.

## Current steady-state routing

Future branch/ref hygiene is not a continuation of Issue #1072.

Use current repository governance instead:

1. ADR 0037 controls terminal source-branch lifecycle and exact-head deletion safety.
2. ADR 0039 controls historical-work canonicalization and managed recovery semantics.
3. The repository Historical Branch Audit / branch-hygiene controls detect new unexplained branch debt and ownership conflicts.
4. A new concrete governance defect is handled through a new/reused live Issue/task/PR under current repository ownership and remediation rules.
5. `OTERYN_PORTAL_COMPLETION` consumes live branch/task/PR truth but does not own historical ref retention or deletion.

Do not reopen or restart Issue #1072 merely because a new branch-hygiene finding appears. A deliberate reopening of the completed one-time reconciliation would require an explicit current owner decision plus evidence that the terminal historical result itself is invalid or incomplete.

## Authority boundary

This tombstone grants no repository mutation, branch deletion, production/protected-environment action, external/server repository access, credential access, payment action or owner-funded AI/model use.

Any future operation derives authority from current system/owner instructions and current trusted repository governance, not from the historical text or Git history of this prompt.

## Historical provenance

For the completed reconciliation result, read only as needed:

- `docs/agents/tasks/archive/OTERYN-20260815-historical-work-reconciliation.md`;
- `docs/agents/HISTORICAL_WORK_RECONCILIATION_REGISTRY.json`;
- ADR 0037 and ADR 0039;
- terminal Issue #1072 and its merged PR provenance.

Historical prompt text remains recoverable from Git history. It must not be copied back into active instructions unless a new authorized task intentionally supersedes this tombstone.
