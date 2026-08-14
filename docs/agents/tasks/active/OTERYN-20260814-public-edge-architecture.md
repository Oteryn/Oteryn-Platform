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

Define one focused, provider-neutral `PublicEdge` architecture and evidence boundary for the accepted Oteryn public endpoint, security and production-readiness invariants, without performing protected-environment operations or claiming live edge correctness.

## Acceptance criteria

- [x] Current `main`, active tasks, open PR ownership and residual Issue #490 scope were reconciled before mutation.
- [x] `PUBLIC_EDGE_ARCHITECTURE.md` owns DNS/proxy, TLS, redirect/HSTS, edge abuse/admin controls, tunnel/origin ingress and direct-origin evidence semantics without taking application-security authority.
- [x] Canonical WWW/Gateway host/service mapping remains owned by `PUBLIC_ENDPOINTS_CONTRACT.md`; Cloudflare-specific material remains subordinate implementation/evidence.
- [x] Repository, staging and production evidence remain separated; no DNS/Tunnel/repository result promotes itself to `PRODUCTION_PROVEN`.
- [x] TLS failure, HTTP challenge/403, redirect behavior, HSTS state and direct-origin exposure remain distinct fail-closed observations.
- [x] Positively observed direct-origin bypass is preserved as a failing/noncompliant exposure rather than collapsed back to `UNKNOWN`.
- [x] `ARCHITECTURE_AUTHORITY.md` routes PublicEdge to the focused owner.
- [x] Architecture-review programme records the package and protected-environment handoff.
- [x] Portal work allocation already keeps live PublicEdge proof `BLOCKED` while repository-safe preparation remains executable; no edit was required.
- [x] ADR allocation is `NOT_APPLICABLE`: no durable hostname, application-security or go-live policy changed.
- [ ] Exact final-head CI passes after the P1/P2 review repairs and PR review hygiene is terminal.
- [x] Runtime/browser E2E is `NOT_APPLICABLE`: this architecture/governance-only package creates no executable route, deployment or user path.
- [ ] PR merge, Issue #490 residual reconciliation, task archival and ownership release are terminal.

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
  - none for repository architecture work
cross_repository_tasks:
  - none
```

## Architecture finding

```yaml
classification: documentation_drift
severity: medium
confidence: high
PROVEN:
  - SYSTEM_ARCHITECTURE declares PublicEdge as a cross-cutting ownership boundary.
  - SECURITY_ARCHITECTURE keeps application auth/authz/MFA/CSRF/rate limits authoritative despite edge defense in depth.
  - PUBLIC_ENDPOINTS_CONTRACT owns the canonical WWW/Gateway mapping and retires login.oteryn.molehill.cloud.
  - Production-readiness evidence remains fail-closed behind direct environment proof.
  - OTERYN-20260801-public-domain-repair remains blocked on external replacement of the protected Cloudflare token with sufficient remaining-edge read scopes.
DERIVED:
  - A provider-neutral focused PublicEdge owner closes the architecture-authority gap without changing ADR 0020 or production activation policy.
UNKNOWN:
  - Exact current protected-environment certificate, redirect, WAF/Bot/Access, HSTS and direct-origin disposition beyond already recorded bounded evidence.
CONFLICT: []
```

## Review repairs

- **P1 recovery/head evidence:** immutable embedded SHA pointers were stale after later checkpoint commits. Recovery now deliberately uses the live PR head as source of truth. `material_head` records only the last material architecture commit; it is not a merge target. Any continuation must resolve PR #1063 live `head_sha` before validation or merge and must not validate an embedded historical SHA as the current head.
- **P2 direct-origin semantics:** confirmed direct-origin reachability is now an observed failing/noncompliant exposure. `UNKNOWN` is reserved for absent/insufficient evidence; a positive bypass observation keeps the applicable go-live gate blocked unless a separately authorized accepted-risk decision covers it.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-14T19:33:00+02:00
head: LIVE_PR_1063_HEAD
material_head: 4210c36277c9115facbcbe0ca06b320a23536356
branch: docs/OTERYN-20260814-public-edge-architecture
pr: 1063
status: validating
phase: validate
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
  - trusted main at task selection was 50cec78777c6ce3f5ddb4cc2ff17499a90164463
  - PR #1063 is the live task PR; continuation must resolve its current head_sha rather than treat an embedded SHA as current
  - material architecture repair commit 4210c36277c9115facbcbe0ca06b320a23536356 preserves positively observed direct-origin exposure as a failure
  - superseded head a6bf23dfefbf9938c472b3456b38f404abbc293e passed all eight emitted workflows before review repair but is not final validation evidence
  - repository-configured ready-for-review automation emitted a Codex review on a6bf23dfefbf9938c472b3456b38f404abbc293e; this task did not manually request or invoke Codex/OpenAI/API review and whether that automatic integration consumes owner-funded quota is UNKNOWN
  - that automated review raised P1 stale recovery evidence and P2 direct-origin classification; both root causes are repaired in the current branch and no additional Codex review will be manually requested
  - active public-domain-repair remains the live protected-environment evidence/repair owner and its blocker is not bypassed
  - no server/game repository, protected environment or production secret was accessed by this task
derived:
  - after review repair the remaining repository gate is exact current live-head CI plus terminal review hygiene
unknown:
  - exact current protected-environment edge-control state remains blocked by the active task token permission boundary
  - whether the repository-configured automatic Codex review consumes the repository owner's quota
conflicts: []
first_failure:
  marker: PR #1063 automated review P1/P2 on a6bf23dfefbf9938c472b3456b38f404abbc293e
  evidence: stale embedded recovery SHA could misroute continuation; direct-origin positive exposure was incorrectly collapsible to UNKNOWN
rejected_hypotheses:
  - a historical embedded checkpoint SHA is safe to use as the current PR head after later task commits
  - a positively observed direct-origin bypass should return to UNKNOWN when no accepted-risk decision exists
  - current protected Cloudflare token permissions can be bypassed from this architecture task
changed_paths:
  - docs/agents/programs/OTERYN_PLATFORM_ARCHITECTURE_REVIEW.md
  - docs/agents/tasks/active/OTERYN-20260814-public-edge-architecture.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/PUBLIC_EDGE_ARCHITECTURE.md
validation:
  - command: live main/task/PR/Issue overlap reconciliation
    result: PASS
    evidence: main, active tasks, open path ownership and Issue #490 were directly inspected before mutation
  - command: architecture negative-path audit after P2 repair
    result: PASS
    evidence: UNKNOWN is now evidence absence only; confirmed origin bypass remains an explicit failing/noncompliant observation
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: architecture/governance-only package creates no executable runtime or user path
  - command: exact current live-head GitHub Actions
    result: NOT_RUN
    evidence: review repair changed the head; prior eight-workflow success belongs only to superseded a6bf23dfefbf9938c472b3456b38f404abbc293e
blockers: []
next_action: Resolve PR #1063 live head_sha, verify both review repairs on that exact diff, then require all emitted workflows and review threads to pass before squash merge.
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 2
  session_id: 20260814T190900+0200-public-edge-architecture
  phase: review-repair-validation
  exact_head: LIVE_PR_1063_HEAD
  pull_request: 1063
  material_head: 4210c36277c9115facbcbe0ca06b320a23536356
  source_of_truth: GitHub PR #1063 live head_sha
  active_operation: none
  status: ready
  safe_to_resume: true
  resume_condition: Resolve the live PR head first; never substitute the embedded material_head for current head validation.
  next_action: Fetch PR #1063, audit the P1/P2 repairs on its live head, then aggregate exact-head CI and review state.
```

## Self-review

```yaml
self_review:
  result: PASS
  exact_head: LIVE_PR_1063_HEAD
  material_head: 4210c36277c9115facbcbe0ca06b320a23536356
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
  evidence:
    - live PR head must be resolved before using this review record
    - provider-neutral ownership does not take application auth/security or production activation authority
    - positive direct-origin exposure remains a preserved failure, while evidence absence remains UNKNOWN
    - current public-domain-repair ownership and protected-token blocker remain untouched
    - no runtime, workflow, persistence, deployment or protected-environment behavior changes
```

## Notes

The Cloudflare token limitation is intentionally not bypassed. This task defines evidence requirements only; it does not inspect, mutate or promote protected-environment state.