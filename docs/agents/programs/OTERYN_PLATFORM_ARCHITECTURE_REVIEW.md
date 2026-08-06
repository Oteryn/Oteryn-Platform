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
updated_at: 2026-08-06T06:27:00Z
status: waiting_owner_decision
current_review_domain: merged-source-branch-lifecycle-policy
active_task: OTERYN-20260806-merged-source-branch-lifecycle-decision
issue: 586
branch: task/OTERYN-20260806-merged-branch-lifecycle-decision
pull_request: null
last_completed_domain: architecture-decision-backlog-implementation
last_completed_issue: 642
last_completed_pull_request: 650
last_completed_merge: 20754620b7a0a4363c70480bda0ee5dff885c9a7
accepted_authority:
  authority_index: docs/architecture/ARCHITECTURE_AUTHORITY.md
  authority_adr: docs/architecture/adr/0022-architecture-authority-index-and-focused-canonical-documents.md
  backlog_adr: docs/architecture/adr/0023-machine-readable-architecture-decision-backlog.md
  backlog_registry: docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
proposed_authority:
  branch_lifecycle_adr: docs/architecture/adr/0024-merged-source-branch-lifecycle-policy.md
active_architecture_decision_ids: ["ARCH-DEC-0001","ARCH-DEC-0002","ARCH-DEC-0003"]
architecture_conflicts:
  - Historical duplicate ADR prefixes remain for 0008, 0010, 0011, 0015, 0016, 0017, 0018 and 0021, but the exact accepted path sets are machine-enforced and cannot expand silently.
  - Issue 586 records delete_branch_on_merge=false as historical evidence, while live repository metadata on 2026-08-06 proves the setting is true.
ci_architecture_findings:
  - PR 626 separates conditional runtime-tests from an always-emitted aggregate protected test context.
  - Runtime/code changes require the complete MariaDB/PHP suite before the aggregate test gate can pass.
  - Documentation-only changes pass only after fail-closed classification proves runtime tests are NOT_APPLICABLE.
proven:
  - Live repository metadata enables automatic merged head-branch deletion, squash merge and auto-merge while disabling merge commits and rebase merges.
  - Full branch enumeration returned 498 refs including main and the current task branch.
  - No accepted exception, retention, recovery, expiry or one-time cleanup policy was found.
  - GitHub branch protection and repository rules can prevent automatic deletion of approved long-lived branches.
  - Proposed ADR 0024 defines Options A, B and C and grants no branch-deletion or repository-setting authority.
derived:
  - Option A is the strongest default because it preserves low-cost automatic cleanup while making every retention exception explicit, protected and fail-closed.
unknown:
  - The repository owner's selected option for ARCH-DEC-0001.
conflicts: []
blockers:
  - Repository-owner selection of A, B or C is required before ADR 0024 can be accepted and ARCH-DEC-0001 can leave decision_required.
next_action: Obtain the repository owner's explicit A, B or C selection in Issue 586 or the decision PR; then accept/reject ADR 0024, transition ARCH-DEC-0001 and create a separate implementation/cleanup handoff without deleting branches in the decision package.
```

## Programme rules

- Proposed ADRs remain proposed until accepted by authoritative owner/repository state.
- Runtime, workflow and infrastructure implementation belongs to remediation after acceptance.
- Do not create a duplicate architecture registry when an existing canonical source can be corrected or extended.
- Keep only the compact `active_architecture_decision_ids` projection here; full record data belongs to `ARCHITECTURE_DECISION_BACKLOG.json`.
- Keep this file compact and update it after a material decision, handoff, blocker, completed review package or rotation.
- Exactly one `next_action` is required while the programme is not terminal.
