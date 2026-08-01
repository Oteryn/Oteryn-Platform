---
task_id: OTERYN-20260801-production-go-live-gate
project_lane: oteryn-platform-core
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/operations/PRODUCTION_READINESS_CHECKLIST.md
  - docs/testing/PRODUCTION_SMOKE_CHECKLIST.md
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - deploy/synology/PUBLIC_ENDPOINTS.md
  - docs/agents/ACTIVE_WORK.md
search_first:
  - docs/agents/tasks/active/**
  - open pull requests touching production, Synology, public edge, or Issue #91
optional_reads: []
---

# OTERYN-20260801-production-go-live-gate

## Goal

Execute Issue #91's fail-closed Production Go-Live Gate against the exact deployed Oteryn Platform and Game Gateway release, recording only sanitized non-secret direct production evidence and ending as either `PRODUCTION_PROVEN` or `BLOCKED — PENDING PRODUCTION VERIFICATION`.

## Acceptance criteria

- [ ] Exact deployed Platform commit/image identity and Game Gateway image identity are directly proven.
- [ ] Effective Synology container, port, network, health, restart, origin and cloudflared topology is directly proven read-only.
- [ ] Both canonical public endpoints receive independent DNS/TLS/HTTP/security/routing verification.
- [ ] Every launch-applicable item in the production readiness and smoke checklists has direct non-secret production evidence.
- [ ] Required CI is green on the exact evaluated release.
- [ ] Final verdict is recorded on Issue #91 without promoting repository or staging evidence to production proof.
- [ ] No production mutation is performed without the separately required explicit owner authorization and rollback plan.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260801-production-go-live-gate.md
  - docs/agents/evidence/OTERYN-20260801-production-go-live-gate/**
modules:
  - production-operations
  - deployment-validation
  - public-edge-validation
dependencies:
  - issue-91
  - PR-387-public-domain-audit-read-only-reference
  - PR-335-synology-restart-policy-read-only-reference
blockers:
  - canonical WWW returns Cloudflare 403 instead of Platform
  - canonical Game Gateway fails TLS before HTTP
  - exact deployed Platform and Gateway identities remain unproven
  - mandatory production backup restore rollback mail observability and smoke evidence remains absent
cross_repository_tasks:
  - Canary and login-server remain read-only unless separately authorized
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
task_kind: validation
implementation_authorized: false
production_mutation_authorized: false
phase: validate
session_id: agent-20260801-production-gate-001
session_role: investigator
execution_mode: chat
execution_reason: connector-backed repository evidence review and bounded read-only production probes
lease_expires_at: 2026-08-01T13:38:03.110Z
context_pressure: high
context_growth: stable
context_score: 12
estimate_confidence: high
decomposition_decision: phased
decomposition_reason: deployment identity, Synology origin, public edge, readiness, and smoke are sequential gates sharing one release verdict
validation_level: focused
last_completed_step: blocked gate evidence persisted and temporary observer files removed
session_rotation_count: 0
heavy_validation_runs: 1
stale_takeover_count: 0
human_interruptions: 0
updated_at: 2026-08-01T13:38:03.110Z
head: 9d842d0fe89f9184aa96f3a43cbe7dc704cbfcb4
branch: agent/production-go-live-gate
pr: 405
status: blocked
context_routes:
  - agent-governance
  - testing
  - security
  - auth-identity
  - canary-integration
  - database
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260801-production-go-live-gate.md
  - docs/agents/evidence/OTERYN-20260801-production-go-live-gate/**
proven:
  - Issue #91 remains open and requires direct production evidence for every mandatory launch-applicable item.
  - Cloudflare fixed-scope Tunnel and DNS reconciliation passed in runs 30699270139 and 30700054602.
  - Independent public observation run 30701140509 artifact 8818850803 directly observed WWW Cloudflare 403 responses, Game Gateway TLS failure, HTTP 403 without HTTPS redirect, and HSTS max-age=0.
  - Trusted-main read-only Synology preflight run 30701440189 reached runner oteryn-synology-staging with restore disabled and failed at the first invariant because MariaDB does not use restart policy unless-stopped.
  - PR #405 head 0c435dd02d2afcc7f0e8d963a79b5441b29a6cb7 passed Agent Governance, CI, static Synology preflight, Edge Security Emulation, Game Auth Ticket Concurrency, Platform DB Outage, Phase 7 and Synology image-build workflows.
  - No Cloudflare, DNS, Synology runtime, database, Redis, secret, deployment, rollback, restore or application-data mutation was performed.
derived:
  - Cloudflare Tunnel and DNS convergence did not establish usable public application delivery.
  - Canonical public endpoint failures independently block launch and prevent application-level production smoke.
  - Mutation smoke is unsafe while release identity, rollback, dated restore evidence, controlled identities and public application reachability remain unproven.
  - The only correct verdict is BLOCKED — PENDING PRODUCTION VERIFICATION.
unknown:
  - exact deployed Platform source SHA, tag, image ID and repository digest
  - exact deployed Game Gateway source SHA, tag, image ID and repository digest
  - exact deployed Canary identity for the selected launch scope
  - actual MariaDB restart policy and full container restart health port network and deployment timestamp inventory
  - whether cloudflared is a host process or container and its effective network path to loopback origins
  - effective production configuration verifier result
  - production DB grants backup policy dated restore evidence and rollback mechanism
  - production Redis session cache queue mail logging metrics alerts and on-call state
  - launch-scope decisions controlled smoke identities and all mutation smoke results
conflicts:
  - archived Synology preflight requires restart policy unless-stopped while current live MariaDB fails that invariant; open PR #335 proposes always but does not prove the effective running value
first_failure:
  marker: public-edge-not-application-reachable
  evidence: run 30701140509 artifact 8818850803 observed WWW 403 challenge responses and Game Gateway TLS negotiation failure after Cloudflare Tunnel/DNS convergence
rejected_hypotheses:
  - Cloudflare configuration convergence proves production readiness: run 30700054602 verifies only managed Tunnel and DNS scope
  - HTTP 200 or DNS resolution alone proves correct routing: direct public checks did not reach either expected application service
  - open PR #335 proves the running restart policy: proposed configuration is not runtime evidence
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260801-production-go-live-gate.md
  - docs/agents/evidence/OTERYN-20260801-production-go-live-gate/index.md
validation:
  - command: repository and Issue #91 preflight through GitHub connector
    result: PASS
    evidence: mandatory documents Issue #91 comments active-work index and open PR inventory inspected on 2026-08-01
  - command: Public Edge Post-Cloudflare run 30701140509
    result: FAIL
    evidence: artifact 8818850803 digest sha256:787ea72c616812ade431eb1cc396e921a6c8b04e459c89557221cbf6caebe656
  - command: trusted-main Synology Production Target Preflight run 30701440189
    result: FAIL
    evidence: job 91373030006 stopped on MariaDB restart-policy mismatch with restore disabled
  - command: PR #405 repository workflows on head 0c435dd02d2afcc7f0e8d963a79b5441b29a6cb7
    result: PASS
    evidence: runs 30701773251 30701773237 30701773212 30701773227 30701773203 30701773233 30701773198 and 30701773188
  - command: final production readiness and mutation smoke
    result: BLOCKED
    evidence: exact deployed release and mandatory backup rollback edge mail observability and controlled-smoke prerequisites remain unproven
blockers:
  - canonical WWW returns Cloudflare 403 challenge content instead of Oteryn Platform
  - canonical Game Gateway fails TLS before HTTP
  - plain HTTP does not redirect to HTTPS and HSTS remains max-age=0
  - exact running release identity and full Synology cloudflared topology remain unproven
  - mandatory production backup restore rollback mail observability launch-scope and controlled-smoke evidence is absent
next_action: complete and inspect the already-dispatched sanitized Synology inventory from observer run 30701773214 without mutating runtime state
```

## Notes

- Cloudflare configuration is not to be changed unless new direct evidence proves a specific drift.
- Production mutations, deployment, restart, rollback, restore, secret/config changes and mutation smoke remain owner-gated.
- Evidence committed here must be sanitized and must never include credentials, tokens, private endpoints, cookies, database dumps, TOTP data or recovery codes.
