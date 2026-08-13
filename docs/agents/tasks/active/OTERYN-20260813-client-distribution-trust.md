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
- [x] Stable/beta, exact target identity, minimum-supported-version and mandatory-update semantics are deterministic and fail closed in the proposal.
- [x] Signed immutable metadata semantics bind artifact identity and update policy with anti-replay and anti-downgrade fencing in the proposal.
- [x] Withdrawal, release/target revocation, rollback and signing-role lifecycle preserve immutable history and cannot repoint published releases to different bytes.
- [x] Browser Download Center, updater policy and game admission/compatibility remain distinct authorities in the proposal.
- [x] Existing Issue 948 artifact-reference immutability and truthful supplied-checksum boundaries remain intact.
- [x] Updater schema v1 preserves the proven one-current-release-per-channel Platform model while exact platform/architecture artifacts fail unavailable rather than creating an implicit second release timeline.
- [ ] Repository owner selects the durable target architecture and the accepted canonical architecture sources are reconciled to that decision.
- [ ] Offline architecture validation and exact-head full-diff self-review pass for the decision-ready proposal.

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
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-13T23:04:00+02:00
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
  - Protected main started at 399a8fbed727a8cae2f35fc682bcb2f05bba297d with architecture programme ready for risk-based rotation.
  - No open architecture PR or active architecture task owned client distribution hardening before Issue 1037 and draft PR 1038 were created.
  - Downloads supports stable/beta channels and Windows/Linux/macOS plus x86_64/arm64/x86 artifact metadata.
  - PublishClientRelease enforces one current release per channel by clearing is_current from other releases in that channel when a new current release is selected.
  - Issue 948 closed after PR 966 enforced machine-testable immutable artifact references while preserving the no-fetch and supplied-checksum boundary.
  - Portal completeness and work allocation identify signed updater metadata, minimum-version, mandatory-update and withdrawal/revocation semantics as unresolved architecture work.
  - Proposed ADR 0035 preserves browser distribution, updater trust and game admission as separate authorities and preserves channel-level current-release coherence for schema v1.
  - ARCH-DEC-0004 records the unresolved owner decision with Option A recommended and Option B as the viable alternative.
derived:
  - The security-sensitive architecture proposal can be completed inside Platform documentation without inspecting the external client repository.
  - Canonical accepted architecture must not be rewritten until the repository owner selects the durable option.
  - Exact platform/architecture target absence/revocation should fail unavailable in schema v1 rather than silently creating platform-specific current releases.
unknown:
  - Exact updater library/metaformat, client trust-bootstrap implementation, protected signing infrastructure and numerical metadata expiry values remain implementation decisions after architecture acceptance.
  - A future need for platform/architecture-specific current or minimum-version timelines is unproven and requires a separate explicit decision if it emerges.
conflicts: []
first_failure:
  marker: decision-backlog-path-missing-from-initial-task-ownership
  evidence: ARCHITECTURE_DECISION_BACKLOG.json was added after the initial task claim but was not listed in the first ownership block; the ownership inventories were repaired before proposal validation.
rejected_hypotheses:
  - An administrator-supplied SHA-256 alone establishes publisher origin.
  - A mutable latest/current pointer can be treated as an immutable release identity.
  - Generic owner instruction to continue architecture is equivalent to acceptance of a new security architecture option.
  - Updater schema v1 should silently permit independent current releases per platform/architecture despite the current one-current-release-per-channel Platform model.
changed_paths:
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/tasks/active/OTERYN-20260813-client-distribution-trust.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/architecture/adr/0035-first-party-client-distribution-and-updater-trust-boundary.md
  - docs/architecture/adr/README.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: final decision-ready proposal head still requires exact-head CI and full-diff review
blockers: []
next_action: Validate the exact decision-ready PR head, then persist the repository-owner decision as the only remaining blocker before canonical acceptance updates.
```

## Notes

The initial branch-creation call used an unsupported connector argument form; the branch was then created from the exact verified main SHA without changing scope. During self-review, an initial standalone contract draft was removed because it permitted an independent per-target release timeline inconsistent with the proven current channel-level `is_current` model; ADR 0035 now keeps schema-v1 release coherence explicit and defers any target-specific timeline to a future decision. No runtime code, migration, route, workflow, deployment, production system, protected signing material or external repository is owned by this task.
