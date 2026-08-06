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
updated_at: 2026-08-06T10:22:00Z
status: validating
current_review_domain: game-auth-topology-current-state-review
active_task: docs/agents/tasks/active/OTERYN-20260806-game-auth-topology-review.md
issue: 720
branch: docs/OTERYN-20260806-game-auth-topology-review
pull_request: 722
last_completed_domain: confidential-vulnerability-disclosure-policy
last_completed_issue: 588
last_completed_pull_request: 702
last_completed_merge: ab6ac645595813653618d91574c717fb4d9c7edd
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
  - GAME_GATEWAY_IDENTITY_CONTRACT and AUTH_GAME_LOGIN_CONTRACT retain pre-delivery status while merged Gateway and Game Session evidence proves a bounded Oteryn authentication path exists.
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
  - Reviewed main d12a4f4a14db0319a8563cb16b1d92a7b1e117b8 contains the separately deployable Game Gateway and terminal Phase 4 delivery evidence.
  - Current canonical game-auth status and topology documents disagree with that merged evidence; Issue 720 owns the bounded correction.
  - PR 722 contains the bounded review report and lifecycle state only.
derived:
  - The three owner decisions formerly tracked by the architecture decision backlog are resolved and preserved by accepted ADRs.
  - The current game-auth discrepancy is documentation drift, not a new owner decision or runtime architecture choice.
unknown:
  - Exact deployed game-auth topology, alternate-path network isolation and production activation evidence.
conflicts: []
blockers: []
next_action: Validate PR 722 on its exact final head, archive the review task after protected merge and then execute Issue 720 as the next documentation-only canonical reconciliation if live ownership remains non-overlapping.
```

## Programme rules

- Proposed ADRs remain proposed until accepted by authoritative owner/repository state.
- Runtime, workflow and infrastructure implementation belongs to remediation after acceptance.
- Do not create a duplicate architecture registry when an existing canonical source can be corrected or extended.
- Keep only the compact `active_architecture_decision_ids` projection here; full record data belongs to `ARCHITECTURE_DECISION_BACKLOG.json`.
- Keep this file compact and update it after a material decision, handoff, blocker, completed review package or rotation.
- Exactly one `next_action` is required while the programme is not terminal.
