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

- [ ] Root agent instructions require prompt cleanup of task-owned temporary containers and related disposable execution resources once they are no longer needed.
- [ ] Closeout policy requires deterministic resource identity, exact targeted cleanup, post-cleanup verification, and explicit blocker recording when cleanup cannot be completed.
- [ ] Shared/persistent services, named volumes, unrelated projects, and blanket Docker prune operations are protected by default.
- [ ] A retained Synology workflow can inventory Docker state without exposing environment/secrets and can remove only fail-closed, verified Oteryn staging orphan containers.
- [ ] The workflow never removes volumes, networks, images, unrelated-project containers, or canonical active portal services.
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
updated_at: 2026-08-10T22:48:25Z
head: d1e2f27d5dee7e3bb650bad38b4d03eaff6d4249
branch: chore/agent-container-cleanup-policy
pr: none
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
  - Root AGENTS.md requires task closeout but does not explicitly define task-owned container cleanup safety or timing.
  - DELIVERY_COMPLETENESS_AND_CLOSEOUT.md currently ends with only the generic instruction to remove temporary execution scaffolding.
  - deploy/synology/compose.yml defines the Oteryn staging services mariadb, redis, platform, canary, tls-init, internal-proxy, and gateway.
  - Existing repair-synology-compose-orphans.yml repairs historical Compose container names and is not a general orphan-cleanup workflow.
  - The Home Assistant execution connector failed twice with a network connection error, while the repository exposes a dedicated oteryn-staging GitHub Actions runner with Docker access.
  - No open PR specifically owns Synology container cleanup or this agent-governance policy.
derived:
  - Safe cleanup requires exact ownership/project identity and fail-closed protection for canonical/shared/persistent resources rather than blanket pruning.
  - A dedicated retained inventory/cleanup workflow is the smallest auditable path to inspect and clean the Synology Docker host through the already-authorized repository runner.
unknown:
  - Current live list of all Synology Docker containers has not yet been collected.
conflicts: []
first_failure:
  marker: direct-home-assistant-synology-execution-unavailable
  evidence: Home Assistant connector returned Connection failed on two shell-service discovery attempts; no Docker mutation occurred through that path.
rejected_hypotheses:
  - Existing repair-synology-compose-orphans.yml performs general cleanup; inspection proves it only renames exact stale replacement candidates.
  - Global docker system prune or volume pruning is safe; unrelated workloads and persistent Oteryn data coexist on the NAS and are outside this task.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260811-container-resource-hygiene.md
validation:
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: Governance and operational-control workflow change; no portal application route, frontend, API, schema, or player-facing behavior is modified.
  - command: exact-head Agent Governance and repository-selected CI
    result: NOT_RUN
    evidence: Coherent policy/workflow implementation is not yet committed.
blockers: []
next_action: Add the mandatory resource-hygiene contract and guarded Synology inventory/cleanup workflow, then validate the exact PR head.
policy_version: 2
phase: implement
session_id: agent-20260811-container-resource-hygiene
session_role: implementer
execution_mode: github
execution_reason: Narrow governance and Synology operational-control change through the GitHub connector and existing self-hosted runner.
lease_expires_at: 2026-08-10T23:33:25Z
context_pressure: low
context_growth: stable
context_score: 4
estimate_confidence: high
decomposition_decision: single
decomposition_reason: One cohesive resource-lifecycle policy plus the bounded runner control needed to apply it safely to the requested Synology cleanup.
validation_level: focused
last_completed_step: Verified the current cleanup-policy gap, canonical Synology staging service set, existing runner path, and limits of the historical name-repair workflow.
session_rotation_count: 0
heavy_validation_runs: 0
stale_takeover_count: 0
human_interruptions: 0
```

## Notes

The live cleanup is a separately authorized staging-host operation. Repository merge authority does not broaden that operation beyond the explicit container-only scope above.