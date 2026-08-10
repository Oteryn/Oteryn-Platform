---
task_id: OTERYN-20260811-container-resource-hygiene
mode: implementation
status: validating
project_lane: oteryn-platform-core
---

# OTERYN-20260811-container-resource-hygiene

## Goal

Finish the post-#973 Synology container-hygiene safety repair and lifecycle closeout without broad pruning or unrelated Docker mutation.

## Acceptance criteria

- [x] Cleanup remains limited to verified stopped `oteryn-staging` containers; volumes, networks, images, forced removal, broad prune and unrelated projects are forbidden.
- [x] The exact trusted-main Compose service set is parsed dynamically.
- [x] Every trusted service is classified exactly once as runtime or one-shot; unknown/unclassified services fail closed.
- [x] Canonical readiness is service-specific: Docker health where declared, Platform `/health`, Gateway `/health` + `/ready`, Canary published TCP sockets, internal-proxy nginx/TLS probes.
- [x] `tls-init` is valid only as successful one-shot exit 0.
- [x] Canonical validity is required before a duplicate is a deletion candidate and is revalidated before/after cleanup.
- [ ] PR #975 exact-head checks pass.
- [ ] PR #975 merges with title `fix(ops): harden Synology container hygiene`, triggering the bounded trusted-main cleanup.
- [ ] Trusted-main cleanup succeeds and canonical services remain ready.
- [ ] Task is archived after live proof.

## Ownership

```yaml
owned_paths:
  - .github/workflows/synology-container-hygiene.yml
  - docs/agents/tasks/active/OTERYN-20260811-container-resource-hygiene.md
modules:
  - agent-governance
  - synology-operations
dependencies:
  - trusted-main self-hosted oteryn-staging runner after merge
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-11T01:50:00+02:00
head: b7ff423d01f2dc96b177f18089bc0be596d71007
branch: ops/synology-container-resource-hygiene-followup
pr: 975
status: validating
phase: followup_safety_repair
context_routes:
  - agent-governance
  - deployment
  - devops-sre
  - security
owned_paths:
  - .github/workflows/synology-container-hygiene.yml
  - docs/agents/tasks/active/OTERYN-20260811-container-resource-hygiene.md
proven:
  - PR #973 merged as bef929a505580d778a93dccc304e3b822e735125 but left this active task with a stale merge next_action.
  - Merged main retained Codex findings 3754146585 and 3754146588: exhaustive service classification was not enforced and readiness used State.Running alone.
  - Follow-up workflow requires exact runtime/one-shot classification equality with trusted Compose.
  - Follow-up workflow uses service-specific readiness and repeats canonical validation before and after removal.
  - PR #975 is the single follow-up PR and changes only this workflow plus this task record.
derived:
  - Exact-head static/governance validation plus the post-merge trusted-main live cleanup can terminally close the stale task and both remaining safety findings.
unknown:
  - trusted-main cleanup result after PR #975 merge
conflicts: []
first_failure:
  marker: stale-post-merge-container-hygiene-task
  evidence: PR #972 Agent Governance rejected the stale #973 next_action; source inspection confirmed both review gaps remained on main.
rejected_hypotheses:
  - PR #973 merge alone made the task terminal.
  - State.Running alone proves a safe canonical replacement.
  - Dynamic Compose parsing is fail-closed if a newly added service can remain unclassified.
changed_paths:
  - .github/workflows/synology-container-hygiene.yml
  - docs/agents/tasks/active/OTERYN-20260811-container-resource-hygiene.md
validation: []
blockers: []
next_action: Validate PR #975 exact head, merge it, verify the automatically triggered trusted-main cleanup, then archive this task.
policy_version: 2
phase: validating
session_id: agent-20260811-container-resource-hygiene-followup
session_role: implementer
execution_mode: github
execution_reason: Reconcile stale post-merge task state and close two material workflow-safety findings.
lease_expires_at: 2026-08-11T02:30:00+02:00
context_pressure: low
context_growth: stable
context_score: 6
estimate_confidence: high
decomposition_decision: single
decomposition_reason: One focused workflow repair followed by one live cleanup verification.
validation_level: focused
last_completed_step: Opened PR #975 after implementing exhaustive service classification and service-specific readiness.
session_rotation_count: 0
heavy_validation_runs: 0
stale_takeover_count: 1
human_interruptions: 0
```

## Self-review

```yaml
result: PENDING
exact_head: none
acceptance_checked: false
full_diff_checked: false
negative_paths_checked: false
rollback_checked: true
compatibility_checked: NOT_APPLICABLE
related_prs_checked: true
findings: []
evidence: []
```
