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

Define the first-party client distribution/updater trust boundary for Oteryn Platform without runtime implementation, production activation, private signing-key operations or external-repository access.

## Acceptance criteria

- [ ] Downloads, immutable artifact storage, protected signing and updater-consumer authorities are separated explicitly.
- [ ] Stable/beta, target identity, minimum-supported-version and mandatory-update semantics are deterministic and fail closed.
- [ ] Signed immutable manifest semantics bind artifact identity and update policy with anti-replay and anti-downgrade fencing.
- [ ] Withdrawal, revocation, rollback and signing-key lifecycle preserve immutable history and cannot repoint published releases to different bytes.
- [ ] Browser Download Center, updater policy and game admission/compatibility remain distinct authorities.
- [ ] Existing Issue 948 artifact-reference immutability and truthful supplied-checksum boundaries remain intact.
- [ ] Canonical architecture, module/portal-completeness and portal work-allocation records are reconciled.
- [ ] Offline architecture validation and exact-head full-diff self-review pass.

## Ownership

```yaml
owned_paths:
  - docs/architecture/adr/0035-first-party-client-distribution-and-updater-trust-boundary.md
  - docs/architecture/CLIENT_DISTRIBUTION_ARCHITECTURE.md
  - docs/contracts/CLIENT_DISTRIBUTION_UPDATE_CONTRACT.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
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
updated_at: 2026-08-13T22:36:00+02:00
head: 399a8fbed727a8cae2f35fc682bcb2f05bba297d
branch: docs/OTERYN-20260813-client-distribution-trust
pr: none
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
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/adr/README.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/agents/tasks/active/OTERYN-20260813-client-distribution-trust.md
proven:
  - Protected main started at 399a8fbed727a8cae2f35fc682bcb2f05bba297d with architecture programme ready for risk-based rotation.
  - No open architecture PR or active architecture task owns client distribution hardening.
  - Downloads already supports stable/beta channels and Windows/Linux/macOS plus x86_64/arm64/x86 target metadata.
  - Issue 948 was closed after PR 966 enforced machine-testable immutable artifact references while preserving the no-fetch and supplied-checksum boundary.
  - Portal completeness and work allocation both identify signed updater manifests, minimum-version, mandatory-update and withdrawal/revocation semantics as unresolved architecture work.
derived:
  - The next bounded security-sensitive architecture package can be completed entirely inside Platform documentation without inspecting the external client repository.
unknown:
  - Exact updater serialization, cryptographic suite, client trust-anchor embedding and protected signing infrastructure remain external implementation decisions unless this Platform contract can safely constrain their semantics without repository access.
conflicts: []
first_failure:
  marker: create-branch-base-ref-tool-call
  evidence: Connector rejected the first base_ref form; branch was then created from the exact verified main SHA without altering scope or repository state.
rejected_hypotheses:
  - An administrator-supplied SHA-256 alone establishes publisher origin.
  - A mutable latest/current pointer can be treated as an immutable release identity.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260813-client-distribution-trust.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: architecture package is being designed
blockers: []
next_action: Define ADR 0035 and the Platform-side signed client-distribution contract, then reconcile canonical architecture records.
```

## Notes

No runtime code, migration, route, workflow, deployment, production system, private key or external repository is owned by this task.
