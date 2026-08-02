---
task_id: OTERYN-20260802-github-only-execution-v1
status: validating
branch: docs/github-only-execution-v1-20260802
base_branch: main
created: 2026-08-02T11:43:00+02:00
updated: 2026-08-02T11:43:00+02:00
feature_pr: "PENDING"
owned_paths:
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/tasks/active/OTERYN-20260802-github-only-execution-v1.md
---

# GitHub-only execution v1

## Goal

Make the GitHub connection and GitHub Actions the mandatory fallback execution path when Codex or a local terminal is unavailable, without weakening platform, database, payment, authentication, production, authorization, or anti-stall rules.

## Acceptance

- [x] Add the normative GitHub-only execution contract.
- [x] Require it from the automatically loaded root bootstrap.
- [x] Route local agent execution through it.
- [x] Define bounded remote validation, temporary workflow, artifact, blocker, PR, merge, and production rules.
- [ ] Pass exact-head governance and required CI.
- [ ] Present a merge-ready PR without merging.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-02T11:43:00+02:00
head: 725930c7a262605ccc10ee48e6235a834c37bb18
branch: docs/github-only-execution-v1-20260802
pr: PENDING
status: validating
phase: validate
session_id: chat-20260802-github-only-execution-v1
session_role: coordinator
execution_mode: chat-github
run_scope: coordinated_governance_rollout
continuation_policy: continue_until_real_stop
task_completion_policy: prepare_validated_pr_without_merge
user_communication: low_noise
context_routes:
  - agent-governance
context_pressure: low
context_growth: stable
decomposition_decision: coordinated_repository_slice
validation_level: focused
last_completed_step: added mandatory GitHub-only execution routing
owned_paths:
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/tasks/active/OTERYN-20260802-github-only-execution-v1.md
proven:
  - Root and local routing require the GitHub-only contract when Codex or a local terminal is unavailable.
  - The contract preserves platform, database, payment, authentication, merge, production, secret, and anti-stall restrictions.
derived:
  - Missing Codex or a local terminal is no longer a generic technical blocker for repository work.
unknown:
  - Exact-head governance and platform workflow results after PR creation.
conflicts: []
first_failure:
  marker: none
  evidence: no validation failure observed
rejected_hypotheses:
  - GitHub-only execution authorizes production deployment
  - GitHub-only execution permits unbounded retries or unrelated PR cleanup
changed_paths:
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/tasks/active/OTERYN-20260802-github-only-execution-v1.md
validation: []
blockers: []
invocation_started_at: 2026-08-02T11:43:00+02:00
last_progress_at: 2026-08-02T11:43:00+02:00
runtime_limit_minutes: 60
no_progress_minutes: 15
ci_checks_for_current_head: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 0
stall_warnings: 0
next_action: open the draft PR, bind this task to its number, and verify exact-head required checks
```
