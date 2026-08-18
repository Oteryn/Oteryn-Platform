---
programme_id: OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION
programme_version: 1
canonical_prompt: docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md
required_reads:
  - AGENTS.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
repository: blakinio/Oteryn-Platform
owner_alias: OTERYN-REPO-MIGRATION
---

# Oteryn Ecosystem Repository Migration — Programme State

## Mission

Move the accepted Oteryn ecosystem topology from logical ownership to verified, bounded physical repository migration without sacrificing history, provenance, CI/release integrity, rollback, security or product delivery.

## Durable state

```yaml
programme_state_version: 1
updated_at: 2026-08-18T08:02:30Z
status: waiting
active_task: null
issue: null
branch: null
pull_request: null
temporary_topology_authority:
  repository: blakinio/Oteryn-Platform
  path: docs/architecture/adr/0041-ecosystem-repository-authority-contracts-and-atlas-integration.md
  status: VERIFIED_LIVE_PENDING_META_AUTHORITY

target_topology:
  - Oteryn
  - Oteryn-Game
  - Oteryn-Platform
  - Oteryn-Atlas

known_migration_sources:
  - blakinio/Oteryn-v2
  - blakinio/Oteryn-Platform
  - blakinio/Otheryn

legacy_read_only_sources:
  - blakinio/canary
  - blakinio/otclient

wave_1_evidence:
  implementation_pr: 1131
  implementation_merge: 43ceb7d17054787698c879a0797718e4a1cb1c28
  readiness_report: docs/architecture/migration/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_READINESS.md
  coordinate_inventory: docs/architecture/migration/oteryn-repository-coordinate-inventory.json
  atlas_extraction_manifest: docs/architecture/migration/oteryn-atlas-extraction-manifest.json
  archived_task: docs/agents/tasks/archive/OTERYN-20260817-ecosystem-repository-migration-wave1.md

organization_access_evidence:
  implementation_pr: 1143
  implementation_merge: 36774bbf2c820572b1f4272dd373c24491d71d96
  archived_task: docs/agents/tasks/archive/OTERYN-20260818-repository-migration-org-access.md
  connector_installation:
    installation_id: 154585379
    account_login: Oteryn
    account_type: Organization
  connector_oteryn_visible: true
  organization_repository_probe:
    repository: Oteryn/Oteryn-Atlas
    visibility: public
    permissions:
      admin: true
      maintain: true
      push: true
      pull: true
      triage: true
  classification: PROVEN_ORGANIZATION_INTEGRATION_ACCESS

meta_preparation_evidence:
  implementation_pr: 1145
  implementation_final_head: 20b8a73487e74a0b66924662a1d7e2b9f8b1e3e0
  implementation_merge: 860273ba7eb56fd4f6f3b1e1f8cbb765b2c094fe
  final_agent_governance_run: 32114183914
  final_ci_run: 32114183887
  archived_task: docs/agents/tasks/archive/OTERYN-20260818-meta-repository-bootstrap.md
  source_branch_deleted: true

observed_target_coordinates:
  Oteryn:
    repository: Oteryn/Oteryn
    state: ABSENT_404
    refreshed_after_preparation_merge: 860273ba7eb56fd4f6f3b1e1f8cbb765b2c094fe
  Oteryn-Atlas:
    repository: Oteryn/Oteryn-Atlas
    state: EXISTS
    visibility: public
    size: 0

migration_transaction:
  transaction_id: OTERYN-META-CREATE-20260818
  mutation: create
  state: PREPARED
  public_status: CUTOVER_READY
  source_coordinate: none
  target_coordinate: Oteryn/Oteryn
  source_head: none
  canonical_preparation_merge: 860273ba7eb56fd4f6f3b1e1f8cbb765b2c094fe
  pre_state_snapshot:
    target_repository: ABSENT_404
    organization: Oteryn
    organization_installation_id: 154585379
    accessible_organization_repositories:
      - Oteryn/Oteryn-Atlas
    platform_authority_head: 860273ba7eb56fd4f6f3b1e1f8cbb765b2c094fe
  expected_post_state:
    repository: Oteryn/Oteryn
    owner: Oteryn
    archived: false
    initial_content: README_ONLY
    visibility: PUBLIC
    connector_access: MUST_BE_PROVEN_AFTER_CREATE
    bootstrap_authority: NOT_CANONICAL_UNTIL_BOOTSTRAP_PR_MERGES
  authority_verified: true
  target_identity_or_absence_verified: true
  target_governance_verified: NOT_APPLICABLE_FOR_ABSENT_TARGET
  source_state_verified: true
  evidence_lease_current: true
  active_pr_task_impact_verified: true
  coordinate_inventory_complete_for_cutover: true
  executable_callers_resolved: true
  ci_impact_resolved: true
  package_impact_resolved_or_owner_risk_acceptance_proven: true
  provenance_strategy_verified: true
  target_collision: false
  ownership_conflict: false
  material_unknowns: []
  preparation_state: CANONICAL
  cutover_lock:
    owner: OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION
    transferred_after_preparation_merge: 860273ba7eb56fd4f6f3b1e1f8cbb765b2c094fe
    invalidated_by:
      - Oteryn/Oteryn starts existing before the intended owner create action
      - Oteryn organization installation/access changes
      - Platform topology authority changes materially
      - overlapping META migration task or PR appears
      - owner changes target visibility or target coordinate
  replay_guard:
    mutation_fingerprint: create_repository:Oteryn/Oteryn
    reissue_forbidden_until_state_proven_not_applied: true
    resume_detection: exact live GET of Oteryn/Oteryn before any create retry
  point_of_no_return:
    reached_when: GitHub exposes a repository object at Oteryn/Oteryn
    consequences: target namespace is occupied and rollback requires a separate destructive repository-deletion operation
  residual_risk_acceptance:
    status: none
    accepted_by: none
    accepted_at: none
    exact_scope: none
    expiry_or_recheck: none
    evidence: none
  rollback:
    feasibility: PROVEN
    operation: owner deletes exact fresh-bootstrap Oteryn/Oteryn repository through GitHub settings
    trigger: wrong create result or inability to bring the fresh repository under required integration/governance before any authority handover
    decision_owner: Oteryn organization owner
    execution_window: before META ADR becomes canonical, before external dependents/releases, and before unique history exists
    verification: Oteryn/Oteryn returns 404 and no canonical manifest/ADR points to the deleted repository
    evidence: Oteryn organization owner explicitly confirmed fresh-repository deletion capability before authority handover
  post_mutation_validation:
    - exact repository owner/name/id and created state
    - PUBLIC visibility and archived=false
    - default branch/README initialization
    - connector pull/push/admin capability required for bootstrap
    - no unexpected template/import/history or product runtime content
    - installation repository access after owner-created repository creation
    - dedicated bootstrap branch and Draft PR before non-bootstrap content
    - META ADR not canonical until bootstrap PR merge
    - Platform ADR 0041 not superseded before canonical META authority exists

meta_bootstrap_plan:
  document: docs/architecture/migration/OTERYN_META_REPOSITORY_BOOTSTRAP.md
  target_visibility: PUBLIC
  target_visibility_decision_source: owner current invocation 2026-08-18
  initial_files:
    - README.md
    - AGENTS.md
    - docs/architecture/adr/0001-ecosystem-topology-authority.md
    - ecosystem/repositories.json
  authority_handover_rule: Platform ADR 0041 remains canonical until the META topology ADR is accepted and merged in Oteryn/Oteryn
  physical_creation_tool: OWNER_GITHUB_WEB_UI_REQUIRED
  connector_repository_create_operation: unavailable

proven:
  - The programme alias is OTERYN-REPO-MIGRATION and resolves to the canonical prompt in this repository.
  - Wave 1 implementation PR 1131 squash-merged through protected main as 43ceb7d17054787698c879a0797718e4a1cb1c28.
  - Organization-access PR 1143 squash-merged as 36774bbf2c820572b1f4272dd373c24491d71d96 and closeout PR 1144 merged as fae1127f081a12ef6bc7c85951b819a3031138a6.
  - GitHub App installation 154585379 targets organization Oteryn and continues to expose Oteryn/Oteryn-Atlas with admin/push/pull capability.
  - The owner explicitly selected PUBLIC visibility for Oteryn/Oteryn.
  - The owner explicitly confirmed fresh Oteryn/Oteryn deletion capability for rollback before authority handover.
  - PR 1145 exact final head 20b8a73487e74a0b66924662a1d7e2b9f8b1e3e0 passed Agent Governance 32114183914 and CI 32114183887 with zero reviews, threads or comments.
  - PR 1145 squash-merged as 860273ba7eb56fd4f6f3b1e1f8cbb765b2c094fe and its source branch is absent.
  - Oteryn/Oteryn remained 404/absent in the final post-merge lease refresh.
  - The current GitHub connector exposes no repository-create operation.
  - Oteryn-v2 package/caller evidence was not refreshed because the current trusted Platform invocation does not authorize server/game repository inspection and those blockers do not block the independent META create transaction.

derived:
  - The accepted four-repository architecture remains valid.
  - All material non-execution gates for `OTERYN-META-CREATE-20260818` are proven and canonical.
  - Exactly one precise unsupported owner-only operation remains: create public `Oteryn/Oteryn` in the GitHub web UI with README initialization.
  - Game cutover and Atlas extraction remain independently fail-closed and are not bundled with META creation.

unknown:
  - Whether installation 154585379 automatically includes an owner-created new repository; this is a required immediate post-create verification item, not a pre-create blocker.
  - Exhaustive external Actions/reusable-workflow callers of Oteryn-v2.
  - Exact Oteryn-v2 GHCR/package names, links, permissions and consumers.
  - Complete path-level Atlas ownership split needed for selective extraction.

conflicts: []

blockers:
  - Game-specific package/caller evidence remains unresolved for any future Oteryn-v2/Oteryn-Game physical cutover.
  - Atlas extraction remains separately coupled to source ownership/deployment evidence and must not be inferred ready from the existence of Oteryn/Oteryn-Atlas.

next_action: Owner performs the exact public Oteryn/Oteryn GitHub web creation flow from the canonical bootstrap plan; if the create result is ambiguous, do not retry and first inspect the exact target coordinate.
```

## Programme rules

- Live repository state outranks this file when newer.
- Do not cache transient main SHAs here as future authority.
- Do not let the Game->Atlas spatial-profile evidence gap block unrelated repository migration work without a proven dependency.
- Do not create empty target repositories merely to satisfy the target diagram.
- META authority supersedes Platform ADR 0041 only after the META authority is actually canonical.
- Every physical migration step requires exact preflight, rollback and post-cutover verification.
- Keep exactly one `next_action` while the programme is not terminal.
