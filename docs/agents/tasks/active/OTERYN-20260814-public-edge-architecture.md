---
task_id: OTERYN-20260814-public-edge-architecture
mode: architecture
task_kind: audit
implementation_authorized: false
issue: 490
status: investigating
programme: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
project_lane: oteryn-platform-core
phase: design
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
- [ ] A focused `PUBLIC_EDGE_ARCHITECTURE.md` owns DNS/proxy, TLS, redirect/HSTS, edge abuse controls, administrative edge access, tunnel/origin ingress and direct-origin evidence semantics without taking application-security authority.
- [ ] The focused architecture preserves the canonical public hostname/service mapping from `PUBLIC_ENDPOINTS_CONTRACT.md` and treats provider-specific Cloudflare material as implementation/operations evidence rather than durable provider lock-in.
- [ ] Repository, staging, protected-environment and production evidence remain explicitly separated; no DNS/Tunnel or repository result may imply `PRODUCTION_PROVEN`.
- [ ] TLS failure, HTTP challenge/403, redirect behavior, HSTS state and direct-origin exposure have distinct fail-closed evidence semantics.
- [ ] `ARCHITECTURE_AUTHORITY.md` routes PublicEdge to the focused owner.
- [ ] The architecture-review programme records the bounded active package and its residual protected-environment handoff.
- [ ] Portal work allocation distinguishes architecture readiness from live PublicEdge proof blocked on protected-environment authority/evidence.
- [x] ADR allocation is `NOT_APPLICABLE`: this package does not change ADR 0020 hostname policy, security policy or production go-live policy; it consolidates an already-declared PublicEdge ownership boundary.
- [ ] Exact final-head self-review, documentation/governance CI and PR hygiene pass with no material finding.
- [x] Runtime/browser E2E is `NOT_APPLICABLE` because this task changes architecture/governance documentation only and creates no executable route, deployment or edge mutation.
- [ ] PR merge, Issue #490 residual-scope reconciliation, task archival and ownership release are terminal.

## Ownership

```yaml
owned_paths:
  - docs/architecture/PUBLIC_EDGE_ARCHITECTURE.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
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
updated_at: 2026-08-14T19:12:50+02:00
head: 50cec78777c6ce3f5ddb4cc2ff17499a90164463
material_head: 50cec78777c6ce3f5ddb4cc2ff17499a90164463
branch: docs/OTERYN-20260814-public-edge-architecture
pr: none
status: investigating
phase: design
context_routes:
  - architecture
  - security
  - deployment-operations
  - agent-governance
owned_paths:
  - docs/architecture/PUBLIC_EDGE_ARCHITECTURE.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/agents/tasks/active/OTERYN-20260814-public-edge-architecture.md
proven:
  - trusted main at task selection is 50cec78777c6ce3f5ddb4cc2ff17499a90164463
  - active public-domain-repair is blocked by external Cloudflare token rotation and remains the live evidence/repair record
  - active native-auth-production-verification is verification-only and blocked; this task does not touch it
  - no open PR owns the declared PublicEdge architecture paths
  - SYSTEM_ARCHITECTURE explicitly declares PublicEdge ownership but ARCHITECTURE_AUTHORITY has no focused PublicEdge canonical route
  - Issue #490 remains the residual shared owner for PublicEdge protected-environment proof
  - no server/game repository was accessed
  - no protected environment, live Cloudflare state, production secret or owner-funded AI service was accessed
  - ADR allocation is not applicable because no durable hostname/security/go-live policy is changed
derived:
  - the smallest safe architecture package is a provider-neutral PublicEdge focused document plus authority/programme routing
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
  - docs/agents/tasks/active/OTERYN-20260814-public-edge-architecture.md
validation:
  - command: live main/task/PR/Issue overlap reconciliation
    result: PASS
    evidence: exact main, active task directory, open PR path ownership and Issue #490 were directly inspected
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: architecture/governance-only package creates no executable runtime or user path
  - command: exact-head documentation/governance CI
    result: NOT_RUN
    evidence: architecture package not yet complete
blockers:
  - none for architecture documentation
next_action: Add the focused PublicEdge architecture and canonical routing, then open the documentation PR and validate its exact final head.
```

## Notes

The current Cloudflare token limitation is intentionally not bypassed. This architecture task may define evidence requirements but cannot inspect, mutate or promote protected-environment state.