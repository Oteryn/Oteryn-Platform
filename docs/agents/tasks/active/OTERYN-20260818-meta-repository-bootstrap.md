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
- [x] `Oteryn/Oteryn` absence is treated as an evidence lease that must be refreshed immediately before creation.
- [x] ADR 0041 and Wave-1 readiness prove real META workload; the repository is not created merely to satisfy a diagram.
- [x] Minimal bootstrap scope, authority handover sequence, rollback and post-create verification are documented.
- [x] Current GitHub repository-creation behavior and GitHub App repository-access behavior are verified from official GitHub documentation.
- [ ] Target visibility is explicitly decided by the owner; no physical create occurs from an inferred visibility.
- [ ] Canonical `migration_transaction` is ready for exact owner-only creation only after all non-execution gates are proven.
- [ ] Exact-head validation/self-review and PR lifecycle are reconciled before physical cutover.

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
  - target repository visibility has not yet been explicitly selected by the owner
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-18T07:39:00Z
invocation_started_at: 2026-08-18T07:24:00Z
last_progress_at: 2026-08-18T07:39:00Z
head: PENDING_INITIAL_COMMIT
branch: docs/oteryn-20260818-meta-repository-bootstrap
pr: none
status: implementing
phase: design
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
decomposition_reason: one isolated META create transaction with one owner-only creation operation and one post-create bootstrap sequence
execution_budget: large
execution_budget_reason: canonical Ultra migration profile; physical repository creation requires exact authority, rollback and resulting-state evidence
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
  - architecture-migration
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260818-meta-repository-bootstrap.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/architecture/migration/OTERYN_META_REPOSITORY_BOOTSTRAP.md
proven:
  - Protected main at admission is fae1127f081a12ef6bc7c85951b819a3031138a6.
  - PR 1144 merged the required entry-task closeout and its source branch is absent.
  - Active task inventory has no repository-migration task at admission; open migration/META PR search returned none.
  - Oteryn organization installation 154585379 and write-capable access to Oteryn/Oteryn-Atlas were proven by the immediately preceding terminal task.
  - Oteryn/Oteryn was absent at the preceding organization-access observation; this must be refreshed before physical creation.
  - ADR 0041 states that META has real topology, manifest, compatibility, release and cross-repository coordination workload and is architecture-ready for demand-triggered creation.
  - Current GitHub connector actions expose no repository-create operation.
  - GitHub documentation allows creation in an organization with sufficient permission, requires a visibility choice, and supports optional README and GitHub App selection at creation.
  - GitHub App repository access can be configured as all repositories or selected repositories; resulting-state access must be verified after owner-created repository creation.
derived:
  - The physical create cannot be executed by the current connector and will ultimately require one precise owner UI operation if every non-execution gate is proven.
  - Initializing the new META with a README is appropriate because this is a new authority repository rather than an import and it creates an immediate default-branch anchor for governed bootstrap.
  - Public visibility is a plausible recommendation because current Platform and Atlas targets are public and META contains public architecture/compatibility material, but visibility is still an owner decision and is not evidence.
unknown:
  - Exact intended visibility of Oteryn/Oteryn.
  - Whether installation 154585379 uses all-repositories or selected-repositories mode for repositories created by the owner rather than by the app.
conflicts: []
first_failure:
  marker: repository-create-operation-unavailable-in-connector
  evidence: GitHub connector action discovery exposes file/branch/PR/issue/tree/commit creation but no repository-create action
rejected_hypotheses:
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
    evidence: main fae1127f081a12ef6bc7c85951b819a3031138a6; no overlapping active migration task; no open migration/META PR
  - command: canonical transaction/runbook structural review
    result: NOT_RUN
    evidence: initial preparation commit being constructed
  - command: physical repository creation E2E
    result: NOT_APPLICABLE
    evidence: physical creation is not yet authorized to proceed while target visibility remains UNKNOWN
blockers:
  - target visibility requires one explicit owner decision before the create transaction can become CUTOVER_READY
next_action: Complete the canonical transaction and bootstrap runbook, open the Draft PR, then persist the visibility decision as the only material non-execution gate if no other issue is found.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: META creation/bootstrap preparation task is active
source_branch_evidence: branch docs/oteryn-20260818-meta-repository-bootstrap
```

## Notes

The current Platform invocation does not grant server/game repository access. `blakinio/Oteryn-v2`, Canary and otclient are intentionally not inspected in this task. No physical repository create/rename/transfer, production/deployment, credential, secret, DNS, Synology or live-game mutation is performed by the preparation PR.
