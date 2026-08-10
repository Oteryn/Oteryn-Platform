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
- [x] Exact-head implementation governance/CI validation passes; runtime/browser E2E is documented as not applicable to the governance/workflow change itself.

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
- Canonical service identity is derived at execution time from `deploy/synology/compose.yml` checked out from trusted `main`.
- Persistent/named volumes, networks, images, unrelated Compose projects, runner containers, production systems, and external repositories are outside cleanup authority.
- Cleanup must refuse to proceed when canonical runtime health/identity is ambiguous, when a candidate is still running, or when runtime service declarations conflict with trusted `main` Compose.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-10T23:04:57Z
head: ba1719bf6ffa22b78c8aa6d44fc939e3e9ee6093
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
  - Root AGENTS.md requires exact, prompt cleanup of task-owned ephemeral execution resources and protects shared/persistent resources by default.
  - DELIVERY_COMPLETENESS_AND_CLOSEOUT.md policy version 5 makes execution-resource hygiene a readiness and terminal-closeout requirement.
  - EXECUTION_RESOURCE_HYGIENE.md defines deterministic ownership, immediate cleanup timing, fail-closed Docker safety, persistent-data protection, verification evidence, and blocked-cleanup handling.
  - The Synology hygiene workflow inventories all Docker containers using sanitized identity/state/image/Compose metadata but limits deletion authority to verified stopped containers owned by the oteryn-staging Compose project.
  - Live expected service identity is parsed from deploy/synology/compose.yml on trusted main immediately before inventory/cleanup; it is not hard-coded in the destructive path.
  - The cleanup workflow validates canonical runtime services immediately before and after deletion, refuses all cleanup when a portal-owned candidate is unsafe/ambiguous, rechecks candidate ownership/state immediately before removal, and contains no volume/network/image deletion or blanket-prune operation.
  - The one-time automatic cleanup bootstrap is restricted to a main push whose merge message starts exactly with chore(agents): enforce container resource hygiene (#973); retained future operation remains workflow_dispatch only after closeout removal of the bootstrap trigger.
  - PR #973 is the single delivery PR for chore/agent-container-cleanup-policy and its changed-file set is limited to the five owned governance/operation paths.
  - Main remained at d1e2f27d5dee7e3bb650bad38b4d03eaff6d4249 through implementation-head validation, so the validated Compose source did not diverge from the PR base.
  - All seven pull-request workflow runs on implementation head ba1719bf6ffa22b78c8aa6d44fc939e3e9ee6093 completed successfully.
derived:
  - Safe cleanup requires exact ownership/project identity and fail-closed protection for canonical/shared/persistent resources rather than blanket pruning.
  - Deriving expected service identity from trusted main Compose closes the race where a service could be added after PR validation but before live cleanup.
unknown:
  - Current live list of all Synology Docker containers has not yet been collected; that evidence is intentionally deferred until the guarded workflow executes from merged main.
conflicts: []
first_failure:
  marker: initial-exact-head-validation-failed
  evidence: Agent Governance run 31440132255 initially rejected omitted PR identity and Synology Container Hygiene run 31440132129 initially self-matched a forbidden-command marker; both root causes were corrected without weakening safety checks.
rejected_hypotheses:
  - Existing repair-synology-compose-orphans.yml performs general cleanup; inspection proves it only renames exact stale replacement candidates.
  - Global Docker prune or volume pruning is safe; unrelated workloads and persistent Oteryn data coexist on the NAS and are outside this task.
  - A hard-coded expected service set is sufficient for destructive live cleanup; it was replaced with trusted-main Compose-derived identity before readiness.
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
  - command: Agent Governance run 31440712563 on ba1719bf6ffa22b78c8aa6d44fc939e3e9ee6093
    result: PASS
    evidence: Checkpoint tests, live-task liveness and Control Room validation all completed successfully.
  - command: Synology Container Hygiene run 31440712564 on ba1719bf6ffa22b78c8aa6d44fc939e3e9ee6093
    result: PASS
    evidence: Fail-closed static validation passed; live-hygiene correctly remained skipped for the pull_request event.
  - command: CI run 31440712553 on ba1719bf6ffa22b78c8aa6d44fc939e3e9ee6093
    result: PASS
    evidence: Classification, container initialization, dependency audit, formatting, static analysis, tests and container teardown completed successfully.
  - command: Phase 7 Production-Like Validation run 31440712567 on ba1719bf6ffa22b78c8aa6d44fc939e3e9ee6093
    result: PASS
    evidence: Production-like exact-SHA validation completed successfully.
  - command: Platform DB Outage Validation run 31440712578 on ba1719bf6ffa22b78c8aa6d44fc939e3e9ee6093
    result: PASS
    evidence: Production-like Platform DB outage/recovery validation completed successfully.
  - command: Edge Security Emulation run 31440712641 on ba1719bf6ffa22b78c8aa6d44fc939e3e9ee6093
    result: PASS
    evidence: Exact-head edge security emulation completed successfully.
  - command: Game Auth Ticket Concurrency run 31440712568 on ba1719bf6ffa22b78c8aa6d44fc939e3e9ee6093
    result: PASS
    evidence: Exact-head game-auth concurrency validation completed successfully.
blockers: []
next_action: After the final checkpoint-only head passes its selected checks, merge PR #973 with the exact one-time bootstrap title, inspect the live Synology cleanup evidence, then archive this task and remove the one-time push bootstrap.
policy_version: 2
phase: ready
session_id: agent-20260811-container-resource-hygiene
session_role: implementer
execution_mode: github
execution_reason: Narrow governance and Synology operational-control change through the GitHub connector and existing self-hosted runner.
lease_expires_at: 2026-08-10T23:49:57Z
context_pressure: low
context_growth: stable
context_score: 6
estimate_confidence: high
decomposition_decision: single
decomposition_reason: One cohesive resource-lifecycle policy plus the bounded runner control needed to apply it safely to the requested Synology cleanup.
validation_level: focused
last_completed_step: Completed full exact-head implementation validation and self-review; only checkpoint-only final checks remain before merge.
session_rotation_count: 0
heavy_validation_runs: 1
stale_takeover_count: 0
human_interruptions: 0
```

## Self-review

```yaml
result: PASS
exact_head: ba1719bf6ffa22b78c8aa6d44fc939e3e9ee6093
acceptance_checked: true
full_diff_checked: true
negative_paths_checked: true
rollback_checked: true
compatibility_checked: NOT_APPLICABLE
related_prs_checked: true
findings: []
evidence:
  - Exact changed-file set contains only the five task-owned paths.
  - No open review threads or requested changes exist on PR #973.
  - No cleanup primitive can delete volumes, networks, images, unrelated projects, running or ambiguous portal candidates, or canonical active runtime services.
```

## Notes

The live cleanup is separately authorized by the repository owner in this task and remains limited to the explicit container-only safety boundary above. The workflow deliberately preserves unrelated Synology workloads even when they are stopped or old.
