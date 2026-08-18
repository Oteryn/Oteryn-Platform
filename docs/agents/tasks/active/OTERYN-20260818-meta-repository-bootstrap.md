---
task_id: OTERYN-20260818-meta-repository-bootstrap
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
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_ULTRA.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/architecture/adr/0041-ecosystem-repository-authority-contracts-and-atlas-integration.md
  - docs/architecture/migration/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_READINESS.md
search_first:
  - Oteryn/Oteryn target existence and organization integration access
  - open migration/META PRs and active task ownership
  - canonical migration_transaction schema
optional_reads: []
---

# OTERYN-20260818-meta-repository-bootstrap

## Goal

Prepare one canonical, fail-closed Tier-2 transaction for creating the real `Oteryn/Oteryn` META repository and define the smallest non-ceremonial bootstrap that can become canonical immediately after creation. Do not inspect or mutate server/game repositories and do not perform the physical repository creation until every canonical gate is satisfied.

## Acceptance criteria

- [x] Entry-task closeout is terminal and its source branch is absent.
- [x] Protected `main` is re-read after closeout.
- [x] No overlapping open migration/META PR or active migration task exists at admission.
- [x] Current `Oteryn` organization integration access remains proven by installation/repository evidence.
- [x] `Oteryn/Oteryn` absence was refreshed for this transaction and remains an expiring evidence lease.
- [x] ADR 0041 and Wave-1 readiness prove real META workload; the repository is not created merely to satisfy a diagram.
- [x] Minimal bootstrap scope, authority handover sequence, replay guard, rollback candidate and post-create verification are documented.
- [x] Current GitHub repository-creation, GitHub App repository-access and repository-deletion behavior are verified from official GitHub documentation.
- [x] Draft PR #1145 owns exactly the three declared Platform preparation paths.
- [x] Full three-file semantic-content diff self-review on `f20eb8abbf0c12cbc6497a48ef2f427397ca79cf` found zero material content findings after rollback was corrected from assumed `PROVEN` to `NOT_PROVEN`.
- [ ] Target visibility is explicitly decided by the owner; no physical create occurs from an inferred visibility.
- [ ] Current Oteryn rollback capability is proven; generic GitHub deletion support does not satisfy the gate when organization/enterprise policy can restrict deletion.
- [ ] Canonical `migration_transaction` advances from `PREPARED/NO_GO` only after the two owner-specific gates above are resolved and exact leases are refreshed.
- [ ] Exact-head repository-selected checks pass after checkpoint repair.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260818-meta-repository-bootstrap.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/architecture/migration/OTERYN_META_REPOSITORY_BOOTSTRAP.md
modules:
  - agent-governance
  - repository-migration-programme
  - ecosystem-architecture
  - migration-runbook
dependencies:
  - OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION@1.1.0
  - OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_ULTRA@1.0.1
  - ADR 0041
  - organization-access recovery PR 1143 / closeout PR 1144
blockers:
  - exact target visibility is not explicitly selected by the owner
  - rollback feasibility is NOT_PROVEN until current Oteryn organization/enterprise policy or equivalent owner deletion capability is confirmed
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-18T07:49:00Z
invocation_started_at: 2026-08-18T07:24:00Z
last_progress_at: 2026-08-18T07:49:00Z
head: f20eb8abbf0c12cbc6497a48ef2f427397ca79cf
branch: docs/oteryn-20260818-meta-repository-bootstrap
pr: 1145
status: blocked
phase: validate
session_id: chat-github-20260818-meta-repository-bootstrap
session_role: coordinator
execution_mode: github
execution_reason: the GitHub connector supports exact repository state inspection and Platform-side durable transaction/runbook preparation; it does not expose repository creation
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: preparation_only
context_pressure: medium
context_growth: stable
context_score: 8
estimate_confidence: high
decomposition_decision: single
decomposition_reason: one isolated META create transaction with owner-specific creation inputs and one post-create bootstrap sequence
execution_budget: large
execution_budget_reason: canonical Ultra migration profile; physical repository creation requires exact authority, rollback and resulting-state evidence
ci_checks_for_current_head: 1
ci_check_generation: draft
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
context_reconstruction_attempts: 1
stall_warnings: 0
context_routes:
  - agent-governance
  - ecosystem-repository-migration
  - architecture-migration
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260818-meta-repository-bootstrap.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/architecture/migration/OTERYN_META_REPOSITORY_BOOTSTRAP.md
proven:
  - Protected main at admission is fae1127f081a12ef6bc7c85951b819a3031138a6.
  - PR 1144 merged the required entry-task closeout and its source branch is absent.
  - Active task inventory had no repository-migration task at admission; open migration/META PR search returned none.
  - Oteryn organization installation 154585379 currently enumerates Oteryn/Oteryn-Atlas with write/admin-capable access.
  - Oteryn/Oteryn was refreshed as 404/absent during this transaction preparation.
  - ADR 0041 states that META has real topology, manifest, compatibility, release and cross-repository coordination workload and is architecture-ready for demand-triggered creation.
  - Current GitHub connector actions expose no repository-create operation.
  - Official GitHub documentation requires an explicit visibility choice for repository creation and permits README/GitHub App selection for a new non-import repository.
  - Official GitHub documentation permits organization-repository deletion for owners/admins but states organization/enterprise policy can restrict it; current Oteryn rollback capability is therefore not proven by generic documentation.
  - PR 1145 is the sole migration/META PR created for this additional task and owns exactly the declared three paths.
  - Agent Governance run 32112973133 on semantic head f20eb8abbf0c12cbc6497a48ef2f427397ca79cf failed only because the active task still recorded `pr: none`; the liveness log explicitly reported `branch_pr_identity_omitted` for Draft PR 1145.
derived:
  - The physical create cannot be executed by the current connector and will require one precise owner GitHub web flow only after visibility and rollback feasibility are proven and the preparation state is canonical/current.
  - Initializing the new META with a README is appropriate because this is a new authority repository rather than an import and creates an immediate default-branch anchor for governed bootstrap.
  - Public visibility is the current recommendation because Platform and Atlas targets are public and META contains public architecture/compatibility metadata, but this is not authority to choose visibility.
unknown:
  - Exact intended visibility of Oteryn/Oteryn.
  - Current Oteryn organization/enterprise repository-deletion policy or equivalent owner rollback capability for a fresh repository.
  - Whether installation 154585379 uses all-repositories or selected-repositories mode for an owner-created repository; post-create access must be verified.
conflicts: []
first_failure:
  marker: branch_pr_identity_omitted
  evidence: Agent Governance 32112973133 / job 95636247858 reported active task branch has Draft PR 1145 but checkpoint recorded pr:none
rejected_hypotheses:
  - General GitHub deletion documentation proves current Oteryn rollback capability; organization/enterprise policy may restrict deletion.
  - The existing Oteryn-Atlas repository means the META repository already exists.
  - Empty membership-list endpoints invalidate direct installation/repository access evidence.
  - An inferred public/private choice is safe enough for Tier-2 creation.
  - Game repository evidence is required to create the independent META coordination plane.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260818-meta-repository-bootstrap.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/architecture/migration/OTERYN_META_REPOSITORY_BOOTSTRAP.md
validation:
  - command: live main/active-task/open-migration-PR admission preflight
    result: PASS
    evidence: main fae1127f081a12ef6bc7c85951b819a3031138a6; no overlapping active migration task; no open migration/META PR at admission
  - command: full three-file semantic-content exact diff review on f20eb8abbf0c12cbc6497a48ef2f427397ca79cf
    result: PASS
    evidence: transaction remains PREPARED/NO_GO, rollback is NOT_PROVEN, authority handover remains ordered, and no server/game or physical mutation is included
  - command: Agent Governance 32112973133 on f20eb8abbf0c12cbc6497a48ef2f427397ca79cf
    result: FAIL
    evidence: branch_pr_identity_omitted only; checkpoint repair in this commit records pr:1145
  - command: physical repository creation E2E
    result: NOT_APPLICABLE
    evidence: physical creation remains NO_GO while owner visibility and rollback gates are unresolved
blockers:
  - owner must select PUBLIC or PRIVATE visibility for Oteryn/Oteryn
  - owner must confirm rollback permission/capability for deleting a fresh Oteryn/Oteryn repository if post-create verification fails before authority handover
next_action: Owner provides the exact visibility and confirms fresh-repository deletion capability; then refresh transaction leases, set rollback PROVEN only with that evidence, run exact-head checks and advance to CUTOVER_READY only if one owner create flow remains.
```

## Self-review

```yaml
self_review:
  result: PASS
  exact_semantic_content_head: f20eb8abbf0c12cbc6497a48ef2f427397ca79cf
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings_repaired:
    - initial generic-deletion evidence was insufficient to prove current Oteryn rollback capability; transaction corrected to rollback.feasibility=NOT_PROVEN
  open_material_findings: []
```

This checkpoint commit repairs the live PR identity required by Agent Governance and records the two real owner-specific blockers. It changes no physical migration state.

## Source branch closeout

```yaml
source_branch_disposition: retain
source_branch_reason: active transaction branch must remain resumable while owner-specific visibility and rollback gates are unresolved
source_branch_evidence: Draft PR #1145 / branch docs/oteryn-20260818-meta-repository-bootstrap
```

## Notes

The current Platform invocation does not grant server/game repository access. `blakinio/Oteryn-v2`, Canary and otclient were intentionally not inspected. No physical repository create/rename/transfer/delete, production/deployment, credential, secret, DNS, Synology or live-game mutation is performed by this preparation PR.
