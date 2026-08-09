---
task_id: OTERYN-20260809-federated-search-revocation-audit
programme_id: OTERYN_PLATFORM_CONTINUOUS_AUDIT
project_lane: oteryn-platform-content
task_kind: audit
implementation_authorized: false
execution_mode: github
execution_reason: Platform-only architecture/security audit can be completed from canonical repository and GitHub evidence
status: validating
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/TERMINAL_ONLY_COMMUNICATION.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/agents/AUDIT_REMEDIATION_ISSUE_TAXONOMY.md
  - docs/architecture/FEDERATED_SEARCH_ARCHITECTURE.md
  - docs/architecture/adr/0033-federated-content-search-and-discoverability.md
search_first:
  - Issue #935 and PR #936 review history
  - open programme:audit-repair findings
  - active tasks and open PR ownership
  - federated search revocation/unpublish/tombstone/index/cache Issues
optional_reads: []
---

# OTERYN-20260809-federated-search-revocation-audit

## Goal

Independently audit the newly accepted WWW Platform federated-search architecture for publication/visibility revocation correctness, especially the interaction between source authority, stale derived indexes, result caches, tombstone propagation and generation rollback. Record confirmed findings without implementing remediation.

## Acceptance criteria

- [x] Refresh protected `main`, active tasks, open PRs and live remediation queue before selecting the domain.
- [x] Confirm previous findings #905 and #908 are terminal and do not remain live owners.
- [x] Audit current ADR 0033 and `FEDERATED_SEARCH_ARCHITECTURE.md` from primary evidence rather than relying on PR summary.
- [x] Inspect PR #936 review history for already-discovered/repaired security or cache findings.
- [x] Test the negative path: newer unpublish/revoke/delete/incompatible decision while an older indexed/cached public result still exists or an older index generation is restored.
- [x] Search open and closed Issues for duplicate ownership before creating a finding.
- [x] Route the confirmed material root cause as OPA-SEC-0005 / Issue #938 with complete taxonomy metadata.
- [x] Keep the audit diff limited to audit evidence/task records; do not edit Issue #938 remediation paths.
- [ ] Complete exact-head self-review, repository-required CI, review hygiene, squash merge and lifecycle archive closeout.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260809-federated-search-revocation-audit.md
  - docs/agents/reports/OTERYN-20260809-federated-search-revocation-audit.md
modules:
  - PublicPortal
  - federated-search architecture
  - continuous-audit governance
dependencies:
  - Issue #935 / PR #936 accepted federated-search architecture
  - Issue #938 independent remediation owner
blockers:
  - none
cross_repository_tasks:
  - none
forbidden_paths:
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

Issue #938 owns the architecture remediation paths. This audit must not repair that contract in the same role.

## Audit result

One material finding is proven:

- **OPA-SEC-0005 / Issue #938 — HIGH / P1**: the accepted architecture requires deterministic unpublish/revoke/delete propagation and permits bounded stale-index/cache behavior, but does not define a monotonic restrictive publication-decision fence, visibility cutoff, fail-closed propagation failure semantics or rollback rule preventing an older index generation from resurrecting a result after a newer revoke.

The audit does **not** prove a current runtime disclosure. ADR 0033 explicitly remains architecture/planned and no federated-search route/index/cache implementation is delivered by this change.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-09T08:19:00Z
invocation_started_at: 2026-08-09T08:12:00Z
last_progress_at: 2026-08-09T08:19:00Z
head: OUT_OF_BAND_AUDIT_PR_HEAD_AFTER_THIS_COMMIT
branch: audit/OTERYN-20260809-federated-search-revocation
pr: none
status: validating
phase: audit-package-validation
session_id: agent-20260809-0812-federated-search-revocation
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
  - docs/agents/tasks/active/OTERYN-20260809-federated-search-revocation-audit.md
  - docs/agents/reports/OTERYN-20260809-federated-search-revocation-audit.md
context_pressure: medium
context_growth: stable
context_score: 7
estimate_confidence: high
decomposition_decision: single
validation_level: documentation_exact_head
invocation_budget_minutes: 60
ci_checks_for_current_head: 0
ci_check_generation: draft
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 0
stall_warnings: 0
proven:
  - Protected main at audit selection is af3c23943106cd10c7eea42f6644ae12e1e69990.
  - Live active tasks are only the blocked public-domain repair and blocked native-auth production-verification records; neither owns federated-search architecture.
  - Live open programme:audit-repair query was empty before this audit finding was created.
  - Issues #905 and #908 are closed completed after their independent repairs and lifecycle closeouts.
  - Open PRs #541 and #338 are independent holds and do not own this audit or Issue #938 remediation paths.
  - Issue #935 is closed after PR #936 accepted ADR 0033 and the focused federated-search architecture.
  - PR #936 had three material review repair cycles covering reverse dependencies and complete privacy-safe semantic response cache identity; its review history contains no publication-revocation/tombstone-ordering finding.
  - The focused architecture says a source becoming unpublished, revoked or incompatible must stop appearing according to canonical source truth.
  - The same architecture permits future bounded stale-index lag, deterministic tombstones, generation-based rebuild/cutover and result caching.
  - Neither ADR 0033 nor the focused architecture defines a monotonic restrictive publication-decision revision/watermark, propagation acknowledgement/fail-closed rule or rollback fence across a newer revoke.
  - Duplicate searches found no open or closed Issue owning this exact federated-search publication/index revocation root cause; OPA-SEC-0005 was unused.
  - Issue #938 now owns the independent architecture repair and is agent:ready with deterministic lock branch repair/issue-938.
derived:
  - A future implementation can satisfy ordinary bounded stale-index/cache freshness while still serving a representation based on an older public decision unless a separate restrictive-decision fence is defined.
  - This is analogous in failure shape to historical PublicGameData Issue #908 but is a distinct federated content-search source-publication boundary.
unknown: []
conflicts: []
first_failure:
  marker: federated-search-revocation-ordering-gap
  evidence: current architecture defines deterministic propagation and bounded stale lag without defining which publication decision wins while propagation is pending, failed or rolled back
rejected_hypotheses:
  - Existing cache generation identity alone proves revocation safety; it does not prove a restrictive source decision advances or fences every still-servable representation.
  - PR #936 already reviewed this root cause; its material reviews addressed reverse dependencies and request-cache identity, not restrictive publication ordering.
  - Issue #908 is a duplicate; it governs native PublicGameData privacy revocation and is terminal, while #938 governs federated content-search publication/index revocation.
  - Current production runtime is leaking revoked federated results; no federated-search runtime/index is delivered yet.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260809-federated-search-revocation-audit.md
  - docs/agents/reports/OTERYN-20260809-federated-search-revocation-audit.md
validation:
  - command: live main / active task / open PR / remediation queue reconciliation
    result: PASS
    evidence: main af3c23943106cd10c7eea42f6644ae12e1e69990; two unrelated blocked active tasks; PRs #541/#338; no pre-existing open remediation finding
  - command: primary ADR 0033 and focused architecture negative-path review
    result: PASS
    evidence: restrictive publication ordering/fencing gap reproduced from the accepted contract itself
  - command: PR #936 review-history duplicate/finding inspection
    result: PASS
    evidence: three P2 repair cycles did not address unpublish/revoke tombstone ordering or rollback fencing
  - command: open/closed Issue and finding-ID duplicate search
    result: PASS
    evidence: no duplicate for federated-search revocation fencing; OPA-SEC-0005 unused
  - command: runtime/browser E2E for audit deliverable
    result: NOT_APPLICABLE
    evidence: audit package changes only non-executable task/report documentation; audited architecture has no delivered federated-search runtime
  - command: exact-head repository CI
    result: NOT_RUN
    evidence: audit PR has not yet been opened
blockers:
  - none
next_action: Open the bounded audit PR, perform exact-head self-review and required CI/review hygiene, then squash-merge and archive the task while registering OPA-SEC-0005 in the continuous-audit historical identity ledger.
```

## Notes

Detailed finding evidence and delivery-layer classification are in `docs/agents/reports/OTERYN-20260809-federated-search-revocation-audit.md`. Remediation belongs exclusively to Issue #938.