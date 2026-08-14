---
task_id: OTERYN-20260814-public-edge-architecture
mode: architecture
task_kind: audit
implementation_authorized: false
issue: 490
status: validating
programme: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
project_lane: oteryn-platform-core
phase: validate
execution_mode: github_connector
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/SYSTEM_ARCHITECTURE.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - docs/operations/PRODUCTION_READINESS_CHECKLIST.md
  - docs/operations/PRODUCTION_TOPOLOGY_EVIDENCE.md
  - docs/operations/CLOUDFLARE_EDGE_AUDIT.md
search_first:
  - Issue #490 PublicEdge residual finding
  - active public-domain repair task
  - open PublicEdge/Cloudflare architecture PRs
optional_reads:
  - deploy/synology/PUBLIC_ENDPOINTS.md
---

# OTERYN-20260814-public-edge-architecture

## Goal

Define one focused, provider-neutral `PublicEdge` architecture and evidence boundary for the already accepted Oteryn public endpoint, security and production-readiness invariants, without performing Cloudflare/protected-environment operations or claiming live edge correctness.

## Acceptance criteria

- [x] Current `main`, active tasks, open PR ownership and residual Issue #490 PublicEdge scope are reconciled before mutation.
- [x] A focused `PUBLIC_EDGE_ARCHITECTURE.md` owns DNS/proxy, TLS, redirect/HSTS, edge abuse controls, administrative edge access, tunnel/origin ingress and direct-origin evidence semantics without taking application-security authority.
- [x] The focused architecture preserves the canonical public hostname/service mapping from `PUBLIC_ENDPOINTS_CONTRACT.md` and treats provider-specific Cloudflare material as implementation/operations evidence rather than durable provider lock-in.
- [x] Repository, staging, protected-environment and production evidence remain explicitly separated; no DNS/Tunnel or repository result may imply `PRODUCTION_PROVEN`.
- [x] TLS failure, HTTP challenge/403, redirect behavior, HSTS state and direct-origin exposure have distinct fail-closed evidence semantics.
- [x] `ARCHITECTURE_AUTHORITY.md` routes PublicEdge to the focused owner.
- [x] The architecture-review programme records the bounded active package and its residual protected-environment handoff.
- [x] Portal work allocation was reviewed and already keeps live PublicEdge proof `BLOCKED` while stating repository-safe preparation is not blocked; no semantic edit is required for this architecture package.
- [x] ADR allocation is `NOT_APPLICABLE`: this package does not change ADR 0020 hostname policy, security policy or production go-live policy; it consolidates an already-declared PublicEdge ownership boundary.
- [ ] Exact final-head documentation/governance CI passes and PR review hygiene has no material finding.
- [x] Runtime/browser E2E is `NOT_APPLICABLE` because this task changes architecture/governance documentation only and creates no executable route, deployment or edge mutation.
- [ ] PR merge, Issue #490 residual-scope reconciliation, task archival and ownership release are terminal.

## Ownership

```yaml
owned_paths:
  - docs/architecture/PUBLIC_EDGE_ARCHITECTURE.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/tasks/active/OTERYN-20260814-public-edge-architecture.md
modules:
  - PublicEdge
dependencies:
  - Issue #490 shared audit owner
  - ADR 0020 canonical Gateway hostname decision
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/operations/PRODUCTION_READINESS_CHECKLIST.md
  - docs/operations/PRODUCTION_TOPOLOGY_EVIDENCE.md
  - OTERYN-20260801-public-domain-repair for live Cloudflare/public acceptance evidence
blockers:
  - none for the architecture package
cross_repository_tasks:
  - none
```

## Context pressure

```yaml
policy_version: 2
scope_breadth: 1
evidence_volume: 2
history_dependency: 2
iteration_uncertainty: 1
parallel_hypotheses: 1
context_score: 7
context_pressure: medium
context_growth: stable
estimate_confidence: high
decomposition_decision: single
decomposition_reason: one documentation-only authority/evidence boundary with a single shared Issue and no runtime ownership
```

## Current-state finding

```yaml
classification: documentation_drift
severity: medium
confidence: high
current_state:
  PROVEN:
    - SYSTEM_ARCHITECTURE declares PublicEdge an explicit cross-cutting ownership boundary and says it owns expected DNS, TLS, redirect, HSTS, WAF, tunnel/origin and private-ingress contract/evidence.
    - SECURITY_ARCHITECTURE defines defense-in-depth and keeps Laravel authentication, authorization, MFA, CSRF and application rate limiting authoritative even when edge controls exist.
    - PUBLIC_ENDPOINTS_CONTRACT defines the canonical WWW and Gateway host/service/origin mapping and retires login.oteryn.molehill.cloud.
    - PRODUCTION_READINESS_CHECKLIST and PRODUCTION_TOPOLOGY_EVIDENCE keep live DNS/TLS/WAF/Access/origin evidence fail-closed behind direct environment proof.
    - Issue #490 remains open for PublicEdge protected-environment proof after the OperationsObservability and PlatformAPI architecture slices became terminal.
    - active public-domain-repair is blocked on replacement of the protected Cloudflare token with minimum required remaining-edge read scopes.
  DERIVED:
    - PublicEdge has enough accepted policy and evidence semantics for a focused canonical architecture document without choosing a new product/provider policy.
    - The lack of a focused PublicEdge row in ARCHITECTURE_AUTHORITY leaves a declared system ownership boundary distributed across system, security, contract and operations documents.
  UNKNOWN:
    - current protected-environment certificate, WAF/Bot/Access/redirect/HSTS and direct-origin state beyond already recorded bounded evidence
    - exact future provider if Cloudflare is ever replaced
  CONFLICT: []
```

## Alternatives and recommendation

| Option | Security/correctness | Operability | Coupling | Reversibility | Result |
|---|---|---|---|---|---|
| A. Focused provider-neutral PublicEdge architecture, provider-specific operations beneath it | Strong: one fail-closed evidence contract while preserving app security | Strong: separates expected controls from live proof | Low provider coupling | High | **Recommended** |
| B. Keep PublicEdge semantics distributed across system/security/contracts/operations | Correct pieces exist, but drift/authority ambiguity persists | Medium/low | Low | High | Rejected: leaves the proven ownership gap unresolved |
| C. Make Cloudflare operations docs the architecture authority | Strong for current provider details but mixes architecture with credential/tooling state | Medium | High Cloudflare coupling | Medium | Rejected: provider implementation evidence must not become durable product/security authority |

Recommendation confidence: `high`.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-14T19:16:49+02:00
head: c3fd0fb3c9121b0706d2e7597c8ebef56ba843ea
material_head: c3fd0fb3c9121b0706d2e7597c8ebef56ba843ea
branch: docs/OTERYN-20260814-public-edge-architecture
pr: 1063
status: validating
phase: validate
invocation_started_at: 2026-08-14T19:09:00+02:00
last_progress_at: 2026-08-14T19:16:49+02:00
ci_checks_for_current_head: 1
ci_check_generation: draft
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 0
stall_warnings: 0
context_routes:
  - architecture
  - security
  - deployment-operations
  - agent-governance
owned_paths:
  - docs/architecture/PUBLIC_EDGE_ARCHITECTURE.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/tasks/active/OTERYN-20260814-public-edge-architecture.md
proven:
  - trusted main at task selection is 50cec78777c6ce3f5ddb4cc2ff17499a90164463
  - active public-domain-repair is blocked by external Cloudflare token rotation and remains the live evidence/repair record
  - active native-auth-production-verification is verification-only and blocked; this task does not touch it
  - no open PR owns the declared PublicEdge architecture paths
  - SYSTEM_ARCHITECTURE explicitly declares PublicEdge ownership but main ARCHITECTURE_AUTHORITY had no focused PublicEdge canonical route
  - Issue #490 remains the residual shared owner for PublicEdge protected-environment proof
  - PR #1063 adds a provider-neutral PublicEdge focused architecture and authority route without runtime/protected-environment changes
  - current Portal Completion work allocation already states PublicEdge live proof is blocked and repository-safe preparation is not blocked
  - no server/game repository was accessed
  - no protected environment, live Cloudflare state, production secret or owner-funded AI service was accessed
  - ADR allocation is not applicable because no durable hostname/security/go-live policy is changed
derived:
  - the smallest safe architecture package is the focused PublicEdge document plus authority/programme routing
unknown:
  - exact current remaining Cloudflare control state blocked by the active task token permission boundary
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - active public-domain-repair can continue autonomously with the current protected token
  - OperationsObservability owns live PublicEdge controls
  - Cloudflare-specific audit documentation should become the durable architecture authority
changed_paths:
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/tasks/active/OTERYN-20260814-public-edge-architecture.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/PUBLIC_EDGE_ARCHITECTURE.md
validation:
  - command: live main/task/PR/Issue overlap reconciliation
    result: PASS
    evidence: exact main, active task directory, open PR path ownership and Issue #490 were directly inspected
  - command: architecture full-diff and negative-path self-review
    result: PASS
    evidence: provider-neutral ownership, application-security non-authority, evidence non-promotion, TLS/HTTP/HSTS/challenge/origin ambiguity and protected-token failure paths were reviewed with no material finding
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: architecture/governance-only package creates no executable runtime or user path
  - command: exact-head documentation/governance CI
    result: NOT_RUN
    evidence: final checkpoint commit must emit and pass exact-head workflows before merge
blockers: []
next_action: Validate required workflows and review hygiene on the final PR #1063 head; merge only if every gate passes.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 1
  session_id: 20260814T190900+0200-public-edge-architecture
  session_started_at: 2026-08-14T19:09:00+02:00
  checkpointed_at: 2026-08-14T19:16:49+02:00
  last_progress_at: 2026-08-14T19:16:49+02:00
  phase: final-validation-and-merge
  exact_head: c3fd0fb3c9121b0706d2e7597c8ebef56ba843ea
  pull_request: 1063
  active_operation: none
  external_run_ids: []
  operation_started_at: null
  wait_deadline_at: null
  check_generation: final-checkpoint
  checks_used: 0
  status: ready
  safe_to_resume: true
  resume_condition: GitHub exposes required workflows for the final checkpoint commit.
  next_action: Fetch PR #1063 final head and aggregate required workflow/review state; merge only after every exact-head gate passes.
```

## Self-review

```yaml
self_review:
  result: PASS
  exact_head: c3fd0fb3c9121b0706d2e7597c8ebef56ba843ea
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
  evidence:
    - PR #1063 changes only four declared architecture/governance paths
    - current public-domain-repair ownership is preserved and its protected-token blocker is not bypassed
    - application auth/security, production activation and server/game repository authority remain explicitly outside PublicEdge
    - provider-specific Cloudflare tooling remains subordinate evidence rather than canonical provider lock-in
    - no runtime, workflow, persistence, deployment or protected-environment behavior changes
```

## Notes

The current Cloudflare token limitation is intentionally not bypassed. This architecture task defines evidence requirements but does not inspect, mutate or promote protected-environment state.