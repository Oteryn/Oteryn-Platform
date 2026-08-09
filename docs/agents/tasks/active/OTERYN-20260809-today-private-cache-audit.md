---
task_id: OTERYN-20260809-today-private-cache-audit
programme_id: OTERYN_PLATFORM_CONTINUOUS_AUDIT
project_lane: oteryn-platform-content
task_kind: audit
implementation_authorized: false
execution_mode: github
execution_reason: WWW Platform architecture/security audit is fully evidenced in the canonical repository
status: validating
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
  - docs/architecture/adr/0032-portal-composition-tracking-and-server-system-ownership.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
search_first:
  - Issue #941
  - PR #933 review history
  - open audit-repair findings, active tasks and open PR ownership
  - Today/command-centre private cache isolation Issues
optional_reads: []
---

# OTERYN-20260809-today-private-cache-audit

## Goal

Audit the accepted `Today` / command-centre composition boundary for confidentiality when one PublicPortal representation mixes public content with authenticated owner-private PlayerCompanion routines, goals and tracked signals. Record material findings without implementing remediation.

## Acceptance criteria

- [x] Refresh protected main, live active tasks and independent finding ownership after the prior audit closeout.
- [x] Preserve OPA-SEC-0005 / Issue #938 as an independent non-overlapping owner.
- [x] Audit ADR 0032 plus Portal Completeness and PlayerCompanion focused architecture from primary evidence.
- [x] Inspect PR #933 review history for already-raised/repaired private-cache concerns.
- [x] Falsify the two-user and authenticated/guest cache-reuse negative paths.
- [x] Search open/closed Issues and finding identities before routing a finding.
- [x] Route OPA-SEC-0006 / Issue #941 with complete independent remediation metadata.
- [x] Keep Issue #941 remediation paths forbidden to the auditor.
- [x] Open bounded audit PR #942.
- [ ] Complete exact-final-head self-review, fresh review, required CI, zero unresolved material threads, merge and lifecycle closeout.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260809-today-private-cache-audit.md
  - docs/agents/reports/OTERYN-20260809-today-private-cache-audit.md
modules:
  - PublicPortal
  - PlayerCompanion
  - architecture-governance
dependencies:
  - ADR 0032 / PR #933
  - Issue #941 independent remediation owner
  - Issue #938 independent federated-search remediation owner
blockers:
  - none
cross_repository_tasks:
  - none
forbidden_paths:
  - docs/architecture/adr/0032-portal-composition-tracking-and-server-system-ownership.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
  - docs/architecture/FEDERATED_SEARCH_ARCHITECTURE.md
  - docs/architecture/adr/0033-federated-content-search-and-discoverability.md
  - app/**
  - routes/**
  - resources/**
  - database/**
  - tests/**
  - deploy/**
  - .github/workflows/**
  - external repositories
```

## Audit result

One material finding is proven:

- **OPA-SEC-0006 / Issue #941 — HIGH / P1**: the accepted Today composition is privacy-aware at source/composition time, but does not define the representation/cache isolation invariant that prevents an authenticated owner's private cards from entering a shared page/fragment/CDN/proxy cache or being reused for another owner/guest after session or authorization changes.

No current runtime disclosure is proven. ADR 0032 explicitly remains architecture authority and does not deliver a Today route, cache or UI.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-09T08:40:00Z
invocation_started_at: 2026-08-09T08:34:40Z
last_progress_at: 2026-08-09T08:40:00Z
head: OUT_OF_BAND_FINAL_HEAD_AFTER_THIS_CHECKPOINT
branch: audit/OTERYN-20260809-today-private-cache
pr: 942
status: validating
phase: exact-head-validation
session_id: agent-20260809-0834-today-private-cache
session_role: auditor
project_lane: oteryn-platform-content
execution_mode: github
context_routes:
  - agent-governance
  - architecture
  - security
  - web-cms
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260809-today-private-cache-audit.md
  - docs/agents/reports/OTERYN-20260809-today-private-cache-audit.md
context_pressure: medium
context_growth: stable
context_score: 7
estimate_confidence: high
decomposition_decision: single
validation_level: documentation_exact_head
invocation_budget_minutes: 60
ci_checks_for_current_head: 0
ci_check_generation: ready
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 0
stall_warnings: 0
proven:
  - Protected main at selection is 1e00d6de235588f8314ec8dae8c4bdb63e5068f9 after lifecycle closeout #940.
  - Active tasks at selection were only the unrelated blocked public-domain repair and native-auth production-verification records plus `.gitkeep`.
  - OPA-SEC-0005 / Issue #938 is open, agent:ready and unclaimed and owns only federated-search ADR 0033/focused architecture paths.
  - ADR 0032 permits PublicPortal Today to compose public CMS/LiveOps/PublicGameData material with authenticated PlayerCompanion owner-private routines, goals and tracked signals.
  - Portal Completeness says personalized cards remain owner-private and are omitted for guests.
  - PlayerCompanion says tracking preferences and derived signal history are owner-private by default and require authenticated owner access.
  - ADR 0032 does not implement Today routes/UI/cache and therefore no current leak is proven.
  - None of the three accepted architecture documents states that a mixed personalized Today representation is private/non-share-cacheable or defines owner/guest cache partitioning, CDN/proxy behavior, private cache identity, or session/privacy transition fencing.
  - PR #933 review history contains a material ADR-authority repair and privacy/freshness boundary review, but no Today response-cache isolation finding.
  - Open/closed duplicate searches found no existing owner for authenticated/guest Today private-cache isolation and OPA-SEC-0006 was unused.
  - Issue #941 independently owns the architecture repair and is agent:ready with deterministic lock branch repair/issue-941.
  - PR #942 changes only the two declared audit documentation paths before exact-head validation.
derived:
  - Correct owner authorization at view-model construction does not protect a later materialized response if a shared cache can replay that representation without re-running authorization.
  - Mixed public/private composition requires an explicit private response/cache boundary before implementation even if public sub-fragments may later be cached independently.
unknown: []
conflicts: []
first_failure:
  marker: today-private-cache-isolation-gap
  evidence: accepted architecture defines owner-private content and guest omission but no representation/cache isolation invariant after composition
rejected_hypotheses:
  - Source privacy checks alone make a cached mixed representation safe; a cache hit may bypass recomposition and authorization.
  - Issue #938 duplicates this root cause; it governs federated public-search publication revocation, not authenticated owner-private Today composition.
  - PR #933 already reviewed private cache isolation; its material review history contains no such finding.
  - A production Today leak exists now; the Today route/cache implementation does not exist under ADR 0032.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260809-today-private-cache-audit.md
  - docs/agents/reports/OTERYN-20260809-today-private-cache-audit.md
validation:
  - command: live main / ownership reconciliation
    result: PASS
    evidence: non-overlapping WWW-only domain selected from main 1e00d6de235588f8314ec8dae8c4bdb63e5068f9
  - command: ADR 0032 / Portal Completeness / PlayerCompanion negative-path review
    result: PASS
    evidence: two-user and auth/guest cache replay are not ruled out by the accepted contract
  - command: PR #933 review-history inspection
    result: PASS
    evidence: no private-cache isolation finding
  - command: open/closed Issue plus OPA-SEC-0006 duplicate search
    result: PASS
    evidence: no duplicate; Issue #941 created as independent owner
  - command: PR #942 changed-path inspection
    result: PASS
    evidence: two audit documentation paths only
  - command: runtime/browser E2E for audit deliverable
    result: NOT_APPLICABLE
    evidence: audit documentation only; Today runtime is not implemented
  - command: exact-final-head self-review / fresh review / repository CI
    result: NOT_RUN
    evidence: checkpoint commit creates the final validation generation
blockers:
  - none
next_action: Validate the unchanged final PR #942 head with exact-head self-review, fresh review, required Agent Governance/CI and zero unresolved threads; merge only if all gates pass, then perform required lifecycle archive/programme reconciliation.
```
