---
task_id: OTERYN-20260818-repository-migration-org-access
project_lane: oteryn-platform-core
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/PROJECT_LANES.json
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_ULTRA.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
search_first:
  - current Oteryn organization visibility and GitHub App installation
  - open repository-migration PRs and active task ownership
optional_reads: []
---

# OTERYN-20260818-repository-migration-org-access

## Goal

Resume `OTERYN-REPO-MIGRATION-ULTRA` from the Wave-1 organization blocker after the owner reported creation of the `Oteryn` GitHub organization. Prove authenticated organization visibility/permissions before any META bootstrap or Tier-2 repository mutation, persist the exact blocker when unavailable, and keep physical migration fail-closed.

## Acceptance criteria

- [x] Trusted `main` and current migration governance are re-read.
- [x] Active task ownership and open repository-migration PR overlap are checked.
- [x] Owner report that `https://github.com/Oteryn/` was created is recorded without treating it as connector permission evidence.
- [x] Authenticated GitHub organization membership and GitHub App installation visibility are checked live.
- [x] No repository create/rename/transfer was attempted while authenticated organization visibility was absent.
- [x] A dedicated task branch and draft PR persist the recovery checkpoint.
- [x] `Oteryn` is visible to the authenticated GitHub integration through installation `154585379`.
- [x] Existing organization repository `Oteryn/Oteryn-Atlas` is visible with `admin/push/pull` permissions, proving write-capable organization-repository access for the installed integration.
- [x] Intended META coordinate `Oteryn/Oteryn` is currently absent (`404 Not Found`).
- [x] The next canonical migration phase is selected: prepare one META repository-creation/bootstrap transaction, without accessing server/game repositories.
- [ ] Exact final-head repository-selected checks and full-diff self-review pass.
- [ ] PR reaches intentional merge/closeout state and the task is archived.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260818-repository-migration-org-access.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
modules:
  - agent-governance
  - repository-migration-programme
dependencies:
  - OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION@1.1.0
  - OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_ULTRA@1.0.1
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-18T07:24:00Z
invocation_started_at: 2026-08-18T07:24:00Z
last_progress_at: 2026-08-18T07:24:00Z
head: PENDING_THIS_COMMIT
branch: docs/oteryn-20260818-repository-migration-org-access
pr: 1143
status: validating
phase: validate
session_id: chat-github-20260818-repo-migration-org-access-resume
session_role: coordinator
execution_mode: github
execution_reason: live organization/repository state inspection and narrow durable state reconciliation are supported by the GitHub connector
project_lane: oteryn-platform-core
task_kind: discovery
implementation_authorized: false
context_pressure: medium
context_growth: stable
context_score: 6
estimate_confidence: high
decomposition_decision: single
decomposition_reason: organization-access recovery is one bounded prerequisite before a separate META creation/bootstrap transaction
execution_budget: large
execution_budget_reason: canonical Ultra migration profile requires live authority, hidden-dependency and rollback evidence before physical repository mutations
ci_checks_for_current_head: 0
ci_check_generation: draft
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 1
stall_warnings: 0
context_routes:
  - agent-governance
  - ecosystem-repository-migration
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260818-repository-migration-org-access.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
proven:
  - Protected main remains da883a788ddb3d08ab853e4bffb5c99f3ca47d0a during this continuation.
  - PR 1138 merged the canonical programme hardening and PR 1142 merged its lifecycle closeout; the prior hardening task is archived.
  - PR 1143 remains the sole live repository-migration PR and owns exactly this task record plus programme state.
  - Previous organization-access blocker was real at the earlier checkpoint: installation list contained only account `blakinio`.
  - Current `list_installations` now exposes installation `154585379` for organization `Oteryn` in addition to installation `78758924` for `blakinio`.
  - `list_user_orgs` and `list_user_org_memberships` still return empty lists; these endpoints are therefore not used as the permission gate.
  - `Oteryn/Oteryn-Atlas` resolves live as a public organization repository with admin, maintain, push, pull and triage permission available through the connector.
  - Installation `154585379` currently enumerates `Oteryn/Oteryn-Atlas` as an accessible organization repository.
  - `Oteryn/Oteryn` returns `404 Not Found` and is therefore absent at the inspected coordinate.
  - PR 1143 prior exact head 42c3d51793eebb1609754b1ab1f88e6affd53bc0 passed Agent Governance run 32102212442 and CI run 32102212441.
derived:
  - The organization integration visibility blocker is resolved despite empty membership-list endpoints because the organization installation and write-capable organization repository are directly observable.
  - Existing `Oteryn/Oteryn-Atlas` is no longer treated as an expected-absent target; its live identity is proven and it is disjoint from the META-creation transaction.
  - The next safe programme frontier is a separate META repository-creation/bootstrap transaction for `Oteryn/Oteryn`.
  - No Game repository access is needed to complete this organization-access task or to prepare the independent META creation transaction.
unknown:
  - Whether the GitHub App installation is configured for `all repositories` or selected repositories; this must be verified by resulting-state access after a new repository exists.
  - Whether the current connector exposes a repository-creation operation; discovery shows no `create repository` action in the available GitHub tool set.
conflicts: []
first_failure:
  marker: oteryn-org-not-visible-to-github-integration
  evidence: resolved after owner authorization; installation 154585379 and Oteryn/Oteryn-Atlas access are now observable
rejected_hypotheses:
  - Empty membership-list endpoints mean the organization integration is still unavailable; direct installation and repository permission evidence disproves this.
  - Existing admin rights on blakinio/Oteryn-Platform alone prove organization access; current proof instead comes from Oteryn installation 154585379 and Oteryn/Oteryn-Atlas permissions.
  - META creation requires inspecting Oteryn-v2 first; the canonical programme explicitly allows disjoint META preparation while Game-specific blockers remain unresolved.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260818-repository-migration-org-access.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
validation:
  - command: GitHub live main/PR/ownership preflight
    result: PASS
    evidence: main remains da883a788ddb3d08ab853e4bffb5c99f3ca47d0a; PR 1143 is open Draft, mergeable, and owns exactly two paths
  - command: GitHub organization installation visibility
    result: PASS
    evidence: installation 154585379 targets organization Oteryn
  - command: GitHub organization repository permission probe
    result: PASS
    evidence: Oteryn/Oteryn-Atlas is visible with admin/maintain/push/pull/triage permission
  - command: intended META target identity probe
    result: PASS
    evidence: Oteryn/Oteryn returns 404 Not Found
  - command: prior exact-head repository-selected validation
    result: PASS
    evidence: head 42c3d51793eebb1609754b1ab1f88e6affd53bc0 passed Agent Governance 32102212442 and CI 32102212441; final head still requires its own checks
  - command: physical repository migration E2E
    result: NOT_APPLICABLE
    evidence: this task only proves organization access and selects the next transaction; no Tier-2 mutation is executed in this PR
blockers:
  - none
next_action: Validate this exact final PR head, mark PR 1143 Ready only if review/AI safety remains satisfied, squash-merge it, archive the task, then start one bounded META creation/bootstrap preparation task if execution budget permits.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: organization-access recovery task should be deleted after successful squash merge and lifecycle closeout
source_branch_evidence: PR #1143 / branch docs/oteryn-20260818-repository-migration-org-access
```

## Notes

The current Platform invocation does not separately authorize reading or operating on server/game repositories. No `Oteryn-v2`, Canary or otclient inspection was performed in this continuation. No repository coordinate, runtime, deployment, secret, production or live-game mutation was attempted.
