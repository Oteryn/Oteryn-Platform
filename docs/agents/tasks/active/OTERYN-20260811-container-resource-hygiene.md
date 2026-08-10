---
task_id: OTERYN-20260811-container-resource-hygiene
mode: implementation
status: implementing
project_lane: oteryn-platform-core
---

# OTERYN-20260811-container-resource-hygiene

## Goal

Make temporary container and execution-resource cleanup an explicit mandatory agent lifecycle rule, and add a fail-closed Synology staging inventory/cleanup path that can remove only verified obsolete Oteryn portal containers without touching persistent data or unrelated workloads.

## Acceptance criteria

- [x] Root agent instructions require prompt cleanup of task-owned temporary containers and related disposable execution resources once they are no longer needed.
- [x] Closeout policy requires deterministic resource identity, exact targeted cleanup, post-cleanup verification, and explicit blocker recording when cleanup cannot be completed.
- [x] Shared/persistent services, named volumes, unrelated projects, and blanket Docker prune operations are protected by default.
- [x] A retained Synology workflow can inventory Docker state without exposing environment/secrets and can remove only fail-closed, verified Oteryn staging orphan containers.
- [x] The workflow never removes volumes, networks, images, unrelated-project containers, or canonical active portal services.
- [ ] Exact-head governance/CI validation passes; runtime/browser E2E is documented as not applicable to the governance/workflow change itself.

## Ownership

```yaml
owned_paths:
  - AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
  - .github/workflows/synology-container-hygiene.yml
  - docs/agents/tasks/active/OTERYN-20260811-container-resource-hygiene.md
modules:
  - agent-governance
  - synology-operations
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - none
```

## Safety boundary

- The live cleanup scope is limited to containers on the `oteryn-staging` Compose project used by the Oteryn portal staging stack.
- Canonical services are `mariadb`, `redis`, `platform`, `canary`, `tls-init`, `internal-proxy`, and `gateway`.
- Persistent/named volumes, networks, images, unrelated Compose projects, runner containers, production systems, and external repositories are outside cleanup authority.
- Cleanup must refuse to proceed when canonical runtime health/identity is ambiguous or when a candidate is still running.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-10T22:56:00Z
head: 22741b1b0cd7cbe6792407e108cadce80bfd8828
branch: chore/agent-container-cleanup-policy
pr: 973
status: implementing
context_routes:
  - agent-governance
  - deployment
owned_paths:
  - AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
  - .github/workflows/synology-container-hygiene.yml
  - docs/agents/tasks/active/OTERYN-20260811-container-resource-hygiene.md
proven:
  - Root AGENTS.md now requires exact, prompt cleanup of task-owned ephemeral execution resources and protects shared/persistent resources by default.
  - DELIVERY_COMPLETENESS_AND_CLOSEOUT.md policy version 5 now makes execution-resource hygiene a readiness and terminal-closeout requirement.
  - EXECUTION_RESOURCE_HYGIENE.md defines deterministic ownership, immediate cleanup timing, fail-closed Docker safety, persistent-data protection, verification evidence, and blocked-cleanup handling.
  - The Synology hygiene workflow inventories all Docker containers using sanitized identity/state/image/Compose metadata but limits deletion authority to verified stopped containers owned by the oteryn-staging Compose project.
  - The cleanup workflow validates all canonical runtime services immediately before and after deletion, refuses any unsafe/ambiguous portal-owned candidate, and contains no volume/network/image deletion or blanket-prune operation.
  - PR #973 is the single delivery PR for chore/agent-container-cleanup-policy.
  - Initial Agent Governance checkpoint validation passed structurally, but live task liveness rejected the task because the newly opened PR #973 had not yet been persisted in the checkpoint.
  - Initial Synology Container Hygiene static validation failed because the forbidden-pattern test contained its own literal docker-rmi marker and therefore matched itself; the cleanup implementation itself did not invoke that command.
derived:
  - Persisting PR #973 in this checkpoint resolves the live ownership inconsistency detected by Agent Governance.
  - Splitting the docker-rmi forbidden marker into concatenated source fragments preserves the exact runtime safety check without self-matching the workflow source.
  - Safe cleanup requires exact ownership/project identity and fail-closed protection for canonical/shared/persistent resources rather than blanket pruning.
unknown:
  - Current live list of all Synology Docker containers has not yet been collected.
conflicts: []
first_failure:
  marker: initial-exact-head-validation-failed
  evidence: Agent Governance run 31440132255 rejected omitted live PR identity; Synology Container Hygiene run 31440132129 failed its self-matching forbidden-pattern guard. Both root causes are repaired in the next task-only correction commit.
rejected_hypotheses:
  - Existing repair-synology-compose-orphans.yml performs general cleanup; inspection proves it only renames exact stale replacement candidates.
  - Global docker system prune or volume pruning is safe; unrelated workloads and persistent Oteryn data coexist on the NAS and are outside this task.
  - The initial Synology hygiene failure proves an unsafe cleanup primitive exists; inspection proves the failing literal was inside the static validator's own forbidden-pattern list.
changed_paths:
  - .github/workflows/synology-container-hygiene.yml
  - AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
  - docs/agents/tasks/active/OTERYN-20260811-container-resource-hygiene.md
validation:
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: Governance and operational-control workflow change; no portal application route, frontend, API, schema, or player-facing behavior is modified.
  - command: Agent Governance run 31440132255 on 22741b1b0cd7cbe6792407e108cadce80bfd8828
    result: FAIL
    evidence: Structural checkpoint validation passed; live liveness rejected the task because PR #973 was opened after the initial checkpoint and had not yet been persisted.
  - command: Synology Container Hygiene run 31440132129 on 22741b1b0cd7cbe6792407e108cadce80bfd8828
    result: FAIL
    evidence: Static guard self-matched its literal docker-rmi forbidden marker; live-hygiene job correctly remained skipped on a pull-request event.
blockers: []
next_action: Validate the corrected exact PR head, inspect the full diff/review state, and merge only if all required checks pass.
policy_version: 2
phase: validate
session_id: agent-20260811-container-resource-hygiene
session_role: implementer
execution_mode: github
execution_reason: Narrow governance and Synology operational-control change through the GitHub connector and existing self-hosted runner.
lease_expires_at: 2026-08-10T23:41:00Z
context_pressure: low
context_growth: stable
context_score: 5
estimate_confidence: high
decomposition_decision: single
decomposition_reason: One cohesive resource-lifecycle policy plus the bounded runner control needed to apply it safely to the requested Synology cleanup.
validation_level: focused
last_completed_step: Repaired the first CI failures by persisting PR #973 identity and removing the static validator self-match without weakening its forbidden-command check.
session_rotation_count: 0
heavy_validation_runs: 0
stale_takeover_count: 0
human_interruptions: 0
```

## Notes

The live cleanup is a separately authorized staging-host operation. Repository merge authority does not broaden that operation beyond the explicit container-only scope above.