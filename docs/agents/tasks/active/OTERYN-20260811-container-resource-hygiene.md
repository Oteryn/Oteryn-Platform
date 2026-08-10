---
task_id: OTERYN-20260811-container-resource-hygiene
mode: implementation
status: implementing
project_lane: oteryn-platform-core
---

# OTERYN-20260811-container-resource-hygiene

## Goal

Make temporary container and execution-resource cleanup an explicit mandatory agent lifecycle rule, clean verified obsolete Oteryn portal test containers from Synology without touching persistent data or unrelated workloads, and retain a fail-closed staging hygiene control.

## Acceptance criteria

- [x] Root agent instructions require prompt cleanup of task-owned temporary containers and related disposable execution resources once they are no longer needed.
- [x] Closeout policy requires deterministic resource identity, exact targeted cleanup, post-cleanup verification, and explicit blocker recording when cleanup cannot be completed.
- [x] Shared/persistent services, named volumes, unrelated projects, and blanket Docker prune operations are protected by default.
- [x] Initial guarded live inventory was collected from the Synology runner and made zero removals after detecting an ambiguous portal-owned service.
- [x] Canonical portal identity in the correction workflow includes both trusted Compose files currently present under `deploy/synology`: `compose.yml` and `compose.marketplace.yml`.
- [ ] `oteryn-staging-marketplace-scheduler-1` is preserved and verified running as a canonical portal runtime service by live correction evidence.
- [ ] The exact `portal-authentik-local-test` project is revalidated from live Docker labels and removed container-only; named volumes, networks and images remain untouched.
- [ ] Post-cleanup inventory proves the three verified test containers are absent and all canonical Oteryn staging runtime services remain running.
- [ ] One-time live cleanup bootstrap is removed after evidence is collected; retained workflow remains fail-closed and manual.
- [ ] Task is archived after final exact-head governance/CI validation and terminal closeout.

## Ownership

```yaml
owned_paths:
  - .github/workflows/synology-container-hygiene.yml
  - deploy/synology/compose.yml
  - deploy/synology/compose.marketplace.yml
  - docs/agents/tasks/active/OTERYN-20260811-container-resource-hygiene.md
  - docs/agents/tasks/archive/OTERYN-20260811-container-resource-hygiene.md
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

- No blanket Docker prune operation is authorized.
- No volume, network, image, runner, database data, production resource, Freqtrade, WickHunter, Home Assistant, UniFi or other unrelated workload cleanup is authorized.
- Canonical `oteryn-staging` services must be derived from trusted `main` Compose sources used by deployment.
- The one-time Authentik cleanup may target only containers whose live `com.docker.compose.project` label is exactly `portal-authentik-local-test` and whose service set is exactly `postgresql`, `server`, and `worker`.
- Every target is prevalidated before the first stop; deletion is non-forced and container-only; canonical Oteryn runtime is validated before and after.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-10T23:30:00Z
head: 69cf8243ae0e53e7853b822dd31ab4d8b9975db4
branch: fix/synology-container-hygiene-overlays
pr: 974
status: implementing
context_routes:
  - agent-governance
  - deployment
owned_paths:
  - .github/workflows/synology-container-hygiene.yml
  - deploy/synology/compose.yml
  - deploy/synology/compose.marketplace.yml
  - docs/agents/tasks/active/OTERYN-20260811-container-resource-hygiene.md
  - docs/agents/tasks/archive/OTERYN-20260811-container-resource-hygiene.md
proven:
  - PR #973 merged as bef929a505580d778a93dccc304e3b822e735125 and installed the mandatory execution-resource hygiene policy on main.
  - Live Synology hygiene run 31441629366 reached the Docker host, inventoried containers, found zero stopped cleanup candidates and one ambiguous oteryn-staging candidate, then failed closed before any removal.
  - Run 31441629366 contains no Removing verified stopped portal orphan line; zero containers were removed by the first live attempt.
  - The candidate oteryn-staging-marketplace-scheduler-1 is running and labeled compose project oteryn-staging/service marketplace-scheduler.
  - deploy/synology/compose.marketplace.yml on main defines marketplace-scheduler and extends platform; deploy/synology/scripts/marketplace-staging.sh deploys and health-checks the scheduler using base plus marketplace Compose.
  - Directory inspection of deploy/synology on main shows exactly two Compose files: compose.yml and compose.marketplace.yml.
  - Live inventory shows exactly three running containers in compose project portal-authentik-local-test with services postgresql, server and worker.
  - Current repository code search has no portal-authentik-local-test or Authentik configuration, current PR search has no Authentik PR, and active task inspection found no owner for that test project.
  - Correction workflow on implementation head 69cf8243ae0e53e7853b822dd31ab4d8b9975db4 derives canonical services from both trusted Compose sources and protects marketplace-scheduler as running canonical runtime.
  - Correction workflow prevalidates the exact three-container Authentik test service set before the first stop, uses supported non-forced docker stop --timeout plus docker rm, preserves volumes/networks/images, and validates canonical Oteryn runtime before and after.
  - Docker official CLI documentation confirms docker stop uses -t/--timeout; the unsupported --time spelling was caught during self-review and corrected before live execution.
  - PR #974 is the single follow-up delivery PR for this correction/live closeout stage.
derived:
  - marketplace-scheduler is canonical portal runtime and must be preserved; the first workflow classified it obsolete only because the marketplace Compose overlay was omitted from its source of truth.
  - portal-authentik-local-test is an abandoned local portal test project with high confidence, but live labels will be revalidated immediately before any stop/removal.
unknown:
  - Original creator and creation task of portal-authentik-local-test are not recoverable from current repository code, PR search or active task state.
conflicts: []
first_failure:
  marker: live-cleanup-source-of-truth-incomplete
  evidence: Synology Container Hygiene run 31441629366 classified running marketplace-scheduler as UNSAFE_RUNNING_OBSOLETE_PORTAL_SERVICE and refused all cleanup; repository inspection proves compose.marketplace.yml still owns that service.
rejected_hypotheses:
  - marketplace-scheduler is stale; current main compose.marketplace.yml and marketplace-staging.sh prove it is active.
  - The first live cleanup partially removed containers; logs show zero candidate removals before fail-closed exit.
  - portal-authentik-local-test has an active repository owner; repository code, PR and active-task searches found none.
changed_paths:
  - .github/workflows/synology-container-hygiene.yml
  - docs/agents/tasks/active/OTERYN-20260811-container-resource-hygiene.md
validation:
  - command: Synology Container Hygiene run 31441629366 on bef929a505580d778a93dccc304e3b822e735125
    result: FAIL
    evidence: Expected fail-closed safety outcome for the first live attempt: Docker access/inventory succeeded, one ambiguous portal-owned container caused refusal, and zero containers were removed.
  - command: repository ownership search for marketplace-scheduler
    result: PASS
    evidence: compose.marketplace.yml and marketplace-staging.sh prove canonical deployment ownership.
  - command: repository/PR/active-task ownership search for portal-authentik-local-test and Authentik
    result: PASS
    evidence: Search completed successfully and found no current code/configuration, PR or active task that owns the exact test project.
  - command: exact-head PR #974 CI/governance
    result: NOT_RUN
    evidence: The corrected implementation head is awaiting its current workflow runs; merge is blocked until required exact-head checks pass.
blockers: []
next_action: Validate the exact PR #974 head, inspect full diff/reviews, and merge only if all required checks pass so the exact guarded live cleanup can execute from trusted main.
policy_version: 2
phase: validate
session_id: agent-20260811-container-resource-hygiene-followup
session_role: implementer
execution_mode: github
execution_reason: Correct the retained Synology hygiene source of truth and complete the explicitly authorized bounded container cleanup through the self-hosted runner.
lease_expires_at: 2026-08-11T00:00:00Z
context_pressure: medium
context_growth: stable
context_score: 6
estimate_confidence: high
decomposition_decision: single
decomposition_reason: One bounded correction and live closeout of the same container-resource hygiene task.
validation_level: focused
last_completed_step: Corrected Docker stop syntax before live execution and normalized task validation result values for repository governance.
session_rotation_count: 0
heavy_validation_runs: 1
stale_takeover_count: 0
human_interruptions: 0
```

## Self-review

```yaml
result: PENDING
exact_head: 69cf8243ae0e53e7853b822dd31ab4d8b9975db4
acceptance_checked: true
full_diff_checked: false
negative_paths_checked: true
rollback_checked: true
compatibility_checked: NOT_APPLICABLE
related_prs_checked: true
findings: []
evidence:
  - First live run failed closed with zero removals.
  - Correction adds no prune, forced rm, volume, network or image removal primitive.
  - Docker stop option syntax was verified against current official Docker CLI documentation before merge.
```

## Notes

The first live failure was a safe false-positive caused by incomplete canonical service discovery. No unrelated Synology workload and no Oteryn container was removed.