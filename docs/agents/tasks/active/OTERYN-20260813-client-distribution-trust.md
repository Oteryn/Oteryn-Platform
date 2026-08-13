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

- [x] Downloads, immutable artifact storage, protected signing and updater-consumer authorities are separated explicitly.
- [x] Stable/beta, exact target identity, minimum-supported-version and mandatory-update semantics are deterministic and fail closed.
- [x] Signed immutable metadata semantics bind artifact identity and update policy with anti-replay and anti-downgrade fencing.
- [x] Withdrawal, release/target revocation, rollback and signing-role lifecycle preserve immutable history and cannot repoint published releases to different bytes.
- [x] Browser Download Center, updater policy and game admission/compatibility remain distinct authorities.
- [x] Existing Issue 948 artifact-reference immutability and truthful supplied-checksum boundaries remain intact.
- [x] Updater schema v1 preserves the proven one-current-release-per-channel Platform model while exact platform/architecture artifacts fail unavailable rather than creating an implicit second release timeline.
- [x] Repository owner selected Option A and ADR 0035 is Accepted.
- [x] Focused canonical client-distribution architecture and Platform implementation handoff exist.
- [ ] Exact final PR head passes repository-required validation and full-diff self-review after accepted-canonical reconciliation.
- [ ] PR/Issue/task lifecycle is terminal and ownership is released.

## Ownership

```yaml
owned_paths:
  - docs/architecture/adr/0035-first-party-client-distribution-and-updater-trust-boundary.md
  - docs/architecture/CLIENT_DISTRIBUTION_ARCHITECTURE.md
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
  - repository-owner acceptance of ARCH-DEC-0004 Option A
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-13T23:51:00+02:00
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
  - Protected main remains 399a8fbed727a8cae2f35fc682bcb2f05bba297d during this architecture rotation.
  - Repository owner accepted ARCH-DEC-0004 Option A in the current invocation on 2026-08-13.
  - Issue 1037 records the owner decision explicitly.
  - ADR 0035 is Accepted and selects TUF-based role-separated updater trust with private signing authority outside Laravel.
  - CLIENT_DISTRIBUTION_ARCHITECTURE.md is the focused canonical Platform architecture for the accepted boundary.
  - ARCHITECTURE_DECISION_BACKLOG.json no longer carries ARCH-DEC-0004 after acceptance.
  - Platform implementation handoff Issue 1039 exists and grants no external-repository, private-signing-key, deployment or production authority.
  - Downloads supports stable/beta channels and Windows/Linux/macOS plus x86_64/arm64/x86 artifact metadata.
  - PublishClientRelease enforces one current release per channel by clearing is_current from other releases in that channel when a new current release is selected.
  - Issue 948 / PR 966 enforce machine-testable immutable artifact references while preserving the no-fetch and supplied-checksum boundary.
  - The pre-decision content head b279d4de8148206ba1f560d22b7261fb111fe518 passed all eight triggered workflows and full-diff review before the owner decision.
derived:
  - Accepted-canonical reconciliation can remain Platform-only; external updater implementation is a later evidence/authority gate.
  - Exact platform/architecture target absence/revocation fails unavailable in schema v1 instead of creating a second platform-specific release timeline.
unknown:
  - Exact maintained TUF implementation/POUF, client trust-bootstrap implementation, protected signing infrastructure and numerical metadata expiry values remain implementation/operations decisions.
  - A future need for platform/architecture-specific current or minimum-version timelines is unproven and requires a separate compatible decision if it emerges.
conflicts: []
first_failure:
  marker: decision-backlog-path-missing-from-initial-task-ownership
  evidence: ARCHITECTURE_DECISION_BACKLOG.json was added after the initial claim and later reconciled into ownership before decision acceptance.
rejected_hypotheses:
  - An administrator-supplied SHA-256 alone establishes publisher origin.
  - A mutable latest/current pointer can be treated as an immutable release identity.
  - Generic owner instruction to continue architecture is equivalent to acceptance of a new security architecture option.
  - Updater schema v1 should silently permit independent current releases per platform/architecture despite the current one-current-release-per-channel Platform model.
changed_paths:
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/tasks/active/OTERYN-20260813-client-distribution-trust.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/architecture/CLIENT_DISTRIBUTION_ARCHITECTURE.md
  - docs/architecture/adr/0035-first-party-client-distribution-and-updater-trust-boundary.md
  - docs/architecture/adr/README.md
validation:
  - command: pre-decision exact-head CI b279d4de8148206ba1f560d22b7261fb111fe518
    result: PASS
    evidence: all eight triggered workflows completed successfully before owner acceptance
  - command: accepted-canonical exact-final-head CI
    result: NOT_RUN
    evidence: canonical routing/release-scope reconciliation is still in progress
blockers: []
next_action: Reconcile canonical authority/release-scope routing to accepted ADR 0035, then perform exact-final-head validation and closeout.
```

## Notes

The initial branch-creation call used an unsupported connector argument form; the branch was then created from the exact verified main SHA without changing scope. During self-review, an initial standalone contract draft was removed because it permitted an independent per-target release timeline inconsistent with the proven current channel-level `is_current` model; ADR 0035 and the focused architecture keep schema-v1 release coherence explicit. Accidental Issue #1040 was created by a connector invocation and immediately closed `not_planned` with no unique scope. No runtime code, migration, route, workflow, deployment, production system, protected signing material or external repository is owned by this task.