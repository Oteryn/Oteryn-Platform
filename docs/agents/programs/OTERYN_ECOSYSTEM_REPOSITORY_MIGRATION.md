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
updated_at: 2026-08-18T08:52:00Z
status: ready
active_task: null
issue: null
branch: null
pull_request: null

ecosystem_topology_authority:
  repository: Oteryn/Oteryn
  path: docs/architecture/adr/0001-ecosystem-topology-authority.md
  status: CANONICAL
  canonical_merge: a2672baac544ada81c526e92f0517903865a9ad0
  supersedes:
    repository: blakinio/Oteryn-Platform
    path: docs/architecture/adr/0041-ecosystem-repository-authority-contracts-and-atlas-integration.md
    scope: ecosystem repository topology and META coordination authority
    platform_reconciliation: COMPLETE
    platform_reconciliation_pr: 1149
    platform_reconciliation_merge: 77914c8c2fab016273ee32cb1df0799370206e80

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
  closeout_merge: fae1127f081a12ef6bc7c85951b819a3031138a6
  archived_task: docs/agents/tasks/archive/OTERYN-20260818-repository-migration-org-access.md
  connector_installation:
    installation_id: 154585379
    account_login: Oteryn
    account_type: Organization
  classification: PROVEN_ORGANIZATION_INTEGRATION_ACCESS

meta_preparation_evidence:
  implementation_pr: 1145
  implementation_final_head: 20b8a73487e74a0b66924662a1d7e2b9f8b1e3e0
  implementation_merge: 860273ba7eb56fd4f6f3b1e1f8cbb765b2c094fe
  closeout_pr: 1146
  closeout_merge: 648cb5edd64d80d3002b19ef6d007d125de1593e
  final_agent_governance_run: 32114183914
  final_ci_run: 32114183887
  archived_task: docs/agents/tasks/archive/OTERYN-20260818-meta-repository-bootstrap.md

meta_create_and_bootstrap_evidence:
  target_repository: Oteryn/Oteryn
  repository_id: 1338152366
  visibility: public
  archived: false
  default_branch: main
  connector_permissions:
    admin: true
    maintain: true
    push: true
    pull: true
    triage: true
  owner_create_result:
    identity: PASS
    visibility: PASS
    integration_access: PASS
    readme_initialization: MISSING_REPAIRABLE
  readme_anchor_repair:
    commit: ef9a8ee8ba16ee6618eecb2511905f1566dec58c
    reason: owner-created repository was empty and the intended README anchor was required before governed branching
  target_bootstrap:
    branch: bootstrap/meta-authority-0001
    pull_request: 1
    final_head: 08a72bc7a9826ff62e2758411a8d31d70d661849
    merge: a2672baac544ada81c526e92f0517903865a9ad0
    changed_paths:
      - AGENTS.md
      - docs/architecture/adr/0001-ecosystem-topology-authority.md
      - ecosystem/repositories.json
    json_validation: PASS
    exact_diff_self_review: PASS
    runtime_e2e: NOT_APPLICABLE
    ci: NOT_CONFIGURED
    main_branch_protection: disabled
    required_status_checks: []
    reviews: 0
    inline_threads: 0
    comments: 0
    source_branch_cleanup: PENDING_CONNECTOR_LACKS_DELETE_REF
  platform_post_create_reconciliation:
    implementation_pr: 1147
    final_head: f8d0ee8cbaa6678184e33fbd83a9265e27d7f105
    merge: bac880386e962224a730aac6952f1c3498e78200
    final_agent_governance_run: 32117192282
    final_ci_run: 32117192288
    reviews: 0
    inline_threads: 0
    comments: 0
    source_branch_deleted: true
    archived_task: docs/agents/tasks/archive/OTERYN-20260818-meta-post-create-bootstrap.md
  platform_adr0041_supersession_reconciliation:
    implementation_pr: 1149
    final_head: 351da55fe0c118725482dcb44b0d81599785f0c7
    merge: 77914c8c2fab016273ee32cb1df0799370206e80
    final_agent_governance_run: 32118404764
    final_ci_run: 32118404842
    adr_body_preservation: PASS_STATUS_BLOCK_ONLY
    reviews: 0
    inline_threads: 0
    comments: 0
    source_branch_deleted: true
    archived_task: docs/agents/tasks/archive/OTERYN-20260818-platform-adr0041-supersession-reconciliation.md

observed_target_coordinates:
  Oteryn:
    repository: Oteryn/Oteryn
    repository_id: 1338152366
    state: EXISTS_BOOTSTRAPPED
    visibility: public
    authority_merge: a2672baac544ada81c526e92f0517903865a9ad0
  Oteryn-Atlas:
    repository: Oteryn/Oteryn-Atlas
    state: EXISTS
    visibility: public
    content_migration_state: PENDING_INDEPENDENT_EVIDENCE
  Oteryn-Platform:
    target_repository: Oteryn/Oteryn-Platform
    current_repository: blakinio/Oteryn-Platform
    state: PENDING_PHYSICAL_MIGRATION
  Oteryn-Game:
    target_repository: Oteryn/Oteryn-Game
    current_repository: blakinio/Oteryn-v2
    state: PENDING_PHYSICAL_MIGRATION
    evidence_note: current coordinate carried from accepted prior authority; no server/game repository inspection in this invocation

migration_transaction:
  transaction_id: OTERYN-META-CREATE-20260818
  mutation: create
  state: COMPLETED
  public_status: COMPLETED
  source_coordinate: none
  target_coordinate: Oteryn/Oteryn
  canonical_preparation_merge: 860273ba7eb56fd4f6f3b1e1f8cbb765b2c094fe
  physical_repository_id: 1338152366
  expected_post_state:
    repository: Oteryn/Oteryn
    owner: Oteryn
    archived: false
    visibility: PUBLIC
    governed_bootstrap: CANONICAL
  actual_post_state:
    repository: Oteryn/Oteryn
    owner: Oteryn
    archived: false
    visibility: PUBLIC
    connector_access: PROVEN_ADMIN_WRITE
    initial_readme: repaired_after_owner_create
    meta_adr: CANONICAL
    repository_manifest: CANONICAL
    target_governance: CANONICAL
  authority_verified: true
  target_identity_verified: true
  target_governance_verified: true
  source_state_verified: true
  active_pr_task_impact_verified: true
  coordinate_inventory_complete_for_cutover: true
  executable_callers_resolved: true
  ci_impact_resolved: true
  package_impact_resolved_or_owner_risk_acceptance_proven: true
  provenance_strategy_verified: true
  target_collision: false
  ownership_conflict: false
  material_unknowns: []
  replay_guard:
    mutation_fingerprint: create_repository:Oteryn/Oteryn
    create_reissue_forbidden: true
    resolved_repository_id: 1338152366
  point_of_no_return:
    reached: true
    authority_handover_reached: true
    authority_handover_merge: a2672baac544ada81c526e92f0517903865a9ad0
  rollback:
    pre_authority_feasibility: PROVEN
    pre_authority_window: CLOSED
    deletion_authority_after_handover: NOT_GRANTED_BY_PREVIOUS_ROLLBACK_PROOF
  post_mutation_validation:
    exact_repository_identity: PASS
    public_visibility: PASS
    archived_false: PASS
    default_branch_anchor: PASS_REPAIRED
    connector_admin_write_access: PASS
    unexpected_template_or_runtime_content: PASS_NONE_FOUND
    dedicated_bootstrap_pr: PASS
    target_governance: PASS
    meta_adr_canonical: PASS
    repository_manifest_canonical: PASS
    platform_adr_supersession_ordering: PASS_COMPLETE

proven:
  - Oteryn/Oteryn exists as public repository ID 1338152366 with GitHub App admin/write access.
  - The missing README was repaired before authority handover without replaying repository creation.
  - Target PR 1 exact diff and JSON validation passed and the PR squash-merged as a2672baac544ada81c526e92f0517903865a9ad0 with clean review hygiene.
  - META ADR 0001 is canonical and explicitly supersedes Platform ADR 0041 for ecosystem repository-topology/META coordination authority.
  - Platform PR 1147 exact final head f8d0ee8cbaa6678184e33fbd83a9265e27d7f105 passed Agent Governance 32117192282 and CI 32117192288, then squash-merged as bac880386e962224a730aac6952f1c3498e78200.
  - Platform ADR 0041 status-only reconciliation PR 1149 final head 351da55fe0c118725482dcb44b0d81599785f0c7 passed Agent Governance 32118404764 and CI 32118404842 and squash-merged as 77914c8c2fab016273ee32cb1df0799370206e80.
  - Platform main ADR 0041 now records its exact canonical META successor while its historical Context and Decision content remains unchanged.
  - No server/game repository was accessed or mutated during the META create/bootstrap and authority-reconciliation work.

derived:
  - The META create and authority-handoff transaction is fully reconciled and terminal.
  - Oteryn/Oteryn is the sole neutral ecosystem topology authority while provider repositories retain provider implementation/schema authority.
  - Game cutover and Atlas extraction remain independently fail-closed and are not made ready by META completion.

unknown:
  - Exhaustive external Actions/reusable-workflow callers of Oteryn-v2.
  - Exact Oteryn-v2 GHCR/package names, links, permissions and consumers.
  - Complete path-level Atlas ownership split needed for selective extraction.

conflicts: []

cleanup_debt:
  - Oteryn/Oteryn branch bootstrap/meta-authority-0001 remains after merged PR 1 because the current connector exposes no delete-ref operation; no unmerged authority remains on it.

blockers:
  - Game-specific package/caller evidence remains unresolved for any future Oteryn-v2/Oteryn-Game physical cutover.
  - Atlas extraction remains separately coupled to source ownership/deployment evidence.

next_action: In a future separately authorized invocation, select one independent remaining migration-readiness transaction for Game or Atlas; do not infer either ready from META completion.
```

## Programme rules

- Live repository state outranks this file when newer.
- Do not cache transient main SHAs here as future authority.
- Do not let Game- or Atlas-specific evidence gaps block unrelated repository migration work without a proven dependency.
- Do not create empty target repositories merely to satisfy the target diagram.
- META ADR 0001 now owns ecosystem topology authority; provider repositories retain provider implementation/schema authority.
- Every remaining physical migration step requires exact preflight, rollback and post-cutover verification.
- Keep exactly one `next_action` while the programme is not terminal.
