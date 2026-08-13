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
updated_at: 2026-08-13T16:30:00+02:00
status: ready
current_review_domain: next-risk-based-rotation
active_task: null
issue: null
branch: null
pull_request: null
last_completed_domain: architecture-programme-state-reconciliation
last_completed_issue: 1023
last_completed_pull_request: 1023
last_completed_merge: 38775e953bd9740df08620482240b483fde69ecc
latest_review_finding_issue: 1023
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
  native_v2_integration_architecture: docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
  portal_completeness_architecture: docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  player_companion_architecture: docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
  federated_search_architecture: docs/architecture/FEDERATED_SEARCH_ARCHITECTURE.md
  native_runtime_status_projection_contract: docs/contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md
  native_pre_admission_handoff_contract: docs/contracts/OTERYN_V2_PRE_ADMISSION_HANDOFF_CONTRACT.md
  native_public_game_data_projection_contract: docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md
  native_character_authority_command_contract: docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md
  native_entitlement_game_delivery_contract: docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md
implementation_handoffs: []
active_architecture_decision_ids: []
architecture_conflicts:
  - Historical duplicate ADR prefixes remain for 0008, 0010, 0011, 0015, 0016, 0017, 0018 and 0021, but the exact accepted path sets are machine-enforced and cannot expand silently.
  - Issue 586 retains historical evidence that delete_branch_on_merge was disabled; ADR 0024 and current metadata prove the accepted current state is enabled.
ci_architecture_findings:
  - PR 626 separates conditional runtime-tests from an always-emitted aggregate protected test context.
  - Runtime/code changes require the complete MariaDB/PHP suite before the aggregate test gate can pass.
  - Documentation-only changes pass only after fail-closed classification proves runtime tests are NOT_APPLICABLE where policy allows narrower validation; authority/contract paths may still route through broader gates.
  - Historical Issue 858 proved that active-task governance could become invalid while only classify-changes and test were protected required contexts; Issue 858 is closed completed, so any new branch-protection claim must come from fresh live evidence rather than this historical finding.
proven:
  - Repository owner selected Option A for the merged source-branch lifecycle policy; PR 653 merged ADR 0024.
  - Repository owner selected Option A for repository licensing; PR 690 merged ADR 0026 and the proprietary/no-permission policy.
  - Repository owner selected Option A for confidential vulnerability disclosure.
  - GitHub Private Vulnerability Reporting is enabled for blakinio/Oteryn-Platform.
  - PR 702 merged ADR 0027, canonical SECURITY.md routing, public-Issue diversion and the empty active decision backlog as ab6ac645595813653618d91574c717fb4d9c7edd after all eight exact-head workflows and independent audit passed.
  - The completed review at docs/agents/reports/OTERYN-20260806-game-auth-topology-current-state-review.md proved the game-auth documentation drift and preserved exact evidence classification.
  - PR 731 merged the canonical game-auth reconciliation as 3c806583d2a0c12d5698f7c30755c22c48da60a4 with exactly five effective documentation/task paths and zero unresolved review threads.
  - Repository owner explicitly accepted Option A for the Native Character Portfolio / Account Center v2 boundary; PR 859 merged accepted ADR 0030 as 73c2426b37cfd5028fe9fbcec8254cc8aab3bc80 and Issue 857 is closed completed.
  - PR 866 merged ADR 0031 and the focused native Oteryn-v2 integration architecture as 4bbed105a66b55476698c8f6ce4075671b3a10fe after all eight exact-head workflows passed and full 9-file self-review found zero material findings.
  - Issue 863 is closed completed; the Platform task did not mutate the external game repository.
  - PR 875 reconciled the stale Platform native gameplay protocol contract and producer operations guide with accepted ADR 0031, preserving the disabled historical producer/schema as reconciliation evidence rather than current native authority.
  - Exact PR 875 head 4522a99c8fe609cb137b4f07c00d9f79ca1b331b passed all eight triggered workflows and full review found zero material findings before squash merge 3dbe7f28585be2cb0b42a16491a91af270a661ea; Issue 874 closed completed.
  - PR 881 defined the Platform-side native runtime-status/readiness consumer boundary; its final head f792155dddaea7a4237ad341d3254989e2f2f0da passed all eight triggered workflows and squash-merged as 4043edfaf67b9489d050d70e6fb7e32f4bf149c2; Issue 880 closed completed.
  - Issue 888 is closed completed; PR 900 established the bounded native pre-admission semantic contract and squash-merged as 3e75c78d8684a1d22ea1000a9c5b3478a61cddc2 before PR 901 archived its task.
  - PR 903 established the accepted native PublicGameData projection/read-model boundary; later PR 916 added a fail-closed Platform privacy revocation fence without transferring game-source authority.
  - PR 920 established the accepted native Character Authority command/result semantic contract with stable operation identity, authoritative game-domain outcomes and ambiguous-result reconciliation.
  - PR 925 established the accepted entitlement/game-delivery authority split; PR 968 later bounded Profile-B stale authority with explicit validity/revision fencing.
  - PR 933 merged accepted ADR 0032 for portal composition, private tracking and server-specific system ownership; PR 970 later hardened Today composition so private-influenced representations are PRIVATE_PERSONALIZED and never shared-cache/public-cache authority. PR 970 squash-merged as c5229194c56198421d13333901cc8953723603a6 and closed Issue 941.
  - PR 936 merged accepted ADR 0033 and the focused federated-search architecture; PR 947 later added restrictive publication-revocation fencing so stale search/index/cache state cannot outlive a newer restrictive publication decision.
  - Current ADR inventory ends at accepted ADR 0033 and the active architecture decision backlog is empty.
  - PR 1023 reconciled the architecture programme, authority routing and focused native-integration backlog with accepted current-main architecture and squash-merged as 38775e953bd9740df08620482240b483fde69ecc after all eight exact-head workflows passed with zero material review findings.
derived:
  - The Platform core remains a sound Laravel modular monolith; native integration is explicitly separated from Legacy Canary Compatibility.
  - New native Platform consumers use canonical AccountId/CharacterId and explicit command/query/event/projection boundaries instead of inheriting Canary numeric IDs, table shapes, session semantics or gameplay protocol ownership.
  - Platform-side semantic boundaries are now accepted for runtime status/readiness, bounded pre-admission, Character Authority command/results, PublicGameData projections and entitlement/game delivery; exact producer/server transports and runtime implementations remain separate delivery facts.
  - The retained Platform PR 542 native producer, protobuf schema and fixtures are historical reconciliation inputs only; they cannot define final game admission, gameplay session/lease/fencing, reconnect or protocol semantics.
  - Native runtime admission readiness on the Platform side is an intersection of configured Platform policy and fresh applicable current-owner runtime evidence; stale/unavailable evidence fails closed for new admission but cannot be fabricated as authoritative public offline/zero state.
  - PublicPortal Today remains composition rather than a new source-of-truth module; any representation influenced by owner-private state is private/non-shareable and must not inherit public cacheability.
  - Federated public content search belongs to PublicPortal orchestration over source-owned public queries; source publication/privacy decisions remain authoritative and restrictive revisions fence derived index/cache state.
unknown:
  - Exact deployed game-auth topology, alternate-path network isolation and production activation evidence.
  - Exact external/native producer and consumer transport, wire/IDL, runtime implementation, lease/fencing, replay stores, numerical freshness/TTL values and cutover evidence for accepted cross-boundary contracts remain outside this Platform architecture state record.
  - Support/moderation to native game-enforcement command semantics remain a focused Platform architecture question.
  - Native Game Catalog/content ownership versus Legacy Canary Compatibility importers remains a focused Platform architecture question.
  - Native guild identity required by the PublicGameData guild projection remains dependent on an accepted game-owned stable identifier; Platform must not invent one.
conflicts: []
blockers: []
next_action: Select the highest-risk unresolved Platform-owned architecture, repository-structure or CI/CD question from current main after a fresh overlap search; prioritize support/moderation game-enforcement or native Game Catalog/content ownership when still unresolved and unowned, and do not broaden this programme into external-repository implementation.
```

## Programme rules

- Proposed ADRs remain proposed until accepted by authoritative owner/repository state.
- Runtime, workflow and infrastructure implementation belongs to remediation after acceptance.
- Do not create a duplicate architecture registry when an existing canonical source can be corrected or extended.
- Keep only the compact `active_architecture_decision_ids` projection here; full record data belongs to `ARCHITECTURE_DECISION_BACKLOG.json`.
- Keep this file compact and update it after a material decision, handoff, blocker, completed review package or rotation.
- Exactly one `next_action` is required while the programme is not terminal.
