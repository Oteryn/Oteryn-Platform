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
updated_at: 2026-08-08T08:21:00+02:00
status: ready
current_review_domain: next-risk-based-rotation
active_task: null
issue: null
branch: null
pull_request: null
last_completed_domain: native-protocol-authority-reconciliation
last_completed_issue: 874
last_completed_pull_request: 875
last_completed_merge: 3dbe7f28585be2cb0b42a16491a91af270a661ea
latest_review_finding_issue: 874
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
  native_v2_integration_architecture: docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
implementation_handoffs:
  - issue: 658
    scope: deterministic branch inventory, retention metadata, conservative cleanup and recovery proof
  - issue: 858
    scope: close the remaining governance merge-gate/regression gap; do not conflate that CI-governance repair with accepted product architecture
active_architecture_decision_ids: []
architecture_conflicts:
  - Historical duplicate ADR prefixes remain for 0008, 0010, 0011, 0015, 0016, 0017, 0018 and 0021, but the exact accepted path sets are machine-enforced and cannot expand silently.
  - Issue 586 retains historical evidence that delete_branch_on_merge was disabled; ADR 0024 and current metadata prove the accepted current state is enabled.
ci_architecture_findings:
  - PR 626 separates conditional runtime-tests from an always-emitted aggregate protected test context.
  - Runtime/code changes require the complete MariaDB/PHP suite before the aggregate test gate can pass.
  - Documentation-only changes pass only after fail-closed classification proves runtime tests are NOT_APPLICABLE where policy allows narrower validation; authority/contract paths may still route through broader gates.
  - Issue 858 proves that PR 856 introduced an invalid active-task checkpoint while Agent Governance was not a required protected merge gate; PR 859 repaired the task schema, while the branch-protection/regression-control disposition remains Issue 858.
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
  - Issue 863 is closed completed; Oteryn-v2 remained read-only throughout the Platform task.
  - PR 875 reconciled the stale Platform native gameplay protocol contract and producer operations guide with accepted ADR 0031, preserving the disabled historical producer/schema as reconciliation evidence rather than current Oteryn-v2 authority.
  - Exact PR 875 head 4522a99c8fe609cb137b4f07c00d9f79ca1b331b passed all eight triggered workflows and full review found zero material findings before squash merge 3dbe7f28585be2cb0b42a16491a91af270a661ea; Issue 874 closed completed.
derived:
  - The Platform core remains a sound Laravel modular monolith; native Oteryn-v2 integration is now explicitly separated from Legacy Canary Compatibility.
  - New native Platform consumers must use canonical AccountId/CharacterId and explicit command/query/event/projection boundaries instead of inheriting Canary numeric IDs, table shapes, session semantics or gameplay protocol ownership.
  - ADR 0031 resolves the former target native-protocol ownership conflict while preserving current Canary/transitional implementation evidence for migration.
  - The retained Platform PR 542 native producer, protobuf schema and fixtures are historical reconciliation inputs only; they cannot define Oteryn-v2 final admission, gameplay session/lease/fencing, reconnect or protocol semantics.
unknown:
  - Exact deployed game-auth topology, alternate-path network isolation and production activation evidence.
  - Deferred P1/P2 Platform-v2 contract details recorded in the focused architecture remain intentionally unresolved, including exact Oteryn-v2 game-admission/session/lease handoff semantics.
conflicts: []
blockers: []
next_action: Select the highest-risk unresolved Platform architecture, repository-structure or CI/CD question from current main after a fresh overlap search; preserve ADR 0031 and the historical-only native producer classification unless higher-ranked accepted authority changes.
```

## Programme rules

- Proposed ADRs remain proposed until accepted by authoritative owner/repository state.
- Runtime, workflow and infrastructure implementation belongs to remediation after acceptance.
- Do not create a duplicate architecture registry when an existing canonical source can be corrected or extended.
- Keep only the compact `active_architecture_decision_ids` projection here; full record data belongs to `ARCHITECTURE_DECISION_BACKLOG.json`.
- Keep this file compact and update it after a material decision, handoff, blocker, completed review package or rotation.
- Exactly one `next_action` is required while the programme is not terminal.
