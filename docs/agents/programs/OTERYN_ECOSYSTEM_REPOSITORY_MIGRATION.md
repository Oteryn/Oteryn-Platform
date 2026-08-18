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
updated_at: 2026-08-18T15:59:00Z
status: blocked
active_task: null
issue: null
branch: null
pull_request: null

ecosystem_topology_authority:
  repository: Oteryn/Oteryn
  repository_id: 1338152366
  path: docs/architecture/adr/0001-ecosystem-topology-authority.md
  status: CANONICAL
  authority_merge: a2672baac544ada81c526e92f0517903865a9ad0
  live_state_reconciliation_merge: 20f87798d6429555031fa4e63e0a115db83adffb
  live_state_reconciliation_pr: 4
  supersedes:
    repository: blakinio/Oteryn-Platform
    path: docs/architecture/adr/0041-ecosystem-repository-authority-contracts-and-atlas-integration.md
    scope: ecosystem repository topology and META coordination authority
    platform_reconciliation_merge: 77914c8c2fab016273ee32cb1df0799370206e80

target_topology:
  - Oteryn
  - Oteryn-Game
  - Oteryn-Platform
  - Oteryn-Atlas

completed_transactions:
  meta_create:
    transaction_id: OTERYN-META-CREATE-20260818
    state: COMPLETED
    target_repository: Oteryn/Oteryn
    repository_id: 1338152366
    authority_merge: a2672baac544ada81c526e92f0517903865a9ad0
    meta_ci_merge: 2351e40aa831458f6c579e182f2968d0b33db99e
  game_history_migration:
    transaction_class: history_preserving_copy
    source_repository: blakinio/Oteryn-v2
    source_repository_id: 1323412342
    preserved_source_main: 16afdf31a15bd49d454cdbcdd98fa7ec72213ef9
    target_repository: Oteryn/Oteryn-Game
    target_repository_id: 1338291140
    reconciliation_pr: 4
    reconciliation_merge: d85a5a075aaf72ec88cf2f4167f1aab2ab2ba3a9
    administration_reconciliation_pr: 6
    administration_closeout_pr: 7
    state: COMPLETED
    source_retention: READ_ONLY_LEGACY_MIGRATION_PROVENANCE

live_state_reconciliation_evidence:
  task: docs/agents/tasks/archive/OTERYN-20260818-repository-migration-live-reconciliation.md
  delivery_pr: 1158
  delivery_final_head: 3ee42f97d444aa0d3e1ac3ef7829b803f95f7952
  delivery_merge: 239d86491a3fc397d50952ff2588aaa6633fe7b3
  agent_governance_run: 32157381009
  ci_run: 32157381123
  delivery_branch_deleted: true
  classification: COMPLETED_RECONCILIATION_NO_PHYSICAL_PLATFORM_MUTATION

observed_target_coordinates:
  Oteryn:
    repository: Oteryn/Oteryn
    repository_id: 1338152366
    state: ACTIVE_CANONICAL_META
    visibility: public
    main_branch_protection: PENDING_ADMIN_FOLLOWUP
    protection_issue: 3
  Oteryn-Game:
    repository: Oteryn/Oteryn-Game
    repository_id: 1338291140
    current_repository: Oteryn/Oteryn-Game
    migration_source: blakinio/Oteryn-v2
    state: HISTORY_PRESERVING_MIGRATION_COMPLETE
    visibility: public
  Oteryn-Atlas:
    repository: Oteryn/Oteryn-Atlas
    repository_id: 1337995824
    current_repository: Oteryn/Oteryn-Atlas
    state: ACTIVE_REPOSITORY_CONTENT_MIGRATION_INDEPENDENTLY_GATED
    visibility: public
    active_task: docs/agents/tasks/active/DYN-ATLAS-001-semantic-thais-z7-proof.md
  Oteryn-Platform:
    intended_target_repository: Oteryn/Oteryn-Platform
    observed_bootstrap_repository: Oteryn/Oteryn-platform
    observed_bootstrap_repository_id: 1338405017
    current_repository: blakinio/Oteryn-Platform
    source_repository_id: 1305155726
    source_main_last_cutover_preflight: 77fa480a3f4e847dac98f76e05b6acd27cca4a57
    source_main_after_reconciliation_delivery: 239d86491a3fc397d50952ff2588aaa6633fe7b3
    bootstrap_main_sha: db381488697eee315bdf5840ab0d4f8807f7bfb0
    state: TARGET_COORDINATE_OCCUPIED_BY_BOOTSTRAP_ONLY_REPOSITORY_TRANSFER_PENDING

platform_preparation_evidence:
  readiness_report: docs/architecture/migration/OTERYN_PLATFORM_TRANSFER_READINESS.md
  coordinate_inventory: docs/architecture/migration/oteryn-platform-transfer-inventory.json
  readiness_merge: b39f8ac31e17f0edb07827c178140867a7e5c04f
  owner_neutral_hardening_merge: 6a3b92cae0099b36d4b58048657fbfa8aea7b9bf
  owner_neutral_repository_code: PASS
  source_main_advanced_after_hardening: true
  source_main_last_cutover_preflight: 77fa480a3f4e847dac98f76e05b6acd27cca4a57
  transfer_capable_connected_surface: false

platform_bootstrap_target_evidence:
  repository: Oteryn/Oteryn-platform
  repository_id: 1338405017
  visibility: public
  default_branch: main
  repository_content_classification: MIGRATION_BOOTSTRAP_ONLY
  bootstrap_pull_request: 1
  bootstrap_pull_request_state: OPEN_DRAFT_DO_NOT_MERGE
  seed_workflow: .github/workflows/one-off-platform-history-seed.yml
  seed_workflow_run: 32140478830
  seed_source_main: c567da9d9ae444110262774f8febf5a5abab2a90
  seed_git_fsck: PASS
  seed_bundle_verify: PASS
  seed_backup_artifact_id: 9325658054
  seed_head_push_rc: 1
  seed_tag_push_rc: 0
  head_push_result: REJECTED_WORKFLOW_PERMISSION
  source_main_drift_after_seed: PROVEN

migration_transaction:
  transaction_id: OTERYN-PLATFORM-TRANSFER-20260818
  mutation: transfer
  state: PREPARED
  public_status: NO_GO
  source_coordinate: blakinio/Oteryn-Platform
  target_coordinate: Oteryn/Oteryn-Platform
  source_head: 77fa480a3f4e847dac98f76e05b6acd27cca4a57
  source_repository_id: 1305155726
  pre_state_snapshot:
    source_repository: blakinio/Oteryn-Platform
    source_repository_id: 1305155726
    source_main: 77fa480a3f4e847dac98f76e05b6acd27cca4a57
    source_visibility: public
    source_archived: false
    intended_target_observed_repository: Oteryn/Oteryn-platform
    intended_target_repository_id: 1338405017
    intended_target_main: db381488697eee315bdf5840ab0d4f8807f7bfb0
    intended_target_content: migration_bootstrap_only
  expected_post_state:
    repository: Oteryn/Oteryn-Platform
    repository_id: 1305155726
    owner: Oteryn
    main_head_continuity: exact_source_head_at_fresh_cutover_preflight
    history_provenance: preserved
    connector_admin_write_access: required
    branch_protection_and_required_checks: required_revalidation
    package_and_runner_cutover: required_revalidation
  authority_verified: true
  target_identity_or_absence_verified: true
  target_governance_verified: false
  source_state_verified: true
  evidence_lease_current: false
  active_pr_task_impact_verified: true
  coordinate_inventory_complete_for_cutover: false
  executable_callers_resolved: false
  ci_impact_resolved: false
  package_impact_resolved_or_owner_risk_acceptance_proven: false
  provenance_strategy_verified: true
  target_collision: true
  ownership_conflict: false
  material_unknowns:
    - live GHCR Platform package objects permissions and repository links at the current owner and intended target owner
    - repository-level Synology runner attachment and online behavior immediately after a future owner transfer
    - target organization ruleset and branch-protection result after transfer
  cutover_lock:
    owner: none
    acquired_at: none
    invalidated_by: target collision source-head drift unresolved live package or runner evidence
  replay_guard:
    mutation_fingerprint: transfer_repository:1305155726:Oteryn/Oteryn-Platform
    reissue_forbidden_until_state_proven_not_applied: true
    resume_detection: verify repository ID 1305155726 owner and exact target coordinate before any transfer request
  point_of_no_return:
    reached_when: successful GitHub repository owner transfer of repository ID 1305155726
    consequences: target repository identity settings packages runners and redirects require immediate verification before further migration work
  residual_risk_acceptance:
    status: none
    accepted_by: none
    accepted_at: none
    exact_scope: none
    expiry_or_recheck: none
    evidence: none
  rollback:
    feasibility: NOT_PROVEN
    operation: owner transfer back to blakinio only if GitHub permits and the old coordinate remains recoverable
    trigger: material post-transfer identity history CI package or runner verification failure
    decision_owner: repository owner
    execution_window: must be proven immediately before READY_TO_EXECUTE
    verification: exact repository ID owner main head rulesets packages runners and redirect state
  post_mutation_validation:
    - exact repository ID owner coordinate visibility default branch and main head
    - history branches tags Issues PRs releases and redirects
    - GitHub App admin/write access branch protection and required checks
    - repository secrets and environments existence without reading values
    - GitHub-hosted Actions package publication/read/linkage and self-hosted runner state

proven:
  - Oteryn/Oteryn is the canonical META authority and its live repository manifest was reconciled by PR 4 merge 20f87798d6429555031fa4e63e0a115db83adffb after meta-gate PASS.
  - Oteryn/Oteryn-Game completed the history-preserving migration from blakinio/Oteryn-v2 and later repository-administration reconciliation; the source remains legacy migration provenance.
  - Oteryn/Oteryn-Atlas exists with repository-local governance CI and active DYN-ATLAS-001 work; stale Platform repository-absence blocker PR 1141 is closed without merge.
  - blakinio/Oteryn-Platform remains the canonical Platform implementation at repository ID 1305155726.
  - The exact Platform source main observed at the final cutover preflight before reconciliation delivery was 77fa480a3f4e847dac98f76e05b6acd27cca4a57; delivery PR 1158 later advanced main to 239d86491a3fc397d50952ff2588aaa6633fe7b3 with documentation/governance changes only, so the transaction evidence lease is intentionally false.
  - Oteryn/Oteryn-platform exists at repository ID 1338405017 as bootstrap-only migration scaffolding and therefore occupies the intended transfer coordinate.
  - Platform seed workflow run 32140478830 proved git fsck and full-bundle verification but did not migrate branch refs; head_push_rc was 1 because workflow-bearing refs were rejected without workflows permission.
  - The connected GitHub action surface exposes no repository-transfer operation and no META branch-protection/ruleset mutation operation.

derived:
  - Game repository migration is terminal and must not remain in the pending migration queue.
  - Atlas repository creation is terminal; Atlas content/history separation remains an independent gated workstream rather than a repository-existence blocker.
  - Platform is not READY_TO_EXECUTE and not CUTOVER_READY because multiple material gates remain unsatisfied.
  - Force-pushing a copied Platform history into the bootstrap target would be a different migration operation and would violate the accepted native-transfer transaction/provenance model.

unknown:
  - Live GHCR package object ownership permissions and repository links required for Platform owner transfer.
  - Repository-level Synology runner behavior immediately after Platform owner transfer.
  - Exact post-transfer organization ruleset/protection result.
  - Final bounded Atlas selective history/content extraction closeout state.

conflicts:
  - The intended Platform target coordinate is already occupied by bootstrap repository ID 1338405017 while the canonical transfer transaction requires repository ID 1305155726 to become Oteryn/Oteryn-Platform.

cleanup_debt:
  - Oteryn/Oteryn main protection remains pending under META Issue 3 because the current connector cannot configure branch protection or rulesets.
  - Oteryn/Oteryn-platform bootstrap PR 1 and its one-off workflow remain migration scaffolding and must not be mistaken for canonical Platform implementation.

blockers:
  - TARGET_COORDINATE_COLLISION: Oteryn/Oteryn-platform repository ID 1338405017 occupies the intended same-name transfer coordinate.
  - TRANSFER_OPERATION_UNAVAILABLE: the connected GitHub action surface cannot perform repository owner transfer.
  - GHCR_LIVE_EVIDENCE_UNAVAILABLE: current package object ownership permissions and repository linkage remain unresolved.
  - SELF_HOSTED_RUNNER_CUTOVER_UNPROVEN: repository-level Synology runner behavior after transfer remains unresolved.

next_action: Owner/admin must first free the intended Oteryn/Oteryn-Platform coordinate by an owner-approved rename of the bootstrap-only Oteryn/Oteryn-platform repository while preserving its migration backup evidence; then rerun the Platform transfer preflight from live state.
```

## Programme rules

- Live repository state outranks this file when newer.
- Do not cache transient main SHAs here as future authority.
- The canonical prompt owns Tier-2 transaction states and gates; this durable state must not invent alternate executable semantics.
- Do not create, copy or force-push a competing Platform repository as a substitute for the accepted repository-transfer transaction.
- Do not let completed Game migration or active Atlas work be reclassified as pending merely because an older checkpoint said so.
- Do not let Game- or Atlas-specific evidence gaps block unrelated repository migration work without a proven dependency.
- META ADR 0001 owns ecosystem topology authority; provider repositories retain provider implementation/schema authority.
- Every remaining physical migration step requires exact preflight, a current evidence lease, proven rollback and immediate post-cutover verification.
- Keep exactly one `next_action` while the programme is not terminal.
