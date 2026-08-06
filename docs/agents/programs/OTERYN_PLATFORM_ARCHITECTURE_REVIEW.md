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
updated_at: 2026-08-06T10:29:00Z
status: ready
current_review_domain: game-auth-topology-canonical-reconciliation
active_task: null
issue: null
branch: null
pull_request: null
last_completed_domain: game-auth-topology-current-state-review
last_completed_issue: 588
last_completed_pull_request: 722
last_completed_merge: 1919f7eb55f6c2a08058652f422b47f841467009
latest_review_finding_issue: 720
accepted_authority:
  authority_index: docs/architecture/ARCHITECTURE_AUTHORITY.md
  authority_adr: docs/architecture/adr/0022-architecture-authority-index-and-focused-canonical-documents.md
  backlog_adr: docs/architecture/adr/0023-machine-readable-architecture-decision-backlog.md
  backlog_registry: docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  branch_lifecycle_adr: docs/architecture/adr/0024-merged-source-branch-lifecycle-policy.md
  licensing_adr: docs/architecture/adr/0026-proprietary-repository-licensing-policy.md
  vulnerability_disclosure_adr: docs/architecture/adr/0027-confidential-vulnerability-disclosure-policy.md
implementation_handoffs:
  - issue: 658
    scope: deterministic branch inventory, retention metadata, conservative cleanup and recovery proof
  - issue: 720
    scope: reconcile current game-auth topology and Gateway contract delivery status without runtime or production changes
active_architecture_decision_ids: []
architecture_conflicts:
  - Historical duplicate ADR prefixes remain for 0008, 0010, 0011, 0015, 0016, 0017, 0018 and 0021, but the exact accepted path sets are machine-enforced and cannot expand silently.
  - Issue 586 retains historical evidence that delete_branch_on_merge was disabled; ADR 0024 and current metadata prove the accepted current state is enabled.
  - GAME_GATEWAY_IDENTITY_CONTRACT and AUTH_GAME_LOGIN_CONTRACT retain pre-delivery status while merged Gateway and Game Session evidence proves a bounded Oteryn authentication path exists; Issue 720 owns the correction.
ci_architecture_findings:
  - PR 626 separates conditional runtime-tests from an always-emitted aggregate protected test context.
  - Runtime/code changes require the complete MariaDB/PHP suite before the aggregate test gate can pass.
  - Documentation-only changes pass only after fail-closed classification proves runtime tests are NOT_APPLICABLE.
proven:
  - Repository owner selected Option A for the merged source-branch lifecycle policy; PR 653 merged ADR 0024.
  - Repository owner selected Option A for repository licensing; PR 690 merged ADR 0026 and the proprietary/no-permission policy.
  - Repository owner selected Option A for confidential vulnerability disclosure.
  - GitHub Private Vulnerability Reporting is enabled for blakinio/Oteryn-Platform.
  - PR 702 merged ADR 0027, canonical SECURITY.md routing, public-Issue diversion and the empty active decision backlog as ab6ac645595813653618d91574c717fb4d9c7edd after all eight exact-head workflows and independent audit passed.
  - The completed review at docs/agents/reports/OTERYN-20260806-game-auth-topology-current-state-review.md proves the current game-auth documentation drift and preserves exact evidence classification.
  - PR 722 merged the bounded review as 1919f7eb55f6c2a08058652f422b47f841467009 after all exact-head workflows passed.
  - Issue 720 remains the sole documentation-only correction owner and does not overlap active native-protocol PR 542 targets.
derived:
  - The three owner decisions formerly tracked by the architecture decision backlog are resolved and preserved by accepted ADRs.
  - The current game-auth discrepancy is documentation drift, not a new owner decision or runtime architecture choice.
unknown:
  - Exact deployed game-auth topology, alternate-path network isolation and production activation evidence.
conflicts: []
blockers: []
next_action: Execute Issue 720 as one bounded documentation-only canonical reconciliation after confirming live path ownership remains non-overlapping; preserve legacy-v1/native-v2 separation and PRODUCTION_PROVEN=false.
```

## Programme rules

- Proposed ADRs remain proposed until accepted by authoritative owner/repository state.
- Runtime, workflow and infrastructure implementation belongs to remediation after acceptance.
- Do not create a duplicate architecture registry when an existing canonical source can be corrected or extended.
- Keep only the compact `active_architecture_decision_ids` projection here; full record data belongs to `ARCHITECTURE_DECISION_BACKLOG.json`.
- Keep this file compact and update it after a material decision, handoff, blocker, completed review package or rotation.
- Exactly one `next_action` is required while the programme is not terminal.
