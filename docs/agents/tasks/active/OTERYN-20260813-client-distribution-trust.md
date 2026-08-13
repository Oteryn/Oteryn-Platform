---
task_id: OTERYN-20260813-client-distribution-trust
mode: architecture
issue: 1037
status: implementing
programme: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
project_lane: oteryn-platform-core
phase: design
execution_mode: github_connector
---

# OTERYN-20260813-client-distribution-trust

## Goal

Define the first-party client distribution/updater trust boundary for Oteryn Platform without runtime implementation, production activation, protected signing operations or external-repository access.

## Acceptance criteria

- [x] Downloads, immutable artifact storage, protected signing and updater-consumer authorities are separated explicitly in a proposed decision.
- [x] Stable/beta, target identity, minimum-supported-version and mandatory-update semantics are deterministic and fail closed in the proposal.
- [x] Signed immutable metadata semantics bind artifact identity and update policy with anti-replay and anti-downgrade fencing in the proposal.
- [x] Withdrawal, revocation, rollback and signing-key lifecycle preserve immutable history and cannot repoint published releases to different bytes in the proposal.
- [x] Browser Download Center, updater policy and game admission/compatibility remain distinct authorities in the proposal.
- [x] Existing Issue 948 artifact-reference immutability and truthful supplied-checksum boundaries remain intact.
- [ ] Repository owner selects the durable target architecture and the accepted canonical architecture sources are reconciled to that decision.
- [ ] Offline architecture validation and exact-head full-diff self-review pass for the decision-ready proposal.

## Ownership

```yaml
owned_paths:
  - docs/architecture/adr/0035-first-party-client-distribution-and-updater-trust-boundary.md
  - docs/architecture/CLIENT_DISTRIBUTION_ARCHITECTURE.md
  - docs/contracts/CLIENT_DISTRIBUTION_UPDATE_CONTRACT.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/adr/README.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/agents/tasks/active/OTERYN-20260813-client-distribution-trust.md
modules:
  - Downloads
dependencies:
  - Issue 948 / PR 966 immutable artifact-reference repair
  - current Downloads stable/beta and platform/architecture catalogue
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-13T22:52:00+02:00
head: UNKNOWN
branch: docs/OTERYN-20260813-client-distribution-trust
pr: 1038
status: implementing
context_routes:
  - architecture
  - security
  - web-cms
owned_paths:
  - docs/architecture/adr/0035-first-party-client-distribution-and-updater-trust-boundary.md
  - docs/architecture/CLIENT_DISTRIBUTION_ARCHITECTURE.md
  - docs/contracts/CLIENT_DISTRIBUTION_UPDATE_CONTRACT.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/adr/README.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/agents/tasks/active/OTERYN-20260813-client-distribution-trust.md
proven:
  - Protected main started at 399a8fbed727a8cae2f35fc682bcb2f05bba297d with architecture programme ready for risk-based rotation.
  - No open architecture PR or active architecture task owned client distribution hardening before Issue 1037 and draft PR 1038 were created.
  - Downloads already supports stable/beta channels and Windows/Linux/macOS plus x86_64/arm64/x86 target metadata.
  - Issue 948 closed after PR 966 enforced machine-testable immutable artifact references while preserving the no-fetch and supplied-checksum boundary.
  - Portal completeness and work allocation identify signed updater metadata, minimum-version, mandatory-update and withdrawal/revocation semantics as unresolved architecture work.
  - Proposed ADR 0035 and CLIENT_DISTRIBUTION_UPDATE_CONTRACT.md preserve browser distribution, updater trust and game admission as separate authorities.
  - ARCH-DEC-0004 records the unresolved owner decision with Option A recommended and Option B as the viable alternative.
derived:
  - The security-sensitive architecture proposal can be completed inside Platform documentation without inspecting the external client repository.
  - Canonical accepted architecture must not be rewritten until the repository owner selects the durable option.
unknown:
  - Exact updater serialization/library, client trust-anchor implementation, protected signing infrastructure and numerical metadata expiry values remain implementation decisions after architecture acceptance.
conflicts: []
first_failure:
  marker: decision-backlog-path-missing-from-initial-task-ownership
  evidence: ARCHITECTURE_DECISION_BACKLOG.json was added after the initial task claim but was not listed in the first ownership block; this checkpoint repairs both ownership inventories before proposal validation.
rejected_hypotheses:
  - An administrator-supplied SHA-256 alone establishes publisher origin.
  - A mutable latest/current pointer can be treated as an immutable release identity.
  - Generic owner instruction to continue architecture is equivalent to acceptance of a new security architecture option.
changed_paths:
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/tasks/active/OTERYN-20260813-client-distribution-trust.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/architecture/adr/0035-first-party-client-distribution-and-updater-trust-boundary.md
  - docs/architecture/adr/README.md
  - docs/contracts/CLIENT_DISTRIBUTION_UPDATE_CONTRACT.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: decision-ready proposal still requires programme projection repair and exact-head review/CI
blockers: []
next_action: Reconcile the programme projection with ARCH-DEC-0004, complete exact-head proposal validation, then persist the owner decision as the only remaining blocker.
```

## Notes

The initial branch-creation call used an unsupported connector argument form; the branch was then created from the exact verified main SHA without changing scope. No runtime code, migration, route, workflow, deployment, production system, protected signing material or external repository is owned by this task.
