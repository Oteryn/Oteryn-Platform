---
task_id: OTERYN-20260805-synology-staging-activation
repository: blakinio/Oteryn-Platform
execution_mode: activation_verification_only
status: completed_historical
closed_by_issue: 876
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/TASK_CLOSEOUT_AUDIT_E2E.md
  - deploy/synology/README.md
---

# OTERYN-20260805 Synology staging activation

## Terminal disposition

`ARCHIVED — FIRST STAGING ACTIVATION ALREADY PROVEN HISTORICALLY`

This task was created with a false current premise: that runner registration, protected Environment setup, immutable Canary image selection, and the first controlled Synology staging deployment had not yet been evidenced.

Issue #876 reconciled that premise against durable protected-main history. The first activation objective is complete historically and no distinct current activation objective remains in this task.

## Proven historical activation evidence

### PR #137 — first successful staging deployment

PR #137 records:

- `Deploy Synology Staging` run `30075926039`, job `89465155605`: success;
- runner `oteryn-synology-staging` with custom label `oteryn-staging`;
- Platform/Gateway image tag `sha-e08548866e6edc70f69eaba40249303b69236625`;
- immutable Canary image `ghcr.io/blakinio/canary@sha256:784e5dbdcc64e311c48c51cd94aa206e2efa1e5eefb2f4ef40170d5aac55031f`;
- Platform, Gateway and Canary health probes passing;
- cleanup of the ephemeral deployment environment file and GHCR logout;
- independent audit run `30088438385` proving all six required services running on expected images with zero restarts, expected loopback-only bindings, and no retained deployment `.env`.

This directly disproves the archived task's old statement that the first controlled deployment had never been evidenced.

### PR #141 — subsequent guarded deployment and live audit

PR #141 records a later successful `Deploy Synology Staging` run `30092611233` on the same dedicated runner with the same immutable Canary digest and a successful independent live audit. This proves the staging deployment mechanism continued to operate after the first activation.

### Later exact-SHA deployment path

PR #267 records a later exact-SHA staging image/deployment verification path. It is supporting evidence that the deployment mechanism continued to be exercised, not proof of present-day runtime state.

## What historical evidence does not prove today

This archive does **not** claim the present value of privileged or mutable external state, including:

- whether the self-hosted runner is online now;
- current GitHub Environment secret/variable values;
- current Synology container/runtime health;
- current image digests running on the NAS;
- current rollback readiness;
- any current production deployment state.

If a future task needs present-state evidence, it must ask explicitly for current-state revalidation. It must not recreate a "first activation" task, re-register the runner, recreate the Environment, or rotate secrets merely because current state is unknown.

## Production boundary

This archive concerns historical staging activation only.

`PRODUCTION_PROVEN=false` remains unchanged. Issue #91 owns the Production Go-Live Gate. Historical staging success must not be promoted into production proof.

## Safety and privilege boundary

Issue #876 performed documentation/lifecycle reconciliation only. It did not:

- register or modify a self-hosted runner;
- modify repository Environments, variables, or secrets;
- dispatch a Synology deployment;
- change `deploy/synology/**` or workflows;
- touch Synology runtime state;
- activate production;
- write to external repositories.

## Closeout checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-08T10:46:00+02:00
status: done
phase: closeout
branch: repair/issue-876
issue: 876
proven:
  - PR 137 records the first successful Synology staging deployment on runner oteryn-synology-staging with label oteryn-staging
  - PR 137 records immutable Canary digest sha256:784e5dbdcc64e311c48c51cd94aa206e2efa1e5eefb2f4ef40170d5aac55031f and healthy Platform/Gateway/Canary probes
  - PR 137 independent audit proves all six required services running with zero restarts, expected bindings/images and no retained deployment env file
  - PR 141 records a subsequent guarded staging deployment and independent live audit
  - the historical first-activation objective is complete and must not remain an active privileged task
unknown:
  - current runner online state
  - current Environment variable and secret values
  - current Synology runtime/image state
  - current rollback readiness
conflicts: []
first_failure:
  marker: none-current
  evidence: the historical first-activation-not-proven premise is superseded by durable deployment evidence
changed_paths:
  - docs/agents/tasks/archive/OTERYN-20260805-synology-staging-activation.md
  - docs/agents/tasks/active/OTERYN-20260805-synology-staging-activation.md
validation:
  - command: lifecycle/evidence reconciliation against protected-main PR 137 and PR 141
    result: PASS
    evidence: first activation and subsequent guarded deployment are directly recorded in durable repository history
  - command: current Synology/runner/Environment verification
    result: NOT_RUN
    evidence: documentation/lifecycle reconciliation only; current privileged state was not required to prove the historical contradiction
blockers: []
next_action: none
```
