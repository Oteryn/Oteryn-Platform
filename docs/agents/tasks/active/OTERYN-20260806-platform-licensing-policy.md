---
task_id: OTERYN-20260806-platform-licensing-policy
programme_id: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: true
issue: 587
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
search_first:
  - overlapping licensing tasks and pull requests
  - existing LICENSE, SPDX, contribution and third-party notice policy
---

# OTERYN-20260806-platform-licensing-policy

## Goal

Record the repository owner's acceptance of `ARCH-DEC-0002` Option A and establish a clear proprietary/no-permission repository boundary without claiming ownership of third-party materials or silently granting contribution rights.

## Acceptance criteria

- [x] ADR 0026 records Option A as Accepted and defines future relicensing as a separate owner decision.
- [x] A canonical proprietary notice states that public visibility grants no copying, modification, redistribution, hosting, sublicensing or commercial-use permission.
- [x] Third-party components, assets, data and materials remain governed by their own rights and are explicitly excluded from the proprietary grant boundary.
- [x] README and contribution guidance consistently describe the licensing and inbound-contribution boundary.
- [x] `ARCH-DEC-0002` leaves the active decision backlog while `ARCH-DEC-0003` remains unchanged.
- [x] Architecture programme projection advances to Issue #588 without inferring its owner decision.
- [ ] Exact-head architecture/backlog/governance validation passes and a fresh independent documentation audit finds no material issue.
- [ ] Issue #587 closes only after protected merge, archival and ownership release.

## Ownership

```yaml
claim_nonce: OTERYN-20260806-platform-licensing-587-01
coordination_key: repository:licensing-policy
branch: docs/OTERYN-20260806-platform-licensing-policy
owned_paths:
  - LICENSE.md
  - THIRD_PARTY_NOTICES.md
  - README.md
  - CONTRIBUTING.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/architecture/adr/README.md
  - docs/architecture/adr/0026-proprietary-repository-licensing-policy.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/tasks/active/OTERYN-20260806-platform-licensing-policy.md
modules:
  - repository-governance
  - architecture
  - legal-documentation
dependencies:
  - repository-owner acceptance of ARCH-DEC-0002 Option A on 2026-08-06
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-06T08:58:00Z
invocation_started_at: 2026-08-06T08:41:00Z
last_progress_at: 2026-08-06T08:58:00Z
phase: validate
session_id: chatgpt-20260806-platform-licensing
session_role: implementer
execution_mode: github
execution_reason: bounded repository documentation, architecture registry and pull-request lifecycle work
lease_expires_at: 2026-08-06T09:43:00Z
head: a7c3d115f3dca18e06541d975e015b4a63fe8f9b
branch: docs/OTERYN-20260806-platform-licensing-policy
pr: 690
issue: 587
status: validating
context_routes:
  - architecture
  - agent-governance
  - testing
context_pressure: medium
context_growth: stable
context_score: 8
estimate_confidence: high
decomposition_decision: phased
decomposition_reason: one cohesive decision package followed by required protected merge and lifecycle archival
validation_level: full
ci_checks_for_current_head: 0
ci_check_generation: draft
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 0
stall_warnings: 0
owned_paths:
  - LICENSE.md
  - THIRD_PARTY_NOTICES.md
  - README.md
  - CONTRIBUTING.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/architecture/adr/README.md
  - docs/architecture/adr/0026-proprietary-repository-licensing-policy.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/tasks/active/OTERYN-20260806-platform-licensing-policy.md
proven:
  - Repository owner explicitly accepted ARCH-DEC-0002 Option A on 2026-08-06.
  - composer.json declares license proprietary.
  - PR 690 contains a canonical proprietary notice and an explicit third-party provenance boundary.
  - ADR 0026 is allocated after ADR 0025 and is listed in the ADR inventory.
  - ARCH-DEC-0002 is removed while ARCH-DEC-0003 remains byte-for-byte equivalent in decision content.
derived:
  - A rights-holder-neutral proprietary notice avoids inventing a legal entity or relicensing third-party material.
unknown:
  - Final exact-head CI, independent audit, protected merge and archive outcome.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Publish an OSI or source-available license without owner selection and provenance review.
  - Name a copyright holder or legal entity not proven by repository evidence.
changed_paths:
  - LICENSE.md
  - THIRD_PARTY_NOTICES.md
  - README.md
  - CONTRIBUTING.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/ARCHITECTURE_DECISION_BACKLOG.json
  - docs/architecture/adr/README.md
  - docs/architecture/adr/0026-proprietary-repository-licensing-policy.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/tasks/active/OTERYN-20260806-platform-licensing-policy.md
validation:
  - command: exact-head GitHub Actions
    result: NOT_RUN
    evidence: final implementation head has just been assembled
  - command: independent documentation audit
    result: NOT_RUN
    evidence: scheduled after exact diff review
blockers:
  - none
next_action: Inspect the exact PR diff, run repository validation, repair material findings and complete independent audit on the frozen head.
```
