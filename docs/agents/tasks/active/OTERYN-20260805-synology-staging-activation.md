---
task_id: OTERYN-20260805-synology-staging-activation
repository: blakinio/Oteryn-Platform
execution_mode: activation_verification_only
branch: none
pull_request: none
status: blocked
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/TASK_CLOSEOUT_AUDIT_E2E.md
  - deploy/synology/README.md
---

# OTERYN-20260805-synology-staging-activation

## Goal

Preserve the privileged staging activation gates after repository implementation completion without claiming deployment assets or workflows: register the isolated runner, configure the `synology-staging` Environment, select a compatible prebuilt Canary image and perform the first controlled deployment with direct evidence.

## Acceptance criteria

- [ ] The dedicated self-hosted runner is registered without default labels and with the required custom staging label.
- [ ] The `synology-staging` Environment contains the required protected variables and secrets without exposing values in Git or logs.
- [ ] A compatible prebuilt Canary image containing the required issuer is selected by immutable reference.
- [ ] The first controlled staging deployment runs from trusted `main` and passes health verification.
- [ ] Rollback readiness and the resulting running image revisions are directly recorded.
- [ ] Production activation remains outside this task.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260805-synology-staging-activation.md
modules:
  - activation evidence only
dependencies:
  - Platform PR 127 merged as 51e7bfc21d493a6ca15591ce4ea2a78158c7b7d5
  - a compatible immutable Canary runtime image
blockers:
  - runner registration state is not directly verified
  - required GitHub Environment variables and secrets are not directly verified
  - compatible Canary image reference is not confirmed
  - first controlled deployment has not been evidenced
cross_repository_tasks:
  - none claimed by this record
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-05T20:59:00Z
head: 51e7bfc21d493a6ca15591ce4ea2a78158c7b7d5
branch: none
pr: none
status: blocked
context_routes:
  - deployment-operations
  - security
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260805-synology-staging-activation.md
proven:
  - Repository-owned staging package PR 127 merged as 51e7bfc21d493a6ca15591ce4ea2a78158c7b7d5 from final head ab16b33ed5fecccdf9386310cc1eb09328b204b4.
  - Repository implementation was designed to build images off-NAS and deploy only trusted prebuilt images.
  - Production deployment and activation were not performed by PR 127.
  - The historical implementation task has been archived and no longer owns deployment or workflow paths.
derived:
  - Activation requires direct privileged environment and runtime evidence; repository documentation alone cannot prove completion.
unknown:
  - current registration and online state of the dedicated staging runner
  - exact protected Environment variable and secret configuration
  - compatible immutable Canary image reference
  - first deployment and health-check result
  - resulting running image digests and rollback evidence
conflicts: []
first_failure:
  marker: staging-activation-not-proven
  evidence: repository implementation merged without external runner, Environment and runtime evidence
rejected_hypotheses:
  - infer activation from green repository image-build checks
  - retain ownership of deploy/synology or workflow files while waiting for external evidence
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260805-synology-staging-activation.md
validation:
  - command: repository lifecycle reconciliation
    result: PASS
    evidence: completed implementation and unresolved external activation are separated without deployment/workflow ownership
  - command: first controlled Synology staging deployment
    result: NOT_RUN
    evidence: runner, Environment configuration and compatible Canary image are not directly verified
blockers:
  - authorized external staging configuration and runtime evidence are required
next_action: Verify or configure the isolated runner and synology-staging Environment, select an immutable compatible Canary image, then execute one controlled trusted-main staging deployment and record health, image and rollback evidence without enabling production.
```
