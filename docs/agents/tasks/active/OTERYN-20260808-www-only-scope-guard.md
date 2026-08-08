---
task_id: OTERYN-20260808-www-only-scope-guard
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
search_first:
  - Platform PR #931
optional_reads: []
---

# OTERYN-20260808 WWW-only repository scope guard

## Goal

Persist the project-owner rule that work in this project defaults to **WWW Platform only** (`blakinio/Oteryn-Platform`) and that every server/game repository requires separate explicit owner permission before any access, inspection, audit or mutation.

## Owner instruction

Server/game repositories, including `blakinio/Oteryn-v2` and any repository whose primary responsibility is game server, runtime, gameplay protocol or server persistence, must **not be accessed, read, searched, fetched, audited, branched, edited, reviewed, merged or otherwise operated on unless the project owner first grants explicit separate permission for server-repository work**.

Generic commands such as `dzialaj dalej`, `kontynuuj`, autonomous continuation, architecture continuation, audit, repair or implementation while operating in the Platform project do **not** authorize any server-repository operation.

If Platform work appears to require server-side evidence, stop before accessing that repository and request explicit owner permission.

This guard exists because an FND-04 server architecture task was accidentally started during a Platform-only session. The owner explicitly allowed one final server checkpoint recording that the already-created FND-04 changes must later be completed and independently audited. That checkpoint does not authorize any further server access.

## Acceptance criteria

- [x] Root agent bootstrap contains WWW-only default scope.
- [x] Explicit separate owner permission is required before any server-repository access or mutation.
- [x] Generic continuation commands cannot expand Platform scope into server repositories.
- [x] No external/server repository is touched by this Platform governance task.
- [ ] Required exact-head governance/CI passes.
- [ ] PR #931 merges to protected `main` and this task is archived/released.

## Ownership

```yaml
owned_paths:
  - AGENTS.override.md
  - docs/agents/tasks/active/OTERYN-20260808-www-only-scope-guard.md
modules:
  - agent-governance
dependencies:
  - protected main branch
blockers: []
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-08T21:55:00+02:00
status: validating
phase: governance_scope_guard_validation
branch: docs/OTERYN-20260808-www-only-scope-guard
head: 5f34d40ce19ca401a411c801840147e71ef78eab
pr: 931
context_routes:
  - agent-governance
  - architecture
owned_paths:
  - AGENTS.override.md
  - docs/agents/tasks/active/OTERYN-20260808-www-only-scope-guard.md
repository_mutation_authorization: PROVEN
external_mutation_scope_authorization: NOT_AUTHORIZED
staging_deployment_authorization: NOT_AUTHORIZED
proven:
  - The project owner explicitly restricted this project to blakinio/Oteryn-Platform WWW work by default.
  - The project owner explicitly requires separate permission before any server-repository access or mutation.
  - PR #931 contains only Platform governance/task documentation.
  - The accidentally started FND-04 server work received one owner-authorized final checkpoint and is not authorized for further access from this Platform task.
derived:
  - Future generic continuation commands inside this Platform project cannot be interpreted as implicit permission to inspect or modify server repositories.
unknown: []
conflicts: []
first_failure:
  marker: agent-governance-checkpoint-validation
  evidence: Initial PR #931 Agent Governance run 31275559536 rejected the non-canonical task checkpoint shape before this repair.
rejected_hypotheses:
  - Generic Platform continuation implicitly authorizes cross-repository server work.
  - Read-only server access is permitted without separate owner permission.
changed_paths:
  - AGENTS.override.md
  - docs/agents/tasks/active/OTERYN-20260808-www-only-scope-guard.md
validation:
  - command: owner-scope reconciliation
    result: PASS
    evidence: root scope guard now explicitly requires separate permission before any server-repository operation
  - command: runtime E2E
    result: NOT_APPLICABLE
    evidence: documentation/governance-only change with no executable runtime behavior
blockers: []
next_action: Validate PR #931 on the exact current head, merge when protected checks pass, then archive this governance task. Do not access any server repository.
```
