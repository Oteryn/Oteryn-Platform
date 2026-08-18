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
updated_at: 2026-08-18T07:49:30Z
status: blocked
active_task: OTERYN-20260818-meta-repository-bootstrap
issue: null
branch: docs/oteryn-20260818-meta-repository-bootstrap
pull_request: 1145
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

observed_target_coordinates:
  Oteryn:
    repository: Oteryn/Oteryn
    state: ABSENT_404
    refreshed_at: 2026-08-18T07:48:00Z
  Oteryn-Atlas:
    repository: Oteryn/Oteryn-Atlas
    state: EXISTS
    visibility: public
    size: 0

migration_transaction:
  transaction_id: OTERYN-META-CREATE-20260818
  mutation: create
  state: PREPARED
  public_status: NO_GO
  source_coordinate: none
  target_coordinate: Oteryn/Oteryn
  source_head: none
  pre_state_snapshot:
    target_repository: ABSENT_404
    organization: Oteryn
    organization_installation_id: 154585379
    accessible_organization_repositories:
      - Oteryn/Oteryn-Atlas
    platform_authority_head: fae1127f081a12ef6bc7c85951b819a3031138a6
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
  material_unknowns:
    - current_oteryn_repository_deletion_policy_or_owner_rollback_capability
  cutover_lock:
    owner: OTERYN-20260818-meta-repository-bootstrap
    acquired_at: 2026-08-18T07:41:00Z
    invalidated_by:
      - Oteryn/Oteryn starts existing before the intended create action
      - Oteryn organization installation/access changes
      - Platform main/topology authority changes materially
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
    feasibility: NOT_PROVEN
    operation: candidate owner deletion of exact fresh-bootstrap Oteryn/Oteryn repository through GitHub settings
    trigger: wrong create result or inability to bring the fresh repository under required integration/governance before any authority handover
    decision_owner: Oteryn organization owner
    execution_window: before META ADR becomes canonical, before external dependents/releases, and before unique history exists
    verification: Oteryn/Oteryn returns 404 and no canonical manifest/ADR points to the deleted repository
    missing_proof: current Oteryn organization or enterprise policy permits the owner to delete the fresh repository
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
  - Organization-access PR 1143 exact final head passed required checks and squash-merged as 36774bbf2c820572b1f4272dd373c24491d71d96; lifecycle closeout PR 1144 merged as fae1127f081a12ef6bc7c85951b819a3031138a6.
  - GitHub App installation 154585379 targets organization Oteryn and currently enumerates Oteryn/Oteryn-Atlas with write/admin-capable access.
  - Oteryn/Oteryn was refreshed as 404/absent for this transaction.
  - ADR 0041 proves that the thin META has real topology, compatibility, release-manifest and cross-repository coordination workload.
  - Official GitHub documentation confirms organization repository creation requires sufficient permission and an explicit visibility choice, and can optionally initialize a README/select GitHub Apps.
  - Official GitHub documentation confirms installed GitHub App repository access may be all repositories or selected repositories; post-create app access must be verified rather than assumed.
  - Official GitHub documentation confirms repository deletion can be restricted by organization or enterprise policy; generic deletion support therefore does not prove this transaction's rollback capability.
  - The current GitHub connector exposes no repository-create operation.
  - PR 1145 repaired its live task identity and exact checkpoint head d7fb9dda72e2a0ebe83d0f2bec4c43c72d50c817 passed Agent Governance 32113083310 and CI 32113083262.
  - The owner explicitly selected PUBLIC visibility for Oteryn/Oteryn in the current invocation on 2026-08-18.
  - Oteryn-v2 package/caller evidence was not refreshed because the current trusted Platform invocation does not authorize server/game repository inspection and those blockers do not block independent META creation preparation.

derived:
  - The accepted four-repository architecture remains valid.
  - META visibility is frozen to PUBLIC for this transaction unless the owner explicitly changes it before creation.
  - META creation is architecture-ready and non-ceremonial, but physical creation is `NO_GO` until rollback capability is proven.
  - If rollback capability is proven and the preparation PR becomes canonical with all other leases current, the transaction may become public `CUTOVER_READY` because the current connector cannot execute repository creation and one precise owner web creation flow will remain.
  - Game cutover and Atlas extraction remain independently fail-closed and are not bundled with META creation.

unknown:
  - Current Oteryn organization/enterprise repository-deletion policy or equivalent owner rollback capability for a freshly created repository.
  - Whether installation 154585379 uses all-repositories or selected-repositories mode for an owner-created new repository; resulting-state access must be verified immediately after creation.
  - Exhaustive external Actions/reusable-workflow callers of Oteryn-v2.
  - Exact Oteryn-v2 GHCR/package names, links, permissions and consumers.
  - Complete path-level Atlas ownership split needed for selective extraction.

conflicts: []

blockers:
  - Rollback feasibility is NOT_PROVEN until current Oteryn organization/enterprise policy or owner capability to delete the fresh repository is confirmed.
  - Game-specific package/caller evidence remains unresolved for any future Oteryn-v2/Oteryn-Game physical cutover.
  - Atlas extraction remains separately coupled to source ownership/deployment evidence and must not be inferred ready from the existence of Oteryn/Oteryn-Atlas.

next_action: Owner confirms that, as Oteryn organization owner, repository deletion is permitted for a fresh Oteryn/Oteryn before authority handover; then freeze that fact, revalidate exact transaction leases, complete preparation PR gates and advance to CUTOVER_READY only if the single remaining action is the owner GitHub web create flow.
```

## Programme rules

- Live repository state outranks this file when newer.
- Do not cache transient main SHAs here as future authority.
- Do not let the Game->Atlas spatial-profile evidence gap block unrelated repository migration work without a proven dependency.
- Do not create empty target repositories merely to satisfy the target diagram.
- META authority supersedes Platform ADR 0041 only after the META authority is actually canonical.
- Every physical migration step requires exact preflight, rollback and post-cutover verification.
- Keep exactly one `next_action` while the programme is not terminal.
