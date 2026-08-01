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
lease_expires_at: 2026-08-01T13:42:43.544Z
context_pressure: high
context_growth: stable
context_score: 12
estimate_confidence: high
decomposition_decision: phased
decomposition_reason: deployment identity, Synology origin, public edge, readiness, and smoke are sequential gates sharing one release verdict
validation_level: full
last_completed_step: exact Synology runtime and public-edge evidence persisted with blocked verdict
session_rotation_count: 0
heavy_validation_runs: 2
stale_takeover_count: 0
human_interruptions: 0
updated_at: 2026-08-01T13:42:43.544Z
head: 90f367963ddaee6fa6884319fc8cc54e23ca8ec4
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
  - Sanitized Synology inventory run 30701775782 job 91373911925 artifact 8819161257 directly observed Platform and Gateway source SHA 3eb109b505f7d1c8718cffb823de6d9d5166717c and immutable Canary digest sha256:784e5dbdcc64e311c48c51cd94aa206e2efa1e5eefb2f4ef40170d5aac55031f.
  - The Synology Compose project is oteryn-staging and effective APP_ENV is staging; production:verify-configuration exits 1 and production_environment_proven is false.
  - All six expected containers are running with restart policy always; MariaDB and Redis are healthy and all bounded Platform and Gateway container and host-loopback probes pass.
  - Platform is loopback-bound on 8000, Gateway on loopback 8080, Canary legacy login on loopback 7171, Canary game on one private-IP 7172 binding, and MariaDB Redis internal-proxy expose no host ports.
  - No cloudflared container is visible through Docker; host-process and effective network-path state remain unknown.
  - Runtime uses file sessions file cache synchronous queue and array non-delivery mail.
  - No Cloudflare DNS Synology runtime database Redis secret deployment rollback restore or application-data mutation was performed.
derived:
  - The exact running target is a staging deployment and cannot be classified as PRODUCTION_PROVEN.
  - Canonical public endpoint failures independently block launch and prevent application-level production smoke.
  - Array mail directly blocks real production password-recovery delivery.
  - Mutation smoke is unsafe while production runtime rollback dated restore controlled identities and public application reachability remain unproven.
  - The only correct verdict is BLOCKED — PENDING PRODUCTION VERIFICATION.
unknown:
  - cloudflared host-process status network mode and effective path to both loopback origins
  - production DB topology credentials effective grants backup policy and dated restore evidence
  - production Redis ACL TLS freshness monitoring and selected session cache queue topology
  - production mail provider sender-domain readiness and bounce monitoring
  - centralized logs metrics alerts and on-call ownership
  - actual production deployment migration and emergency rollback mechanism
  - launch-scope decisions controlled smoke identities and all mutation smoke results
conflicts:
  - archived Synology preflight requires unless-stopped while current live runtime proves always for all six services; open PR #335 proposes always but remains unmerged
first_failure:
  marker: effective-runtime-not-production
  evidence: run 30701775782 artifact 8819161257 classifies the exact running target as STAGING_TARGET with APP_ENV staging and production configuration verifier exit 1
rejected_hypotheses:
  - Cloudflare configuration convergence proves production readiness: run 30700054602 verifies only managed Tunnel and DNS scope
  - local loopback health proves public delivery: run 30701140509 still observes WWW 403 and Game Gateway TLS failure
  - open PR #335 proves the running restart policy: only inventory run 30701775782 directly proves always
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
    evidence: job 91373030006 stopped on stale unless-stopped restart-policy expectation with restore disabled
  - command: sanitized Synology inventory run 30701775782
    result: PASS
    evidence: artifact 8819161257 digest sha256:67b1d16eb67f90e1534a9071644aeaf42da97adc527643964a14df120c37db9c
  - command: PR #405 repository workflows on head 0c435dd02d2afcc7f0e8d963a79b5441b29a6cb7
    result: PASS
    evidence: runs 30701773251 30701773237 30701773212 30701773227 30701773203 30701773233 30701773198 and 30701773188
  - command: final production readiness and mutation smoke
    result: BLOCKED
    evidence: exact running target is staging and mandatory public edge backup rollback mail observability and controlled-smoke prerequisites fail or remain unproven
blockers:
  - exact running target is STAGING_TARGET and production configuration verification fails
  - canonical WWW returns Cloudflare 403 challenge content instead of Oteryn Platform
  - canonical Game Gateway fails TLS before HTTP
  - plain HTTP does not redirect to HTTPS and HSTS remains max-age=0
  - cloudflared host-process and loopback routing topology remain unproven
  - mandatory production backup restore rollback mail observability launch-scope and controlled-smoke evidence is absent
next_action: obtain explicit owner authorization for a separate guarded Cloudflare zone-edge audit/apply task covering login certificate WWW challenge policy HTTP redirect and HSTS without changing current Tunnel ingress or DNS
```

## Notes

- Cloudflare configuration is not to be changed unless new direct evidence proves a specific drift.
- Production mutations, deployment, restart, rollback, restore, secret/config changes and mutation smoke remain owner-gated.
- Evidence committed here must be sanitized and must never include credentials, tokens, private endpoints, cookies, database dumps, TOTP data or recovery codes.
