---
task_id: OTERYN-20260811-container-resource-hygiene
mode: implementation
status: implementing
project_lane: oteryn-platform-core
---

# OTERYN-20260811-container-resource-hygiene

## Goal

Make temporary execution-resource cleanup mandatory for agents, remove verified obsolete portal test containers from Synology without touching persistent or unrelated resources, and leave a fail-closed manual hygiene control for the Oteryn staging stack.

## Acceptance criteria

- [x] Agent governance requires prompt task-owned temporary-resource cleanup and exact ownership-scoped deletion.
- [x] Blanket prune, unrelated workloads, canonical services, runners, persistent/named volumes, durable data and shared infrastructure are protected by default.
- [x] First Synology inventory failed closed with zero removals when canonical service discovery was incomplete.
- [x] Canonical discovery was corrected to include the marketplace Compose overlay and `marketplace-scheduler` was preserved.
- [x] The exact `portal-authentik-local-test` service set was revalidated live immediately before mutation.
- [x] Exactly three obsolete Authentik local-test containers were stopped and removed; no other removal occurred.
- [x] Named volumes, networks, images and unrelated workloads were preserved.
- [x] Canonical Oteryn runtime, including `marketplace-scheduler`, passed post-cleanup validation.
- [x] PR #977 removes the one-time push/Authenik cleanup path and retains manual fail-closed inventory/stopped-orphan cleanup only.
- [ ] PR #977 passes exact-head validation and merges.
- [ ] Task is archived after the closeout merge with its final merge SHA.

## Ownership

```yaml
owned_paths:
  - .github/workflows/synology-container-hygiene.yml
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
- No volume, network, image, runner, database-data, production-resource or unrelated-project cleanup is authorized.
- The retained workflow may mutate only verified stopped containers whose live Compose project is exactly `oteryn-staging`.
- Running/ambiguous portal resources cause cleanup refusal; canonical runtime is validated before and after any retained cleanup.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-10T23:48:00Z
head: d0d75441d8c22e63e5f4d4f1ae1e46a7b2b3fb21
branch: chore/synology-container-hygiene-closeout
pr: 977
status: validating
context_routes:
  - agent-governance
  - deployment
owned_paths:
  - .github/workflows/synology-container-hygiene.yml
  - docs/agents/tasks/active/OTERYN-20260811-container-resource-hygiene.md
  - docs/agents/tasks/archive/OTERYN-20260811-container-resource-hygiene.md
proven:
  - PR #973 merged as bef929a505580d778a93dccc304e3b822e735125 and installed mandatory execution-resource hygiene policy in AGENTS.md, DELIVERY_COMPLETENESS_AND_CLOSEOUT.md and EXECUTION_RESOURCE_HYGIENE.md.
  - First live run 31441629366 reached Synology Docker and failed closed before any removal because marketplace-scheduler was not included in the initial canonical source set.
  - Repository inspection proved marketplace-scheduler is canonical in deploy/synology/compose.marketplace.yml and marketplace-staging.sh.
  - PR #974 final head 54b57a4c10c2e5123782453f0bedace2219d6e81 passed all seven repository workflows and merged as b04ecd9fec838f8ee2c87b5be6c22b55ccb7abb7.
  - Live run 31443167942 job 93631882433 completed SUCCESS on the Synology runner.
  - That live job classified marketplace-scheduler as CANONICAL_PORTAL_SERVICE and verified it running before cleanup.
  - That live job revalidated exactly portal-authentik-local-test worker, server and postgresql containers, then stopped and removed exactly those three containers.
  - The successful job verified zero remaining containers with project portal-authentik-local-test and revalidated canonical Oteryn runtime after deletion.
  - No workflow command deleted volumes, networks or images; the successful job explicitly retained those resource classes.
  - Unrelated Synology workloads including Freqtrade, WickHunter, Home Assistant, UniFi, AdGuard and other projects were classified OUTSIDE_SCOPE and had no removal path.
  - PR #977 removes the one-time push-triggered/Authenik-specific destructive path and retains workflow_dispatch only.
derived:
  - The requested portal-container cleanup is complete; only repository terminal closeout remains.
unknown:
  - Original creator/task of the historical portal-authentik-local-test stack was not recoverable from current repository history/search.
conflicts: []
first_failure:
  marker: initial-live-source-of-truth-incomplete
  evidence: Run 31441629366 safely refused mutation on marketplace-scheduler; no resource was removed in that attempt.
rejected_hypotheses:
  - marketplace-scheduler was obsolete; current marketplace Compose/deployment tooling and successful live revalidation prove it is canonical.
  - The first cleanup attempt partially mutated Synology; its log contains zero removal events.
changed_paths:
  - .github/workflows/synology-container-hygiene.yml
  - docs/agents/tasks/active/OTERYN-20260811-container-resource-hygiene.md
validation:
  - command: PR #973 exact-head repository workflows on 20b8967c9973613fff11bd201275be8aaa4c0a0a
    result: PASS
    evidence: All seven required workflows passed before merge bef929a505580d778a93dccc304e3b822e735125.
  - command: Synology live hygiene run 31441629366
    result: FAIL
    evidence: Expected fail-closed outcome; Docker inventory succeeded, ambiguous canonical identity stopped the operation, and zero removals occurred.
  - command: PR #974 exact-head repository workflows on 54b57a4c10c2e5123782453f0bedace2219d6e81
    result: PASS
    evidence: All seven required workflows passed before merge b04ecd9fec838f8ee2c87b5be6c22b55ccb7abb7.
  - command: Synology live hygiene run 31443167942 job 93631882433
    result: PASS
    evidence: Exact three-container local-test cleanup completed; target absence and canonical runtime post-validation passed.
  - command: PR #977 exact-head checks
    result: NOT_RUN
    evidence: Closeout workflow/task update has just been pushed and awaits repository-selected checks.
blockers: []
next_action: Validate PR #977 exact head and merge it if the full diff, review state and required checks pass; then archive this task with the resulting closeout merge SHA.
policy_version: 2
phase: validate
session_id: agent-20260811-container-resource-hygiene-closeout
session_role: implementer
execution_mode: github
execution_reason: Retire one-time live cleanup authority after successful bounded execution and preserve only the manual fail-closed hygiene control.
lease_expires_at: 2026-08-11T00:18:00Z
context_pressure: medium
context_growth: stable
context_score: 6
estimate_confidence: high
decomposition_decision: single
decomposition_reason: Terminal cleanup of temporary workflow authority followed by task archival.
validation_level: focused
last_completed_step: Verified successful live cleanup evidence and removed the one-time push/Authenik-specific cleanup path in PR #977.
session_rotation_count: 0
heavy_validation_runs: 2
stale_takeover_count: 0
human_interruptions: 0
```

## Self-review

```yaml
result: PENDING
exact_head: d0d75441d8c22e63e5f4d4f1ae1e46a7b2b3fb21
acceptance_checked: true
full_diff_checked: false
negative_paths_checked: true
rollback_checked: true
compatibility_checked: NOT_APPLICABLE
related_prs_checked: true
findings: []
evidence:
  - Live cleanup outcome was verified from job 93631882433 rather than inferred from workflow success alone.
  - Retained workflow contains no push-triggered mutation, no running-container stop path and no Authentik-specific cleanup path.
```

## Notes

Synology live cleanup is complete. This active record now exists only until the retained workflow closeout merges; archival follows that merge so the archive can record its final SHA.
