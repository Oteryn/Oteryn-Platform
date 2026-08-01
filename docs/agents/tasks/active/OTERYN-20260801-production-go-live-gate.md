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
  - docs/operations/PRODUCTION_READINESS_CHECKLIST.md
  - docs/testing/PRODUCTION_SMOKE_CHECKLIST.md
modules:
  - production-operations
  - deployment-validation
  - public-edge-validation
dependencies:
  - issue-91
  - PR-387-public-domain-audit-read-only-reference
  - PR-335-synology-restart-policy-read-only-reference
blockers:
  - exact deployed Platform and Gateway identities are not yet directly proven
  - usable sanitized read-only production evidence path is not yet proven
cross_repository_tasks:
  - Canary and login-server remain read-only unless separately authorized
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
task_kind: validation
implementation_authorized: false
phase: investigate
session_id: agent-20260801-production-gate-001
session_role: investigator
execution_mode: chat
execution_reason: repository coordination, narrow documentation writes, GitHub evidence review, and read-only production probes
lease_expires_at: 2026-08-01T13:57:52.542Z
context_pressure: high
context_growth: stable
context_score: 12
estimate_confidence: high
decomposition_decision: phased
decomposition_reason: deployment identity, Synology origin, public edge, readiness, and smoke are sequential gates sharing one release verdict
validation_level: focused
last_completed_step: repository preflight and conflict review completed
session_rotation_count: 0
heavy_validation_runs: 0
stale_takeover_count: 0
human_interruptions: 0
updated_at: 2026-08-01T13:12:52.542Z
head: de949075d14ebecc57423237b9330d865da28645
branch: agent/production-go-live-gate
pr: none
status: investigating
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
  - docs/operations/PRODUCTION_READINESS_CHECKLIST.md
  - docs/testing/PRODUCTION_SMOKE_CHECKLIST.md
proven:
  - Issue #91 is open and its authoritative verdict is PENDING PRODUCTION VERIFICATION.
  - Branch agent/production-go-live-gate was created from repository commit de949075d14ebecc57423237b9330d865da28645.
  - Cloudflare audit run 30699270139 and apply run 30700054602 completed the fixed-scope Tunnel and canonical DNS reconciliation recorded by Issue #91.
  - docs/agents/ACTIVE_WORK.md records no active task in its index; open PRs remain authoritative.
  - Open PR #387 owns its separate public-domain audit report/task paths; open PR #335 owns Synology restart-policy implementation paths, so neither owns this task record or evidence path.
derived:
  - Cloudflare Tunnel/DNS convergence does not prove application-origin reachability, TLS coverage, WAF behavior, or production readiness.
  - The gate must remain fail-closed until exact deployed release identities and all mandatory production evidence are directly observed.
unknown:
  - exact deployed Oteryn Platform commit SHA and image digest
  - exact deployed Game Gateway commit SHA and image digest
  - exact deployed Canary revision when launch-applicable
  - effective Synology container, Compose, restart, health, port, network, and cloudflared topology
  - current Platform DB, Canary SQL, Redis, session, cache, queue, mail, observability, backup, restore, and rollback production evidence
  - launch scope and availability of controlled production smoke identities/data
conflicts: []
first_failure:
  marker: exact-deployed-release-identity
  evidence: Issue #91 and its latest comments do not directly identify the currently running Platform or Game Gateway release
rejected_hypotheses:
  - Cloudflare configuration convergence proves production readiness: run 30700054602 verifies only the managed Tunnel/DNS scope
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260801-production-go-live-gate.md
validation:
  - command: repository and Issue #91 preflight through GitHub connector
    result: PASS
    evidence: Issue #91 plus comments, default branch, mandatory documents, active-work index, and open PR inventory inspected on 2026-08-01
  - command: production readiness and smoke execution
    result: NOT_RUN
    evidence: exact deployed release identity and sanitized production evidence path must be established first
blockers:
  - exact deployed Platform and Gateway identities are not directly proven
  - sanitized read-only access to the actual Synology runtime is not yet proven available
next_action: identify and execute the existing read-only deployment-target inspection path for the Synology runtime without mutating production
```

## Notes

- Cloudflare configuration is not to be changed unless new direct evidence proves a specific drift.
- Production mutations, deployment, restart, rollback, restore, secret/config changes and mutation smoke remain owner-gated.
- Evidence committed here must be sanitized and must never include credentials, tokens, private endpoints, cookies, database dumps, TOTP data or recovery codes.
