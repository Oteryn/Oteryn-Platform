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

Independently audit the newly accepted WWW Platform federated-search architecture for publication/visibility revocation correctness, especially source authority versus stale derived indexes, result caches, tombstone propagation and generation rollback. Record confirmed findings without implementing remediation.

## Acceptance criteria

- [x] Refresh protected `main`, active tasks, open PRs and live remediation queue before domain selection.
- [x] Confirm historical findings #905 and #908 are terminal and not live owners.
- [x] Audit current ADR 0033 and `FEDERATED_SEARCH_ARCHITECTURE.md` from primary evidence.
- [x] Inspect PR #936 review history for already-discovered/repaired findings.
- [x] Falsify the revoke/unpublish path against stale index/cache and rollback semantics.
- [x] Deduplicate against open/closed Issues before creating a finding.
- [x] Route OPA-SEC-0005 as independent Issue #938 with current taxonomy metadata.
- [x] Keep Issue #938 remediation paths out of the audit diff.
- [x] Open bounded audit PR #939.
- [ ] Complete exact-final-head self-review, fresh review, required CI, zero unresolved material threads, merge and lifecycle closeout.

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

Issue #938 exclusively owns the architecture repair. This audit does not remediate that finding.

## Audit result

One material finding is proven:

- **OPA-SEC-0005 / Issue #938 — HIGH / P1**: ADR 0033/focused search architecture require deterministic unpublish/revoke/delete propagation and permit bounded stale-index/cache behavior, but do not define a monotonic restrictive publication-decision fence, authoritative visibility cutoff, fail-closed propagation-failure semantics, or rollback rule preventing an older index generation from resurrecting a result after a newer restrictive decision.

This is a future architecture-contract risk. No current federated-search runtime/index/cache disclosure is claimed.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-09T08:24:00Z
invocation_started_at: 2026-08-09T08:12:00Z
last_progress_at: 2026-08-09T08:24:00Z
head: OUT_OF_BAND_FINAL_HEAD_AFTER_THIS_CHECKPOINT
branch: audit/OTERYN-20260809-federated-search-revocation
pr: 939
status: validating
phase: exact-head-validation
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
ci_check_generation: ready
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 0
stall_warnings: 0
proven:
  - Protected main at audit selection was af3c23943106cd10c7eea42f6644ae12e1e69990.
  - Active tasks at selection were only the unrelated blocked public-domain repair and native-auth production-verification records.
  - Open PRs #541 and #338 are unrelated independent holds.
  - Live programme:audit-repair queue was empty before this finding was created.
  - Issues #905 and #908 are closed completed after independent repairs and closeouts.
  - Issue #935 is terminal after PR #936 accepted ADR 0033 and the focused federated-search architecture.
  - PR #936 review history shows three material repairs for reverse dependency and cache request identity; no publication-revocation ordering/tombstone rollback finding was recorded.
  - The focused architecture requires revoked/unpublished/incompatible content to stop appearing according to canonical source truth while also permitting bounded stale-index lag, tombstones, index generations and result caching.
  - Neither ADR 0033 nor the focused architecture defines a restrictive publication-decision revision/watermark, affected-result fail-closed propagation rule, or rollback fence across a newer revoke.
  - Duplicate searches found no existing Issue owning the same root cause and no prior OPA-SEC-0005 identity.
  - Issue #938 is the independent remediation owner and was published `agent:ready` after confirming deterministic branch `repair/issue-938` did not exist.
  - PR #939 changes exactly the two declared audit documentation paths.
derived:
  - Ordinary bounded stale-index/cache freshness can coexist with a newer restrictive source decision unless a separate ordered publication-authority fence is defined.
  - Historical Issue #908 has a structurally similar hazard but a distinct PublicGameData privacy contract and is not a duplicate.
unknown: []
conflicts: []
first_failure:
  marker: federated-search-revocation-ordering-gap
  evidence: accepted contract defines desired revocation propagation and stale tolerance without defining which publication decision wins while propagation is delayed, failed or rolled back
rejected_hypotheses:
  - Provider/source/index generation in cache identity alone proves revocation safety; it does not prove that a restrictive source decision fences every still-servable older representation.
  - PR #936 already repaired this root cause; its three material review cycles did not address restrictive publication ordering or rollback after revoke.
  - Issue #908 is duplicate ownership; it is terminal and governs native PublicGameData privacy revocation, not federated content publication/index revocation.
  - Current production federated search is leaking revoked data; no such runtime/index is delivered by ADR 0033.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260809-federated-search-revocation-audit.md
  - docs/agents/reports/OTERYN-20260809-federated-search-revocation-audit.md
validation:
  - command: live main / active task / open PR / remediation queue reconciliation
    result: PASS
    evidence: non-overlapping domain selected from main af3c23943106cd10c7eea42f6644ae12e1e69990
  - command: ADR 0033 and focused architecture negative-path review
    result: PASS
    evidence: restrictive publication ordering/fencing gap is reproducible from the accepted contract
  - command: PR #936 review-history inspection
    result: PASS
    evidence: three material review repairs do not cover this root cause
  - command: open/closed Issue plus OPA-SEC-0005 duplicate search
    result: PASS
    evidence: no duplicate; Issue #938 created as independent owner
  - command: PR #939 changed-path inspection
    result: PASS
    evidence: exactly two audit documentation paths; no remediation/product path
  - command: runtime/browser E2E for audit deliverable
    result: NOT_APPLICABLE
    evidence: non-executable audit documentation only; audited federated-search runtime is not implemented
  - command: exact-final-head self-review / fresh review / repository CI
    result: NOT_RUN
    evidence: checkpoint commit creates the final validation generation
blockers:
  - none
next_action: Validate the unchanged final PR #939 head with exact-head self-review, fresh review, required Agent Governance/CI and zero unresolved threads; merge only if all gates pass, then perform required lifecycle archive/programme reconciliation.
```

## Notes

Detailed finding evidence and delivery-layer classification are in `docs/agents/reports/OTERYN-20260809-federated-search-revocation-audit.md`. Remediation belongs exclusively to Issue #938.
