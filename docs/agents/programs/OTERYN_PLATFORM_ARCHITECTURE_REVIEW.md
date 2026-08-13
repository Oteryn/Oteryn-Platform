---
programme_id: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
programme_version: 2
canonical_prompt: docs/agents/prompts/OTERYN_PLATFORM_ARCHITECTURE_REVIEW_PROGRAM.md
required_reads:
  - docs/agents/OTERYN_PLATFORM_PROGRAM_SCOPE.md
  - docs/agents/SHORT_PROGRAM_INVOCATIONS.md
repository: blakinio/Oteryn-Platform
---

# Oteryn Platform Architecture, Structure and CI Review — Programme State

## Mission

Continuously challenge Platform architecture, repository structure and CI/CD; identify contradictions and missing decisions; compare alternatives; and persist accepted decisions and implementation handoffs without changing runtime code.

## Durable queue

```yaml
programme_state_version: 2
updated_at: 2026-08-13T22:56:00+02:00
status: blocked
current_review_domain: client-distribution-updater-trust
active_task: OTERYN-20260813-client-distribution-trust
issue: 1037
branch: docs/OTERYN-20260813-client-distribution-trust
pull_request: 1038
last_completed_domain: native-game-catalog-content-ownership
last_completed_issue: 1033
last_completed_pull_request: 1034
last_completed_merge: 7a0664cfd7dadf27aef0a33e2308bf4975fb1405
latest_review_finding_issue: 1037
accepted_authority:
  authority_index: docs/architecture/ARCHITECTURE_AUTHORITY.md
  authority_adr: docs/architecture/adr/0022-architecture-authority-index-and-focused-canonical-documents.md
  backlog_adr: docs/architecture/adr/0023-machine-readable-architecture-decision-backlog.md
  backlog_registry: docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  branch_lifecycle_adr: docs/architecture/adr/0024-merged-source-branch-lifecycle-policy.md
  licensing_adr: docs/architecture/adr/0026-proprietary-repository-licensing-policy.md
  vulnerability_disclosure_adr: docs/architecture/adr/0027-confidential-vulnerability-disclosure-policy.md
  character_portfolio_adr: docs/architecture/adr/0030-native-character-portfolio-account-center-v2.md
  native_v2_integration_adr: docs/architecture/adr/0031-native-oteryn-v2-integration-boundary.md
  portal_composition_adr: docs/architecture/adr/0032-portal-composition-tracking-and-server-system-ownership.md
  federated_search_adr: docs/architecture/adr/0033-federated-content-search-and-discoverability.md
  native_game_catalog_content_adr: docs/architecture/adr/0034-native-game-catalog-content-ownership.md
  native_v2_integration_architecture: docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
  portal_completeness_architecture: docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  player_companion_architecture: docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
  federated_search_architecture: docs/architecture/FEDERATED_SEARCH_ARCHITECTURE.md
  native_runtime_status_projection_contract: docs/contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md
  native_pre_admission_handoff_contract: docs/contracts/OTERYN_V2_PRE_ADMISSION_HANDOFF_CONTRACT.md
  native_public_game_data_projection_contract: docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md
  native_character_authority_command_contract: docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md
  native_entitlement_game_delivery_contract: docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md
  native_game_enforcement_command_contract: docs/contracts/OTERYN_V2_GAME_ENFORCEMENT_COMMAND_CONTRACT.md
  native_game_catalog_content_contract: docs/contracts/OTERYN_V2_GAME_CATALOG_CONTENT_CONTRACT.md
implementation_handoffs: []
active_architecture_decision_ids: ["ARCH-DEC-0004"]
architecture_conflicts:
  - Historical duplicate ADR prefixes remain for 0008, 0010, 0011, 0015, 0016, 0017, 0018 and 0021; the accepted exact path sets remain machine-enforced.
ci_architecture_findings:
  - Documentation-only changes use fail-closed path classification; runtime tests may be NOT_APPLICABLE while required aggregate/governance gates still run.
proven:
  - Protected main for this rotation was 399a8fbed727a8cae2f35fc682bcb2f05bba297d.
  - The prior completed architecture domain was native-game-catalog-content-ownership through PR 1034 / merge 7a0664cfd7dadf27aef0a33e2308bf4975fb1405.
  - Issue 1037 and draft PR 1038 now own the bounded client-distribution updater-trust decision.
  - Proposed ADR 0035 preserves the current one-current-release-per-channel model and separates browser release presentation, updater trust and game admission authority.
  - ARCH-DEC-0004 records Option A as recommended and Option B as the viable alternative; implementation remains unauthorized.
  - Decision-ready PR 1038 head b279d4de8148206ba1f560d22b7261fb111fe518 passed all eight triggered workflows, exact-head governance/CI gate inspection and full five-path diff self-review with no outstanding review thread.
derived:
  - Canonical accepted architecture must remain unchanged until the repository owner selects the durable updater-trust option.
  - The proposal can remain Platform-only; exact external updater implementation details are not needed to make the Platform architecture decision.
unknown:
  - Exact updater implementation/library, trust bootstrap, release-publishing infrastructure and numerical metadata expiry values remain implementation details after architecture acceptance.
conflicts: []
blockers:
  - ARCH-DEC-0004 requires repository-owner selection of Option A or Option B before ADR 0035 can become Accepted and canonical architecture can be reconciled.
next_action: Repository owner selects ARCH-DEC-0004 Option A or Option B; then reconcile accepted architecture and implementation handoff in the same bounded package.
```

## Programme rules

- Proposed ADRs remain proposed until accepted by authoritative owner/repository state.
- Runtime, workflow and infrastructure implementation belongs to remediation after acceptance.
- Do not create a duplicate architecture registry when an existing canonical source can be corrected or extended.
- Keep only the compact `active_architecture_decision_ids` projection here; full record data belongs to `ARCHITECTURE_DECISION_BACKLOG.json`.
- Keep this file compact and update it after a material decision, handoff, blocker, completed review package or rotation.
- Exactly one `next_action` is required while the programme is not terminal.
