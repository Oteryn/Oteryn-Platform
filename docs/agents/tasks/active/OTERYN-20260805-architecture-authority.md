---
task_id: OTERYN-20260805-architecture-authority
programme_id: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
issue: 548
pull_request: 550
status: waiting
agent: ChatGPT
branch: task/OTERYN-20260805-architecture-authority
base_branch: main
exact_base: 3ab77c072dce796b09004c54b649db009a75d524
latest_main_reconciled: a7eb03d49e328e8115adb54e772c9c8366b737d3
created: 2026-08-05T16:48:00+02:00
updated: 2026-08-05T17:21:00+02:00
risk: medium
execution_mode: github-only
task_kind: architecture-review
implementation_authorized: false
production_activation_authorized: false
owned_paths:
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/SYSTEM_ARCHITECTURE.md
  - docs/architecture/adr/README.md
  - docs/architecture/adr/0022-architecture-authority-index-and-focused-canonical-documents.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/tasks/active/OTERYN-20260805-architecture-authority.md
  - docs/agents/reports/OTERYN-20260805-architecture-authority-review.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
shared_path_lease: []
excluded_overlap:
  - PR 542 native-protocol implementation and contracts
  - PR 541 public-edge checkpoint
  - application, workflow, migration, dependency, deployment and infrastructure paths
---

# Canonical architecture authority review

## Goal

Define and implement the accepted ordering of current architecture truth, accepted decisions, implementation evidence and historical planning documents so stale text cannot direct incompatible work.

## Entry evidence

- original trusted base: `main@3ab77c072dce796b09004c54b649db009a75d524`;
- latest main reconciled before implementation: `a7eb03d49e328e8115adb54e772c9c8366b737d3`;
- Issue #548 owns the decision;
- owner accepted Option B on 2026-08-05;
- active PR #542 overlaps native-protocol contracts and is excluded;
- PR #541 public-edge work is excluded;
- merged PR #453 is reused as evidence for module-catalogue drift.

## Acceptance

- [x] inspect live main, programme state, open PRs and overlapping ownership;
- [x] compare current system architecture, roadmap, module catalogue, ADR registry and routing;
- [x] classify proven contradictions, corrections and unknowns;
- [x] compare at least three authority-model alternatives;
- [x] create a deduplicated decision Issue;
- [x] record the owner architecture decision;
- [x] deterministically inventory every ADR path before allocating a number;
- [x] create accepted ADR 0022;
- [x] add the architecture authority index;
- [x] route repository and agent architecture discovery through the index;
- [x] mark historical initial-phase statements in the system architecture;
- [x] persist the final report and programme handoffs;
- [x] keep runtime and overlapping PR paths unchanged;
- [ ] pass exact-head documentation/governance CI;
- [ ] close or archive the task and release ownership after terminal PR state.

## Accepted decision

**Option B — an authority index plus focused canonical documents** was accepted by the repository owner and recorded in:

- Issue #548 comment `5193592765`;
- `docs/architecture/adr/0022-architecture-authority-index-and-focused-canonical-documents.md`;
- `docs/architecture/ARCHITECTURE_AUTHORITY.md`.

## Audit correction

The preliminary package claimed that `REPOSITORY_MAP.md` referenced a missing lowercase architecture overview. Revalidation of both the task branch and current `main` disproved that claim. The final package records the correction and only adds the accepted authority-index route.

The deterministic ADR inventory also expanded the original collision finding: duplicate prefixes exist for `0008`, `0010`, `0011`, `0015`, `0016`, `0017`, `0018` and `0021`. Existing ADRs remain unchanged; the accepted decision uses collision-free prefix `0022`.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-05T17:21:00+02:00
head: 3d587e03394e0c6893523f5d0ee96ee35c7ce522
branch: task/OTERYN-20260805-architecture-authority
pr: 550
status: waiting
context_routes:
  - agent-governance
  - architecture
owned_paths:
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/SYSTEM_ARCHITECTURE.md
  - docs/architecture/adr/README.md
  - docs/architecture/adr/0022-architecture-authority-index-and-focused-canonical-documents.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/tasks/active/OTERYN-20260805-architecture-authority.md
  - docs/agents/reports/OTERYN-20260805-architecture-authority-review.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
proven:
  - Owner accepted Option B on 2026-08-05 and the decision is recorded in Issue 548 and ADR 0022.
  - The ADR directory highest observed prefix before allocation was 0021.
  - Historical duplicate ADR prefixes exist for 0008, 0010, 0011, 0015, 0016, 0017, 0018 and 0021.
  - SYSTEM_ARCHITECTURE mixed first-phase planning with current architecture routing.
  - The preliminary missing-overview-path claim was disproved by branch and main revalidation.
  - PR 453 remains the separate evidence owner for module-catalogue drift.
  - PR 542 and PR 541 owned scopes are excluded.
derived:
  - The accepted authority index prevents historical planning from silently overriding focused current sources.
  - Existing ADR collisions require a compatibility-safe validator and repair decision rather than renumbering in this task.
unknown:
  - Final exact-head CI and PR review/merge result.
conflicts:
  - Existing ADR numeric identifiers are not globally unique; the inventory exposes this without rewriting accepted history.
first_failure:
  marker: Agent Governance checkpoint validation
  evidence: Earlier run 31017720578 failed because PASS_WITH_GOVERNANCE_REMEDIATION is not an allowed machine-contract result value; the final checkpoint uses only PASS or WAITING.
rejected_hypotheses:
  - The governance failure was not a runtime, database, edge, game-auth or production-like regression.
  - Runtime E2E is not required because the final diff is documentation-only.
  - REPOSITORY_MAP did not contain the missing lowercase overview reference on the inspected branch or current main.
changed_paths:
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/SYSTEM_ARCHITECTURE.md
  - docs/architecture/adr/README.md
  - docs/architecture/adr/0022-architecture-authority-index-and-focused-canonical-documents.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/reports/OTERYN-20260805-architecture-authority-review.md
  - docs/agents/tasks/active/OTERYN-20260805-architecture-authority.md
validation:
  - command: deterministic ADR directory inventory on branch and current main
    result: PASS
    evidence: Highest existing prefix was 0021; 0022 was unused; all observed paths and historical collisions are listed in adr/README.md.
  - command: authority and routing content audit
    result: PASS
    evidence: Authority index, ADR, repository map, context routing and system scope use the same precedence model and preserve excluded ownership.
  - command: exact PR changed-path audit
    result: PASS
    evidence: Declared paths are documentation-only; no runtime, workflow, contract, migration, dependency, deployment or infrastructure path is authorized.
  - command: exact-head GitHub Actions
    result: WAITING
    evidence: Final checks must run after this checkpoint commit.
blockers:
  - Await exact-head CI and terminal PR review/merge state.
next_action: Observe the final PR 550 head checks; if required gates pass, present the documentation-only PR for review/merge and close Issue 548 only after terminal PR state.
```

## E2E

`NOT_APPLICABLE`: this task changes architecture and agent-routing documentation only and does not alter runtime behavior.