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

- [ ] ADR 0026 records Option A as Accepted and defines future relicensing as a separate owner decision.
- [ ] A canonical proprietary notice states that public visibility grants no copying, modification, redistribution, hosting, sublicensing or commercial-use permission.
- [ ] Third-party components, assets, data and materials remain governed by their own rights and are explicitly excluded from the proprietary grant boundary.
- [ ] README and contribution guidance consistently describe the licensing and inbound-contribution boundary.
- [ ] `ARCH-DEC-0002` leaves the active decision backlog while `ARCH-DEC-0003` remains unchanged.
- [ ] Architecture programme projection advances to Issue #588 without inferring its owner decision.
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
updated_at: 2026-08-06T08:41:00Z
invocation_started_at: 2026-08-06T08:41:00Z
last_progress_at: 2026-08-06T08:41:00Z
phase: implement
session_id: chatgpt-20260806-platform-licensing
session_role: implementer
execution_mode: github
execution_reason: bounded repository documentation, architecture registry and pull-request lifecycle work
lease_expires_at: 2026-08-06T09:26:00Z
head: ab37c3caf5c4a3522788a160109cb6bf29ec8a23
branch: docs/OTERYN-20260806-platform-licensing-policy
pr: none
issue: 587
status: implementing
context_routes:
  - architecture
  - agent-governance
  - testing
context_pressure: medium
context_growth: stable
context_score: 7
estimate_confidence: high
decomposition_decision: phased
decomposition_reason: one cohesive decision package followed by required protected merge and lifecycle archival
validation_level: focused
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
  - composer.json already declares license proprietary.
  - No canonical LICENSE file exists on the trusted base.
  - ADR 0025 is the highest allocated ADR and no overlapping licensing branch was found.
derived:
  - A rights-holder-neutral proprietary notice avoids inventing a legal entity or relicensing third-party material.
unknown:
  - Complete provenance of all bundled or referenced third-party assets and data remains unresolved and must remain excluded rather than guessed.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Publish an OSI or source-available license without owner selection and provenance review.
  - Name a copyright holder or legal entity not proven by repository evidence.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260806-platform-licensing-policy.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: decision package is being implemented
blockers:
  - none
next_action: Create the canonical proprietary notice, ADR 0026 and synchronized backlog/programme updates on this branch.
```
