---
task_id: OTERYN-20260811-container-resource-hygiene
mode: implementation
status: validating
project_lane: oteryn-platform-core
---

# OTERYN-20260811-container-resource-hygiene

## Goal

Close the remaining Synology container-hygiene safety and lifecycle work after merged PR #973 without weakening the no-broad-prune rule or touching unrelated Docker resources.

## Acceptance criteria

- [x] Broad Docker prune, forced removal, volumes, networks, images and unrelated projects remain outside cleanup authority.
- [x] Trusted-main Compose service identity remains parsed dynamically from `deploy/synology/compose.yml`.
- [x] Every trusted Compose service must be classified exactly once as runtime or one-shot; unclassified/unknown services fail closed.
- [x] Runtime canonical safety uses service-specific readiness rather than only `.State.Running`: Docker health where declared, Platform `/health`, Gateway `/health` + `/ready`, Canary published TCP sockets, and internal-proxy nginx/TLS listener probes.
- [x] Canonical `tls-init` is valid only as a successful one-shot (`exited`, exit code 0).
- [x] A stopped duplicate can be removed only when its canonical service passes the same runtime/one-shot validity contract.
- [x] Canonical runtime readiness is revalidated immediately before and after cleanup.
- [ ] Exact-head pull-request static/governance checks pass.
- [ ] Follow-up PR merges with title prefix `fix(ops): harden Synology container hygiene` so trusted-main push executes the bounded cleanup once.
- [ ] Trusted-main live cleanup completes successfully and preserves canonical runtime readiness.
- [ ] This task is archived only after live evidence is proven.

## Ownership

```yaml
owned_paths:
  - .github/workflows/synology-container-hygiene.yml
  - docs/agents/tasks/active/OTERYN-20260811-container-resource-hygiene.md
modules:
  - agent-governance
  - synology-operations
dependencies:
  - trusted-main self-hosted `oteryn-staging` runner after merge
blockers: []
cross_repository_tasks: []
```

## Safety boundary

- Cleanup authority is limited to stopped containers labelled for Compose project `oteryn-staging`.
- Runtime services are `mariadb redis platform canary internal-proxy gateway`; one-shot service is `tls-init`; static validation requires their union to equal the exact trusted Compose service set with no overlap.
- Persistent/named volumes, networks, images, unrelated Compose projects, runner containers, production systems and external repositories remain outside scope.
- Any missing, unclassified, unhealthy, running-noncanonical or otherwise ambiguous portal-owned container causes fail-closed refusal.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-11T01:48:00+02:00
head: ea351554b90f9db66be300a98d9e16a0303148c4
branch: ops/synology-container-resource-hygiene-followup
pr: none
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
  - PR #973 merged as `bef929a505580d778a93dccc304e3b822e735125` but left this active task pointing at the already-completed merge action, which blocks unrelated live-task governance.
  - Codex review findings `3754146585` and `3754146588` remain material on merged main: exhaustive service classification was not enforced and canonical readiness used `.State.Running` alone.
  - Current trusted Compose has six runtime services plus one one-shot `tls-init`.
  - Follow-up workflow requires exact service-classification set equality and no runtime/one-shot overlap.
  - Follow-up readiness probes Docker health where present, Platform/Gateway HTTP readiness, Canary TCP sockets and internal-proxy nginx/TLS listeners; validation runs before and after deletion.
  - No blanket prune or volume/network/image deletion primitive was introduced.
derived:
  - Merging the focused workflow repair and then observing its automatically triggered trusted-main cleanup can terminally close the two P2 findings and this stale task.
unknown:
  - trusted-main live cleanup result after follow-up merge
conflicts: []
first_failure:
  marker: stale-post-merge-container-hygiene-task
  evidence: PR #972 Agent Governance rejected this main task because `next_action` still requested the already-merged PR #973; source review then confirmed the two P2 safety gaps.
rejected_hypotheses:
  - PR #973 merge alone makes this task terminal; post-merge cleanup and review findings remain open.
  - `.State.Running=true` proves a safe canonical replacement; service-specific readiness is now required.
  - Trusted Compose parsing is fail-closed while new services may remain unclassified; exact set equality is now required.
changed_paths:
  - .github/workflows/synology-container-hygiene.yml
  - docs/agents/tasks/active/OTERYN-20260811-container-resource-hygiene.md
validation: []
blockers: []
next_action: Open the focused follow-up PR, pass exact-head checks, merge it with the required title prefix, verify the resulting trusted-main live cleanup, then archive this task.
policy_version: 2
phase: validating
session_id: agent-20260811-container-resource-hygiene-followup
session_role: implementer
execution_mode: github
execution_reason: Reconcile a stale merged-task blocker and close two material workflow-safety findings using only GitHub-governed repository changes and the existing staging runner.
lease_expires_at: 2026-08-11T02:30:00+02:00
context_pressure: low
context_growth: stable
context_score: 6
estimate_confidence: high
decomposition_decision: single
decomposition_reason: One focused operational workflow repair plus its terminal live verification.
validation_level: focused
last_completed_step: Implemented exhaustive service classification and service-specific canonical readiness on the follow-up branch.
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

## Notes

This follow-up does not broaden cleanup authority. The live cleanup remains limited to the explicit stopped-container safety boundary above and must preserve all unrelated Synology workloads.
