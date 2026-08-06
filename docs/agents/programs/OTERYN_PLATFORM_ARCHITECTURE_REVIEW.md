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
updated_at: 2026-08-06T06:17:03Z
status: waiting_owner_decision
current_review_domain: merged-source-branch-lifecycle-policy
active_task: null
issue: 586
branch: null
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
active_architecture_decision_ids: ["ARCH-DEC-0001","ARCH-DEC-0002","ARCH-DEC-0003"]
architecture_conflicts:
  - Historical duplicate ADR prefixes remain for 0008, 0010, 0011, 0015, 0016, 0017, 0018 and 0021, but the exact accepted path sets are machine-enforced and cannot expand silently.
ci_architecture_findings:
  - PR 626 separates conditional runtime-tests from an always-emitted aggregate protected test context.
  - Runtime/code changes require the complete MariaDB/PHP suite before the aggregate test gate can pass.
  - Documentation-only changes pass only after fail-closed classification proves runtime tests are NOT_APPLICABLE.
proven:
  - PR 650 merged through protected main as 20754620b7a0a4363c70480bda0ee5dff885c9a7 and closed Issue 642 as completed.
  - Final synchronized head f5c3365b5d0353a988820eaeb41c7e076b4de347 passed all eight emitted workflows, including full runtime-tests and the aggregate protected test gate.
  - The canonical registry contains exactly ARCH-DEC-0001, ARCH-DEC-0002 and ARCH-DEC-0003 and grants no accepted-decision, implementation or activation authority.
  - Issue 586 remains open and is the linked owner-decision route for ARCH-DEC-0001.
  - Zero unresolved review threads remained at merge.
derived:
  - The highest-priority next architecture action is the repository-owner decision for the merged source-branch exception, recovery and cleanup policy.
unknown:
  - The repository owner's selected option for ARCH-DEC-0001.
conflicts: []
blockers:
  - Repository-owner selection in Issue 586 is required before ARCH-DEC-0001 can leave decision_required.
next_action: Present ARCH-DEC-0001 options A, B and C from Issue 586 to the repository owner, record the selected policy in one bounded architecture package, and do not infer acceptance.
```

## Programme rules

- Proposed ADRs remain proposed until accepted by authoritative owner/repository state.
- Runtime, workflow and infrastructure implementation belongs to remediation after acceptance.
- Do not create a duplicate architecture registry when an existing canonical source can be corrected or extended.
- Keep only the compact `active_architecture_decision_ids` projection here; full record data belongs to `ARCHITECTURE_DECISION_BACKLOG.json`.
- Keep this file compact and update it after a material decision, handoff, blocker, completed review package or rotation.
- Exactly one `next_action` is required while the programme is not terminal.
