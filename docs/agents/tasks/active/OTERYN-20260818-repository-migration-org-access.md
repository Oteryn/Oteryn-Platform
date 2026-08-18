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

Resume `OTERYN-REPO-MIGRATION-ULTRA` from the Wave-1 blocker after the owner reported creation of the `Oteryn` GitHub organization. Prove authenticated organization visibility/permissions before any META bootstrap or Tier-2 repository mutation, persist the exact blocker when unavailable, and keep physical migration fail-closed.

## Acceptance criteria

- [x] Trusted `main` and current migration governance are re-read.
- [x] Active task ownership and open repository-migration PR overlap are checked.
- [x] Owner report that `https://github.com/Oteryn/` was created is recorded without treating it as connector permission evidence.
- [x] Authenticated GitHub organization membership and GitHub App installation visibility are checked live.
- [x] No repository create/rename/transfer is attempted while authenticated organization visibility is absent.
- [x] A dedicated task branch and draft PR persist the blocker and exact next action.
- [ ] `Oteryn` is visible to the authenticated GitHub integration with sufficient permission for the next bounded operation.
- [ ] The next canonical migration transaction or preparation phase is selected from fresh evidence.

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
  - authenticated GitHub integration does not currently expose the Oteryn organization or an Oteryn GitHub App installation
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-18T05:14:18Z
invocation_started_at: 2026-08-18T05:11:00Z
last_progress_at: 2026-08-18T05:14:18Z
head: 7282eff8c82e5f6582c7ae3e9114e06ef495a059
branch: docs/oteryn-20260818-repository-migration-org-access
pr: 1143
status: blocked
phase: investigate
session_id: chat-github-20260818-repo-migration-org-access
session_role: coordinator
execution_mode: github
execution_reason: live repository/organization state inspection and narrow durable documentation are supported by the GitHub connector
project_lane: oteryn-platform-core
task_kind: discovery
implementation_authorized: false
context_pressure: medium
context_growth: stable
context_score: 6
estimate_confidence: medium
decomposition_decision: single
decomposition_reason: one bounded organization-access gate precedes any physical migration transaction
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
  - Protected main is da883a788ddb3d08ab853e4bffb5c99f3ca47d0a at this task admission point.
  - PR 1138 merged the canonical programme hardening and PR 1142 merged its lifecycle closeout; the prior hardening task is archived.
  - Active task directory contains no repository-migration task before this task admission.
  - Repository-migration PR search returned no open matching PR.
  - Authenticated GitHub `list_user_orgs` returned an empty organization list.
  - Authenticated GitHub `list_user_org_memberships` returned an empty organization list.
  - Authenticated GitHub `list_installations` returned only installation 78758924 for account `blakinio`; no `Oteryn` installation was exposed.
  - The current owner message reports that the `Oteryn` organization has been created.
  - Draft PR 1143 owns exactly this task record and the migration programme state checkpoint.
derived:
  - Organization creation alone does not prove that the current GitHub integration can inspect or mutate organization-owned repositories.
  - META bootstrap and every Tier-2 create/rename/transfer under `Oteryn` remain NO_GO until authenticated organization visibility/permission is proven.
unknown:
  - Whether the current authenticated GitHub identity is a member/owner of `Oteryn` from the connector's perspective.
  - Whether the ChatGPT GitHub App has been installed/authorized for the `Oteryn` organization.
  - Exact organization-level repository creation/transfer permissions available to this integration after authorization.
conflicts: []
first_failure:
  marker: oteryn-org-not-visible-to-github-integration
  evidence: organization and membership lists are empty; installation list contains only account blakinio
rejected_hypotheses:
  - Owner-reported organization creation automatically makes the existing GitHub connector installation organization-visible.
  - Existing admin rights on blakinio/Oteryn-Platform authorize blind Tier-2 writes to a newly created organization.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260818-repository-migration-org-access.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
validation:
  - command: GitHub live main/active-task/open-PR/branch preflight
    result: PASS
    evidence: main da883a788ddb3d08ab853e4bffb5c99f3ca47d0a; no active migration task; no open matching migration PR; no matching migration branch before task creation
  - command: GitHub authenticated organization visibility checks
    result: BLOCKED
    evidence: list_user_orgs=[], list_user_org_memberships=[], list_installations contains only blakinio installation 78758924
  - command: PR 1143 changed-file scope
    result: PASS
    evidence: PR creation snapshot reports exactly 2 changed files, matching owned paths
  - command: physical repository migration E2E
    result: NOT_APPLICABLE
    evidence: Tier-2 mutation is intentionally not attempted while the organization-access gate is unsatisfied
blockers:
  - Oteryn organization is not visible/installed in the current GitHub integration, so organization-owned repository operations cannot be verified or executed safely.
next_action: Install or authorize the ChatGPT GitHub integration for the `Oteryn` organization, then rerun organization membership/installation visibility before any repository creation, transfer or rename.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: active blocked migration task must remain resumable until organization integration visibility is established
source_branch_evidence: PR #1143 / branch docs/oteryn-20260818-repository-migration-org-access
```

## Notes

The current Platform invocation does not separately authorize reading or operating on server/game repositories. No `Oteryn-v2`, Canary or otclient inspection was performed in this continuation. No repository coordinate, runtime, deployment, secret, production or live-game mutation was attempted.