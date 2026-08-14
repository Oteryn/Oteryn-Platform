---
task_id: OTERYN-20260814-public-edge-architecture
mode: architecture
task_kind: audit
implementation_authorized: false
issue: 490
status: completed
programme: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
project_lane: oteryn-platform-core
phase: closeout
execution_mode: github_connector
---

# OTERYN-20260814-public-edge-architecture

## Goal

Define one focused, provider-neutral `PublicEdge` architecture and evidence boundary for accepted Oteryn public endpoint, security and production-readiness invariants without performing protected-environment operations or claiming live edge correctness.

## Terminal result

- PR #1064 reached terminal merged state as `c56abdd1a3298d7c5222449fd7c2aa863601eea3`; Issue #1050 later closed `completed` through closeout PR #1066 as `ef156b16286d531b08feb9477b5e0d72f177d5ae`, with the reviewed branch-lifecycle cleanup terminal and approval-free inventories reporting zero deletion candidates.
- PR #1063 synchronized to `main@ef156b16286d531b08feb9477b5e0d72f177d5ae`; exact final head `cca8be2b19928311dd5ae5835ccc6ef79be83b11` was `behind_by=0` and changed exactly four declared architecture/governance paths.
- Both prior material review threads on PR #1063 are resolved and outdated. The only submitted automated review was on superseded head `a6bf23dfef`; no additional owner-funded AI review was invoked for final validation or closeout.
- Exact final head `cca8be2b19928311dd5ae5835ccc6ef79be83b11` passed all eight emitted PR workflows: Platform DB Outage Validation `31839589553`, Native protocol contract `31839589513`, Edge Security Emulation `31839589526`, Phase 7 Production-Like Validation `31839589518`, Native protocol contract audits `31839589580`, Game Auth Ticket Concurrency `31839589535`, Agent Governance `31839589532`, and CI `31839589555`.
- Runtime/browser E2E is `NOT_APPLICABLE`: the delivery is architecture/governance-only and creates no executable user or integration path.
- PR #1063 squash-merged as `780ad6c8178206b13d001537ba651b6e0bd22219`.
- Issue #490 comment `5298101568` records the PublicEdge architecture/applicability/evidence-contract slice terminal while intentionally leaving Issue #490 open for live protected-environment evidence.
- The implementation source branch `docs/OTERYN-20260814-public-edge-architecture` is absent after merge.
- Protected-environment evidence remains independently blocked by `OTERYN-20260801-public-domain-repair`; exact certificate, redirect, WAF/Bot/Access, HSTS/direct-origin and final public acceptance evidence are not claimed by this task.
- No production, Cloudflare/protected-environment, external game/server repository, secret, or additional owner-funded AI operation was performed by this closeout.

## Acceptance criteria

- [x] Current `main`, active tasks, open PR ownership and residual Issue #490 scope were reconciled before mutation.
- [x] `PUBLIC_EDGE_ARCHITECTURE.md` owns DNS/proxy, TLS, redirect/HSTS, edge abuse/admin controls, tunnel/origin ingress and direct-origin evidence semantics without taking application-security authority.
- [x] Canonical WWW/Gateway host/service mapping remains owned by `PUBLIC_ENDPOINTS_CONTRACT.md`; Cloudflare-specific material remains subordinate implementation/evidence.
- [x] Repository, staging and production evidence remain separated; no DNS/Tunnel/repository result promotes itself to `PRODUCTION_PROVEN`.
- [x] TLS failure, HTTP challenge/403, redirect behavior, HSTS state and direct-origin exposure remain distinct fail-closed observations.
- [x] Positively observed direct-origin bypass is preserved as a failing/noncompliant exposure rather than collapsed back to `UNKNOWN`.
- [x] `ARCHITECTURE_AUTHORITY.md` routes PublicEdge to the focused owner.
- [x] Architecture-review programme records the package and protected-environment handoff.
- [x] Portal work allocation keeps live PublicEdge proof blocked while repository-safe preparation remains executable.
- [x] ADR allocation is `NOT_APPLICABLE`: no durable hostname, application-security or go-live policy changed.
- [x] Exact final-head CI passed after review repairs and repository-governance dependency closeout.
- [x] Runtime/browser E2E is `NOT_APPLICABLE` for the architecture/governance-only package.
- [x] PR merge, Issue #490 residual reconciliation, task archival and ownership release are terminal through this lifecycle-only closeout carrier once merged.

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: same-repository architecture PR #1063 merged only after exact-head gates passed
source_branch_evidence: refs/heads/docs/OTERYN-20260814-public-edge-architecture is absent after squash merge 780ad6c8178206b13d001537ba651b6e0bd22219
```

## Closeout carrier

This archive copy is proposed terminal state on the lifecycle-only closeout branch `closeout/OTERYN-20260814-public-edge-architecture`. Protected `main` remains canonical until the closeout PR merges. The closeout carrier itself must also be removed by repository `delete_branch_on_merge=true` and verified after merge; that post-merge fact cannot truthfully be embedded in this pre-merge commit.

## Post-merge governance observation

`main@780ad6c8178206b13d001537ba651b6e0bd22219` emitted Agent Governance run `31839691822`; all static validator, checkpoint and terminal source-branch-closeout steps passed, while live-aware Control Room/liveness enforcement failed with the merged PublicEdge task still present under `docs/agents/tasks/active/`. This closeout PR removes that stale active ownership and returns the programme to `ready / next-risk-based-rotation`; its exact-head Agent Governance result is required before merge.
