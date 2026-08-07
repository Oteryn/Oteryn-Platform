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
updated_at: 2026-08-07T22:32:00Z
status: blocked
current_review_domain: native-character-portfolio-account-center-v2
active_task: OTERYN-20260808-native-character-portfolio-context
issue: 857
branch: docs/OTERYN-20260808-native-character-portfolio-decision
pull_request: null
last_completed_domain: game-auth-topology-canonical-reconciliation
last_completed_issue: 720
last_completed_pull_request: 731
last_completed_merge: 3c806583d2a0c12d5698f7c30755c22c48da60a4
latest_review_finding_issue: 857
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
active_architecture_decision_ids: ["ARCH-DEC-0001"]
architecture_conflicts:
  - Historical duplicate ADR prefixes remain for 0008, 0010, 0011, 0015, 0016, 0017, 0018 and 0021, but the exact accepted path sets are machine-enforced and cannot expand silently.
  - Issue 586 retains historical evidence that delete_branch_on_merge was disabled; ADR 0024 and current metadata prove the accepted current state is enabled.
ci_architecture_findings:
  - PR 626 separates conditional runtime-tests from an always-emitted aggregate protected test context.
  - Runtime/code changes require the complete MariaDB/PHP suite before the aggregate test gate can pass.
  - Documentation-only changes pass only after fail-closed classification proves runtime tests are NOT_APPLICABLE where policy allows narrower validation; authority/contract paths may still route through broader gates.
proven:
  - Repository owner selected Option A for the merged source-branch lifecycle policy; PR 653 merged ADR 0024.
  - Repository owner selected Option A for repository licensing; PR 690 merged ADR 0026 and the proprietary/no-permission policy.
  - Repository owner selected Option A for confidential vulnerability disclosure.
  - GitHub Private Vulnerability Reporting is enabled for blakinio/Oteryn-Platform.
  - PR 702 merged ADR 0027, canonical SECURITY.md routing, public-Issue diversion and the empty active decision backlog as ab6ac645595813653618d91574c717fb4d9c7edd after all eight exact-head workflows and independent audit passed.
  - The completed review at docs/agents/reports/OTERYN-20260806-game-auth-topology-current-state-review.md proved the game-auth documentation drift and preserved exact evidence classification.
  - PR 722 merged the bounded current-state review as 1919f7eb55f6c2a08058652f422b47f841467009.
  - Issue 720 was reconciled by documentation-only PR 731; final head a45df8563fcd51d2ec28741bf326734b3a24bfa4 passed all emitted workflows after independent audit Issue 755 recorded PASS_ZERO_MATERIAL_FINDINGS.
  - PR 731 merged the canonical game-auth reconciliation as 3c806583d2a0c12d5698f7c30755c22c48da60a4 with exactly five effective documentation/task paths and zero unresolved review threads.
  - Current Platform Account Center still uses Canary numeric account/player identifiers and row-count-derived character creation availability in its compatibility implementation.
  - Oteryn-v2 merged PR 90 establishes game-owned CharacterId/current AccountId-to-CharacterId ownership and native character lifecycle commands.
derived:
  - The three owner decisions formerly tracked by the architecture decision backlog are resolved and preserved by accepted ADRs.
  - Finding OPA-ARCH-20260806-001 is resolved on main; the focused game-auth contracts, topology and module wording no longer contradict the bounded repository-delivered Gateway path.
  - Native Character Portfolio needs a Platform-side durable boundary before PlayerCompanion or PlatformAPI adopts legacy Canary identifiers.
unknown:
  - Exact deployed game-auth topology, alternate-path network isolation and production activation evidence.
  - Repository-owner selection for ARCH-DEC-0001.
  - Exact Character Portfolio transport, freshness TTL, entitlement exchange and legacy mapping implementation.
conflicts: []
blockers:
  - ARCH-DEC-0001 requires repository-owner selection before Proposed ADR 0030 may become Accepted.
next_action: Obtain the repository-owner decision for ARCH-DEC-0001; if Option A is accepted, promote ADR 0030 and reconcile MODULE_CATALOG, DATA_OWNERSHIP and PORTAL_COMPLETENESS_ARCHITECTURE without runtime changes.
```

## Programme rules

- Proposed ADRs remain proposed until accepted by authoritative owner/repository state.
- Runtime, workflow and infrastructure implementation belongs to remediation after acceptance.
- Do not create a duplicate architecture registry when an existing canonical source can be corrected or extended.
- Keep only the compact `active_architecture_decision_ids` projection here; full record data belongs to `ARCHITECTURE_DECISION_BACKLOG.json`.
- Keep this file compact and update it after a material decision, handoff, blocker, completed review package or rotation.
- Exactly one `next_action` is required while the programme is not terminal.
