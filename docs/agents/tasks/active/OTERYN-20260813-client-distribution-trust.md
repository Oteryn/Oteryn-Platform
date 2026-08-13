---
task_id: OTERYN-20260813-client-distribution-trust
mode: architecture
issue: 1037
status: validating
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
updated_at: 2026-08-13T23:57:00+02:00
head: UNKNOWN
branch: docs/OTERYN-20260813-client-distribution-trust
pr: 1038
status: validating
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
  - Repository owner accepted ARCH-DEC-0004 Option A in the current invocation on 2026-08-13 and Issue 1037 records the decision explicitly.
  - ADR 0035 is Accepted and selects TUF-based role-separated updater trust with private signing authority outside Laravel.
  - CLIENT_DISTRIBUTION_ARCHITECTURE.md is the focused canonical Platform architecture for the accepted boundary and ARCHITECTURE_AUTHORITY.md routes this narrower concern to it.
  - ARCHITECTURE_DECISION_BACKLOG.json no longer carries ARCH-DEC-0004 after acceptance.
  - Portal work allocation classifies Client Distribution hardening as ARCHITECTURE_READY and Platform implementation handoff Issue 1039 exists.
  - Downloads supports stable/beta channels and Windows/Linux/macOS plus x86_64/arm64/x86 artifact metadata; PublishClientRelease enforces one current release per channel.
  - Issue 948 / PR 966 enforce machine-testable immutable artifact references while preserving the no-fetch and supplied-checksum boundary.
  - Accepted-canonical content head 16c6a2c3e352b2cd070caf9db6bc8ba6a64b473f passed all eight triggered workflows.
  - Full seven-path diff review on 16c6a2c3e352b2cd070caf9db6bc8ba6a64b473f found no remaining material architecture, authority, lifecycle or scope defect; review-thread/submitted-review inspection found none.
derived:
  - Generic older portal/module wording about deciding updater trust is superseded for this narrow concern by accepted ADR 0035 and the focused owner routed in ARCHITECTURE_AUTHORITY; implementation availability remains correctly unproven.
  - External updater implementation, maintained TUF profile and protected signing infrastructure remain later evidence/authority gates.
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
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/agents/tasks/active/OTERYN-20260813-client-distribution-trust.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/CLIENT_DISTRIBUTION_ARCHITECTURE.md
  - docs/architecture/adr/0035-first-party-client-distribution-and-updater-trust-boundary.md
  - docs/architecture/adr/README.md
validation:
  - command: pre-decision exact-head CI b279d4de8148206ba1f560d22b7261fb111fe518
    result: PASS
    evidence: all eight triggered workflows completed successfully before owner acceptance
  - command: accepted-canonical content head 16c6a2c3e352b2cd070caf9db6bc8ba6a64b473f
    result: PASS
    evidence: all eight triggered workflows completed successfully; branch was zero commits behind main and PR remained mergeable
  - command: seven-path full-diff and review hygiene inspection
    result: PASS
    evidence: no material finding, zero review threads and zero submitted reviews; ADR inventory newline defect was repaired before this checkpoint
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: architecture-only documentation changes no executable user or integration journey
  - command: terminal metadata exact-head CI
    result: NOT_RUN
    evidence: recovery checkpoint commit creates the terminal CI head and must itself pass before merge
blockers: []
next_action: Resolve the current PR head created by this recovery checkpoint, wait under the bounded terminal-CI contract for all required checks, then squash-merge and archive if review/merge gates remain clean.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 1
  session_id: architecture-20260813-2343
  session_started_at: 2026-08-13T23:43:00+02:00
  checkpointed_at: 2026-08-13T23:57:00+02:00
  last_progress_at: 2026-08-13T23:57:00+02:00
  phase: terminal_exact_head_ci_and_merge
  exact_head: 275dd6ef8a5f3a8fe7a042f18283f237a593cb52
  pull_request: 1038
  active_operation: resolve the new recovery-checkpoint head, then observe its required exact-head CI
  external_run_ids: [31747814872, 31747814851, 31747814896, 31747814886, 31747814845, 31747814926, 31747814925, 31747814892]
  operation_started_at: 2026-08-13T23:57:00+02:00
  wait_deadline_at: 2026-08-14T00:42:00+02:00
  check_generation: ready
  checks_used: 0
  status: active
  safe_to_resume: true
  resume_condition: current PR head has all repository-required checks successful, no unresolved review thread/requested change, remains current-base and mergeable
  next_action: Resolve the PR head produced by this checkpoint commit and begin bounded aggregate terminal-CI observation for that exact head.
```

## Notes

The initial branch-creation call used an unsupported connector argument form; the branch was then created from the exact verified main SHA without changing scope. During self-review, an initial standalone contract draft was removed because it permitted an independent per-target release timeline inconsistent with the proven current channel-level `is_current` model; ADR 0035 and the focused architecture keep schema-v1 release coherence explicit. Accidental Issue #1040 was created by a connector invocation and immediately closed `not_planned` with no unique scope. No runtime code, migration, route, workflow, deployment, production system, protected signing material or external repository is owned by this task.