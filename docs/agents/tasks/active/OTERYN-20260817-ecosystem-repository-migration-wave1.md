---
task_id: OTERYN-20260817-ecosystem-repository-migration-wave1
required_reads:
  - AGENTS.md
  - docs/agents/AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/PROJECT_LANES.json
  - docs/architecture/adr/0041-ecosystem-repository-authority-contracts-and-atlas-integration.md
---

# OTERYN-20260817-ecosystem-repository-migration-wave1

## Goal

Execute Wave 1 of `OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION`: reconstruct fresh repository state, classify migration-critical coordinates and publish a fail-closed physical-cutover readiness decision before any rename/create/transfer/extraction.

Issue: #1130
PR: #1131
Branch: `docs/oteryn-ecosystem-repository-migration-wave1`

## Acceptance criteria

- [x] Current source repositories and target-name existence are re-read from GitHub.
- [x] Current authenticated organization membership is checked.
- [x] Current ADR 0041 topology authority is re-read and no superseding topology decision is located.
- [x] Migration-critical Oteryn-v2 control-plane paths are inspected.
- [x] Current Oteryn-v2 Releases state is inspected.
- [x] GHCR/package inventory access is attempted and any access gap is recorded as UNKNOWN, not guessed.
- [x] Migration-relevant Oteryn-Platform native-protocol workflows are inspected.
- [x] Current Otheryn Atlas automation is revalidated against fresh main, including the private Synology deployment path.
- [x] A-G repository-coordinate inventory is persisted.
- [x] Selective Atlas extraction manifest is persisted.
- [x] META, Game, Platform and Atlas receive explicit readiness states and one next action.
- [x] No physical repository rename/create/transfer/extraction is performed while blockers remain.
- [ ] Exact-head repository-selected CI/governance validation passes for the documentation delivery.
- [ ] Whole-diff self-review has zero material findings.
- [ ] PR is integrated through protected main, then task lifecycle is archived and ownership released.

## Ownership

```yaml
owned_paths:
  - docs/architecture/migration/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_READINESS.md
  - docs/architecture/migration/oteryn-repository-coordinate-inventory.json
  - docs/architecture/migration/oteryn-atlas-extraction-manifest.json
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/agents/tasks/active/OTERYN-20260817-ecosystem-repository-migration-wave1.md
modules:
  - ecosystem-repository-migration
  - architecture-governance
project_lane: oteryn-platform-core
blockers:
  - future Oteryn GitHub organization not visible to authenticated account
  - Oteryn-v2 GHCR/package inventory inaccessible through current integration
  - external Actions/reusable-workflow caller inventory not exhaustive
  - Otheryn Atlas remains tied to active private Synology deployment and mixed path ownership
cross_repository_tasks:
  - repository: blakinio/Oteryn-v2
    mode: read_only_discovery
  - repository: blakinio/Otheryn
    mode: read_only_discovery
```

## Context checkpoint

```yaml
checkpoint_version: 2
policy_version: 2
updated_at: 2026-08-17T14:48:00+02:00
phase: validate
execution_mode: chat_github
execution_reason: GitHub connector supports exact repository/branch/file/PR inspection and documentation delivery; destructive repository administration is neither required nor safe in this wave
status: working
head: null
head_resolution: read the exact current head live from PR 1131; this in-branch checkpoint intentionally does not claim its own resulting commit SHA
branch: docs/oteryn-ecosystem-repository-migration-wave1
pr: 1131
issue: 1130
project_lane: oteryn-platform-core
task_kind: discovery
implementation_authorized: false
context_pressure: medium
context_growth: stable
context_score: 8
estimate_confidence: high
decomposition_decision: discovery_first
decomposition_reason: physical cutover is forbidden until repository-coordinate/package/Actions/deployment evidence is complete
invocation_started_at: 2026-08-17T14:21:00+02:00
last_progress_at: 2026-08-17T14:48:00+02:00
ci_checks_for_current_head: 0
ci_check_generation: pending_after_checkpoint
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 1
stall_warnings: 0
proven:
  - Oteryn-Platform baseline 3724349a8738bff8229b83239244120876bdedfd.
  - Oteryn-v2 baseline c8a3ac666845f3e8679e55dd4c84d1e440e830c8.
  - Otheryn advanced during the audit and was refreshed to 5001cb42b9027d763d87a099696136acd3e12e83.
  - Target names blakinio/Oteryn, blakinio/Oteryn-Game and blakinio/Oteryn-Atlas returned Not Found at inspection.
  - Authenticated GitHub organization membership returned an empty organization set.
  - Oteryn-v2 repository-policy application and merge-gate use dynamic current-repository identity for inspected routing-critical calls.
  - Oteryn-v2 Releases endpoint returned an empty list at the inspected baseline.
  - Oteryn-v2 package inventory endpoint returned 403 Resource not accessible by integration.
  - Oteryn-v2 has open draft PRs #314 and #317 at inspection.
  - Platform native-protocol contract and audit workflows inspected in this wave contain no literal old Game executable Actions coordinate.
  - Otheryn current Atlas deployment request explicitly targets private-synology and dispatches the canonical full-world workflow through GITHUB_REPOSITORY.
  - Otheryn full-world Atlas workflow includes [ots, synology] execution, /volume1/docker/otheryn/atlas state and mixed tools/deploy paths.
  - Oteryn-v2 tools/game-atlas-profile-spike exists at the inspected Game baseline and contains README.md plus spike.py.
derived:
  - Accepted four-repository architecture remains valid but physical topology is not ready.
  - Game rename is NO_GO_YET pending external Actions caller and GHCR/package proof.
  - META is architecture-ready but physically blocked on the intended future organization.
  - Platform remains one repository and needs no physical change in Wave 1.
  - Atlas remains extractable-with-refactor but physical extraction is unsafe while mixed ownership and production deployment authority remain coupled.
unknown:
  - exact future Oteryn GitHub organization identity/permissions
  - exhaustive external Actions/reusable-workflow callers of Oteryn-v2
  - exact Oteryn-v2 GHCR/package names, links, consumers and permissions
  - complete path-level Atlas ownership split inside tools/otbm_atlas and tools/otbm_atlas_facts
conflicts: []
first_failure: null
rejected_hypotheses:
  - Creating empty temporary blakinio/Oteryn, blakinio/Oteryn-Game or blakinio/Oteryn-Atlas repositories is not an acceptable substitute for the intended topology.
  - Ordinary GitHub rename redirects are not sufficient proof for GitHub Actions/reusable-workflow consumers.
  - Otheryn Atlas is not dormant code; current main has active private Synology deployment automation.
changed_paths:
  - docs/architecture/migration/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_READINESS.md
  - docs/architecture/migration/oteryn-repository-coordinate-inventory.json
  - docs/architecture/migration/oteryn-atlas-extraction-manifest.json
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/agents/tasks/active/OTERYN-20260817-ecosystem-repository-migration-wave1.md
validation:
  - command: fresh GitHub live-state and target-name readback
    result: PASS
    evidence: source baselines plus target Not Found results recorded in readiness report
  - command: authenticated organization membership inspection
    result: PASS_WITH_BLOCKER
    evidence: zero organizations returned
  - command: migration-critical Game/Platform/Atlas file inspection
    result: PASS_WITH_UNKNOWN_GATES
    evidence: coordinate inventory and Atlas extraction manifest
  - command: runtime/component/browser E2E
    result: NOT_APPLICABLE
    evidence: discovery/documentation only; no executable product or deployment path changed
blockers:
  - intended future Oteryn organization is unavailable to current authenticated account
  - package and external executable-caller proofs are incomplete
  - Atlas deployment/path ownership requires a later separately authorized cutover design
next_action: Validate PR 1131; after integration/archive, owner creates or identifies the intended future Oteryn GitHub organization and makes it visible to the authenticated account.
```

## Safety boundary

No writes were made to `blakinio/Oteryn-v2`, `blakinio/Otheryn`, `blakinio/canary` or `blakinio/otclient`. No Synology, production, DNS, package, secret, payment or live-game mutation was performed. No owner-funded Codex/OpenAI/API call was used.
