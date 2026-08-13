---
task_id: OTERYN-20260813-client-distribution-trust
mode: architecture
issue: 1037
status: blocked
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
- [x] Decision-ready proposal passed exact-head repository CI/governance checks and full-diff self-review found no remaining material finding.

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
  - Repository owner decision for ARCH-DEC-0004: accept Option A (TUF-based role-separated updater repository) or select Option B (custom single signed manifest envelope).
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-13T22:55:00+02:00
head: b279d4de8148206ba1f560d22b7261fb111fe518
branch: docs/OTERYN-20260813-client-distribution-trust
pr: 1038
status: blocked
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
  - Exact decision-ready head b279d4de8148206ba1f560d22b7261fb111fe518 passed all eight triggered workflows: Agent Governance, Native protocol contract, Native protocol contract audits, CI, Phase 7 Production-Like Validation, Platform DB Outage Validation, Game Auth Ticket Concurrency and Edge Security Emulation.
  - Exact-head Agent Governance validated checkpoint tests, policy consistency, prompt contracts, all active task checkpoints and live active-task ownership; exact-head CI classification and required aggregate test gate also passed.
  - PR 1038 full-diff review at the decision-ready head contains exactly five documentation/governance paths and no runtime/workflow change; review-thread and submitted-review inspection found none.
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
  - command: GitHub.fetch_commit_workflow_runs b279d4de8148206ba1f560d22b7261fb111fe518
    result: PASS
    evidence: all eight triggered exact-head workflows completed successfully
  - command: GitHub.fetch_workflow_run_jobs 31742991047 and 31742991085
    result: PASS
    evidence: exact-head Agent Governance checkpoint/policy/live-ownership checks and CI classify/aggregate gate succeeded; runtime-tests were correctly NOT_APPLICABLE for the documentation-only change set
  - command: direct incremental inspection against tools/validation/adr_registry.py and tools/validation/architecture_decision_backlog.py contracts
    result: PASS
    evidence: ADR 0035 uses a unique valid prefix/name and one Proposed lifecycle declaration; README adds the exact ADR entry without changing the preserved duplicate allowlist; ARCH-DEC-0004 has decision_required fields/options/recommendation/owner question, implementation_authorized=false, existing local paths and an exact programme projection
  - command: GitHub PR 1038 changed-file, patch, review-thread and submitted-review inspection
    result: PASS
    evidence: exactly five documentation/governance paths, no remaining material self-review finding, zero review threads and zero submitted reviews
blockers:
  - Repository owner must decide ARCH-DEC-0004 before ADR 0035 can become Accepted or canonical architecture can be rewritten.
next_action: Repository owner selects Option A or Option B; then reconcile accepted canonical architecture, implementation handoff and portal work allocation in the same bounded package.
```

## Notes

The initial branch-creation call used an unsupported connector argument form; the branch was then created from the exact verified main SHA without changing scope. During self-review, an initial standalone contract draft was removed because it permitted an independent per-target release timeline inconsistent with the proven current channel-level `is_current` model; ADR 0035 now keeps schema-v1 release coherence explicit and defers any target-specific timeline to a future decision. No runtime code, migration, route, workflow, deployment, production system, protected signing material or external repository is owned by this task. The decision-ready content head is fully validated; this checkpoint/status commit only persists the owner-decision blocker and does not claim the proposed ADR is accepted.