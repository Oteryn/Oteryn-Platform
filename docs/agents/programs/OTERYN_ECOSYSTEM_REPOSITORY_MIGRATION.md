---
programme_id: OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION
programme_version: 1
canonical_prompt: docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md
required_reads:
  - AGENTS.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
repository: Oteryn/Oteryn-Platform
owner_alias: OTERYN-REPO-MIGRATION
---

# Oteryn Ecosystem Repository Migration — Programme State

## Mission

Move the accepted Oteryn ecosystem topology from logical ownership to verified, bounded physical repository migration without sacrificing history, provenance, CI/release integrity, rollback, security or product delivery.

## Durable state

```yaml
programme_state_version: 1
updated_at: 2026-08-20T08:15:00Z
status: active
active_task: docs/agents/tasks/active/OTERYN-20260820-platform-stale-coordinate-terminal-reconciliation.md
issue: 1171
branch: migration/issue-1171-terminal-reconciliation
pull_request: 1181

ecosystem_topology_authority:
  repository: Oteryn/Oteryn
  repository_id: 1338152366
  path: docs/architecture/adr/0001-ecosystem-topology-authority.md
  status: CANONICAL
  authority_merge: a2672baac544ada81c526e92f0517903865a9ad0
  live_state_reconciliation_merge: 20f87798d6429555031fa4e63e0a115db83adffb
  live_state_reconciliation_pr: 4
  supersedes:
    repository: Oteryn/Oteryn-Platform
    path: docs/architecture/adr/0041-ecosystem-repository-authority-contracts-and-atlas-integration.md
    scope: ecosystem repository topology and META coordination authority
    platform_reconciliation_merge: 77914c8c2fab016273ee32cb1df0799370206e80

target_topology:
  - Oteryn
  - Oteryn-Game
  - Oteryn-Platform
  - Oteryn-Atlas

platform_post_transfer_verification_evidence:
  transaction_id: OTERYN-PLATFORM-TRANSFER-20260818
  provider_transfer_state: POST_TRANSFER_VERIFIED
  provider_transfer_complete: true
  ecosystem_migration_complete: false
  repository: Oteryn/Oteryn-Platform
  repository_id: 1305155726
  exact_transferred_main: 42f6741deacbd3ba9e1c4f609bb1073ebe0cff7b
  bounded_verification_run: 32309057579
  repository_identity: PASS
  branch_protection: PASS
  target_ghcr_publish_readback: PASS
  self_hosted_runner_attachment: PASS
  protected_staging_or_production_operation_performed: false
  package_repository_link_metadata: UNKNOWN_NON_BLOCKING_FOR_PROVIDER_CUTOVER
  remaining_ecosystem_gates:
    - META reconciliation
    - temporary migration-backup terminal disposition

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
  closeout_pr: 1159
  closeout_merge: ac39722ab348c71748e915395787195d2ea20ebb
  agent_governance_run: 32157381009
  ci_run: 32157381123
  delivery_branch_deleted: true
  classification: COMPLETED_RECONCILIATION_NO_PHYSICAL_PLATFORM_MUTATION

platform_post_rename_preflight_evidence:
  task: docs/agents/tasks/archive/OTERYN-20260818-platform-transfer-post-rename-preflight.md
  delivery_pr: 1160
  delivery_final_head: 31486e49705294a86d182bce810548b6c4e68db8
  delivery_merge: 7ea7dfcd11d4c2d94095f6d93516858f7f4c383a
  agent_governance_run: 32159946833
  ci_run: 32159946881
  delivery_branch_deleted: true
  target_collision_resolution: PASS
  classification: COMPLETED_POST_RENAME_PREFLIGHT_NO_PHYSICAL_TRANSFER

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
    repository: Oteryn/Oteryn-Platform
    repository_id: 1305155726
    original_source_repository: blakinio/Oteryn-Platform
    exact_transferred_main: 42f6741deacbd3ba9e1c4f609bb1073ebe0cff7b
    current_repository: Oteryn/Oteryn-Platform
    target_main_protected: true
    target_required_checks:
      - classify-changes
      - test
    transfer_state: POST_TRANSFER_VERIFIED
    bounded_ghcr_cutover: PASS
    repository_runner_attachment: PASS
    migration_backup_repository: Oteryn/Oteryn-Platform-Migration-Backup-20260818
    migration_backup_repository_id: 1338405017
    governance_coordinate_reconciliation: PASS

platform_preparation_evidence:
  readiness_report: docs/architecture/migration/OTERYN_PLATFORM_TRANSFER_READINESS.md
  coordinate_inventory: docs/architecture/migration/oteryn-platform-transfer-inventory.json
  readiness_merge: b39f8ac31e17f0edb07827c178140867a7e5c04f
  owner_neutral_hardening_merge: 6a3b92cae0099b36d4b58048657fbfa8aea7b9bf
  liquid20_scope_removal_merge: 88e1661bbd13ddb36b064d411d54702075f64852
  liquid20_scope_closeout_merge: 77fa480a3f4e847dac98f76e05b6acd27cca4a57
  owner_neutral_repository_code: PASS
  source_main_last_preflight: ac39722ab348c71748e915395787195d2ea20ebb
  source_main_after_preflight_delivery: 7ea7dfcd11d4c2d94095f6d93516858f7f4c383a
  drift_after_last_runtime_sensitive_preflight: DOCUMENTATION_GOVERNANCE_ONLY
  branch_protection_last_verified: PASS
  transfer_capable_connected_surface: false

platform_backup_evidence:
  repository: Oteryn/Oteryn-Platform-Migration-Backup-20260818
  repository_id: 1338405017
  visibility: public
  archived: false
  default_branch: main
  repository_content_classification: MIGRATION_BOOTSTRAP_BACKUP_NOT_CANONICAL_PLATFORM
  bootstrap_pull_request: 1
  bootstrap_pull_request_state: CLOSED_WITHOUT_MERGE
  seed_workflow: .github/workflows/one-off-platform-history-seed.yml
  seed_workflow_run: 32140478830
  seed_source_main: c567da9d9ae444110262774f8febf5a5abab2a90
  seed_git_fsck: PASS
  seed_bundle_verify: PASS
  seed_backup_artifact_id: 9325658054
  seed_head_push_rc: 1
  seed_tag_push_rc: 0
  head_push_result: REJECTED_WORKFLOW_PERMISSION
  target_collision_resolution: PASS_RENAMED_PRESERVING_REPOSITORY_ID

migration_transaction:
  transaction_id: OTERYN-PLATFORM-TRANSFER-20260818
  mutation: transfer
  state: POST_TRANSFER_VERIFIED
  provider_transfer_complete: true
  ecosystem_migration_complete: false
  source_coordinate: blakinio/Oteryn-Platform
  target_coordinate: Oteryn/Oteryn-Platform
  repository_id: 1305155726
  transferred_main: 42f6741deacbd3ba9e1c4f609bb1073ebe0cff7b
  identity_continuity: PASS
  history_pr_continuity: PASS
  connector_admin_write_access: PASS
  branch_protection_required_checks: PASS
  bounded_ghcr_publish_readback:
    run: 32309057579
    result: PASS
    game_gateway_digest: sha256:323fd66336b3de62f82fda69c4c299c78444dfe93481d26420bbc65b1c9b90f7
    platform_digest: sha256:1d1e8f367a2006d117224577cc678a60aee4ed08aae304a49f78b4e7097f07c2
    deploy_runner_digest: sha256:1eb10741adf42262834825e8e2c50dec4edf8e0f2791935727e75a798f83b520
  self_hosted_runner_attachment:
    run: 32309057579
    job: 96248074731
    runner: oteryn-synology-staging
    routing_label: oteryn-staging
    result: PASS
  protected_staging_or_production_operation_performed: false
  material_unknowns:
    - package repository-link metadata unavailable through current token
    - GitHub App user-token installation metadata unavailable through current connector surface
  remaining_ecosystem_gates:
    - cross-repository META reconciliation
    - temporary migration-backup terminal disposition

proven:
  - Oteryn/Oteryn is the canonical META authority and its live repository manifest was reconciled by PR 4 merge 20f87798d6429555031fa4e63e0a115db83adffb after meta-gate PASS.
  - Oteryn/Oteryn-Game completed the history-preserving migration from blakinio/Oteryn-v2 and later repository-administration reconciliation; the source remains legacy migration provenance.
  - Oteryn/Oteryn-Atlas exists with repository-local governance CI and active DYN-ATLAS-001 work; stale Platform repository-absence blocker PR 1141 is closed without merge.
  - Oteryn/Oteryn-Platform is the canonical Platform implementation at repository ID 1305155726 after the physical owner transfer; blakinio/Oteryn-Platform is historical pre-transfer provenance.
  - source main was ac39722ab348c71748e915395787195d2ea20ebb at the post-rename preflight and was protected with required classify-changes and test contexts.
  - bootstrap repository ID 1338405017 survived the owner/admin rename and now exists at Oteryn/Oteryn-Platform-Migration-Backup-20260818.
  - the live Oteryn organization repository inventory contained no current repository named Oteryn-Platform at the post-rename preflight, so the target-coordinate collision is resolved.
  - Platform seed workflow run 32140478830 proved git fsck and full-bundle verification but did not migrate branch refs; head_push_rc was 1 because workflow-bearing refs were rejected without workflows permission.
  - current official GitHub transfer documentation requires source admin access, target-organization creation permission and an absent same-name target; repository/coordinate preconditions were proven in PR 1160.
  - current official GitHub REST documentation states the Transfer a repository endpoint supports GitHub App user access tokens with Administration write; the connected GitHub action surface exposes no such transfer mutation.
  - current official GitHub Packages documentation states granular-permission package links are removed when a linked repository transfers to another owner, so GHCR linkage remains a material cutover verification gate.
  - post-rename preflight PR 1160 passed exact-head Agent Governance and CI and squash-merged as 7ea7dfcd11d4c2d94095f6d93516858f7f4c383a.

derived:
  - Game repository migration is terminal and must not remain in the pending migration queue.
  - Atlas repository creation is terminal; Atlas content/history separation remains independently gated.
  - Platform physical owner transfer is terminally proven at the provider level and must not be re-requested.
  - Platform target GHCR publication/readback and repository-scoped Synology runner attachment are terminally proven for the transfer transaction.
  - Ecosystem MIGRATION_COMPLETE=YES is still forbidden until META reconciliation and temporary migration-backup disposition are terminal.

unknown:
  - GitHub package repository-link metadata unavailable through the current token; operational target publication/readback is proven.
  - GitHub App user-token installation metadata unavailable through the current connector surface; connector admin/write access is proven.
  - Final bounded Atlas selective history/content extraction closeout state.

conflicts: []

cleanup_debt:
  - Oteryn/Oteryn terminal desired-state protection remains under META Issue 3; current protection has `meta-gate`, while reusable `ai-review-gate` enforcement remains independently gated by META PR #15.
  - Oteryn/Oteryn META repository manifest still describes the pre-rename bootstrap target state and requires a separate bounded META reconciliation after this closeout.

blockers:
  - META_TERMINAL_RECONCILIATION_PENDING: META may be updated only after provider closeout is merged and exact residual gates are known.
  - MIGRATION_BACKUP_TERMINAL_DISPOSITION_PENDING: backup repository remains until recovery/evidence obligations are explicitly resolved.

next_action: After this provider coordinate closeout merges, reconcile the META manifest/desired state to the terminal provider facts, then resolve the temporary migration-backup disposition without re-running the completed physical transfer.
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
