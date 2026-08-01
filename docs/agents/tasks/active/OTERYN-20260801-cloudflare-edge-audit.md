---
task_id: OTERYN-20260801-cloudflare-edge-audit
required_reads:
  - AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - deploy/synology/PUBLIC_ENDPOINTS.md
  - docs/architecture/adr/0020-use-single-level-gateway-public-hostname.md
  - docs/operations/CLOUDFLARE_ENDPOINT_MANAGEMENT.md
  - docs/operations/CLOUDFLARE_EDGE_AUDIT.md
search_first:
  - PR #427 single-level Gateway hostname cutover
  - PR #424 final read-only edge checkpoint
  - runs 30708559130 and 30709108382
optional_reads:
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
  - Issue #91
---

# OTERYN-20260801-cloudflare-edge-audit

## Goal

Replace the uncovered `login.oteryn.molehill.cloud` Game Gateway hostname with the owner-approved `gateway.molehill.cloud`, preserve the dedicated `127.0.0.1:8080` Gateway origin, avoid Advanced Certificate Manager, and retire only the exact safe legacy Tunnel/DNS state.

## Acceptance criteria

- [x] ADR 0020 and current public endpoint contracts designate `gateway.molehill.cloud`.
- [x] Tunnel/DNS automation preserves unrelated ingress and removes legacy state only when exact and safe.
- [x] Edge audits implement one-label wildcard semantics and classify the old hostname as retired.
- [x] Synology health checks use the canonical Gateway Host header.
- [x] CI blocks accidental runtime/configuration reuse of the retired hostname.
- [x] Focused tests and all exact-head repository workflows pass for implementation head `5083f0aa6f903db58accb0662d82dacd2554930b`.
- [ ] Trusted-main endpoint audit and guarded live migration complete.
- [ ] Post-migration edge audit and independent public DNS/TLS/HTTP acceptance pass.
- [ ] Native-client consumer configuration is updated through its separately controlled release path.
- [ ] WAF/Bot repair is performed only with the exact required Edit permissions.
- [ ] `PUBLIC_DOMAIN_LAUNCH_READY` and `PRODUCTION_PROVEN` remain false until production acceptance passes.

## Ownership

```yaml
owned_paths:
  - .github/workflows/cloudflare-oteryn-edge-audit.yml
  - .github/workflows/cloudflare-oteryn-endpoints.yml
  - .github/workflows/cloudflare-zone-edge-audit.yml
  - deploy/synology/PUBLIC_ENDPOINTS.md
  - deploy/synology/scripts/health-check.sh
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/reports/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/architecture/adr/0020-use-single-level-gateway-public-hostname.md
  - docs/architecture/adr/README.md
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - docs/operations/CLOUDFLARE_EDGE_AUDIT.md
  - docs/operations/CLOUDFLARE_ENDPOINT_MANAGEMENT.md
  - docs/operations/CLOUDFLARE_ZONE_EDGE_AUDIT.md
  - scripts/operations/cloudflare-oteryn-edge-audit.py
  - scripts/operations/cloudflare-oteryn-endpoints.sh
  - scripts/operations/cloudflare-zone-edge-audit.sh
  - tests/operations/cloudflare-oteryn-edge-audit/**
  - tests/operations/cloudflare-oteryn-endpoints/**
  - tests/operations/cloudflare-zone-edge-audit/**
modules:
  - operations
  - edge-security
  - game-gateway
  - synology-staging
dependencies:
  - production-cloudflare GitHub environment
  - account token for Tunnel/DNS
  - dedicated user token for zone-edge reads
blockers:
  - none for merge or bounded Tunnel/DNS migration
  - WAF/Bot mutation still requires Zone WAF Edit and Bot Management Edit
cross_repository_tasks:
  - native-client endpoint rollout remains a separately controlled consumer update
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-01T17:44:00Z
head: 5083f0aa6f903db58accb0662d82dacd2554930b
branch: fix/OTERYN-20260801-gateway-single-level-host-cutover
pr: 427
status: ready
policy_version: 2
task_kind: implementation
phase: repository_implementation_validated
execution_mode: chat-github-connector
execution_reason: bounded repository implementation, deterministic emulation and exact-head validation
context_pressure: medium
decomposition_decision: phased
context_routes:
  - agent-governance
  - security
  - api
  - testing
owned_paths:
  - .github/workflows/cloudflare-oteryn-edge-audit.yml
  - .github/workflows/cloudflare-oteryn-endpoints.yml
  - .github/workflows/cloudflare-zone-edge-audit.yml
  - deploy/synology/PUBLIC_ENDPOINTS.md
  - deploy/synology/scripts/health-check.sh
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/reports/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/architecture/adr/0020-use-single-level-gateway-public-hostname.md
  - docs/architecture/adr/README.md
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - docs/operations/CLOUDFLARE_EDGE_AUDIT.md
  - docs/operations/CLOUDFLARE_ENDPOINT_MANAGEMENT.md
  - docs/operations/CLOUDFLARE_ZONE_EDGE_AUDIT.md
  - scripts/operations/cloudflare-oteryn-edge-audit.py
  - scripts/operations/cloudflare-oteryn-endpoints.sh
  - scripts/operations/cloudflare-zone-edge-audit.sh
  - tests/operations/cloudflare-oteryn-edge-audit/**
  - tests/operations/cloudflare-oteryn-endpoints/**
  - tests/operations/cloudflare-zone-edge-audit/**
proven:
  - Owner approved gateway.molehill.cloud as the canonical public Game Gateway hostname.
  - Universal wildcard coverage applies to one-label gateway.molehill.cloud and not to the retired two-label hostname.
  - Advanced Certificate Manager and SSL certificate write scope are unnecessary for this design.
  - Gateway remains bound to Synology loopback 127.0.0.1:8080 as a separate service.
  - Migration verifies new canonical Tunnel/DNS state before deleting an exact legacy CNAME to the same tunnel.
  - Ambiguous, non-CNAME, duplicate or externally targeted legacy state fails closed.
  - Implementation head 5083f0aa6f903db58accb0662d82dacd2554930b passed every triggered repository workflow.
derived:
  - Trusted-main endpoint audit may proceed without adding Cloudflare permissions or purchasing a certificate add-on.
  - WAF/Bot remediation remains independent from hostname migration.
unknown:
  - Live drift for gateway.molehill.cloud before trusted-main audit.
  - Public DNS/TLS/HTTP behavior after migration.
  - Native-client consumer repository or release configuration location.
conflicts:
  - Historical evidence and explicit migration code retain the old hostname while current runtime/configuration reuse is forbidden.
first_failure:
  marker: legacy-multilevel-gateway-hostname-not-covered
  evidence: runs 30708559130 and 30709108382 proved missing old-host coverage and zero Advanced Certificate Manager quota
rejected_hypotheses:
  - A path-based Gateway is required; a dedicated one-label hostname preserves separation without rewriting paths.
  - Advanced Certificate Manager is required; Universal SSL can cover gateway.molehill.cloud.
  - Legacy DNS can be deleted unconditionally; unsafe or ambiguous legacy state must fail closed.
  - Tunnel/DNS migration alone repairs 403 responses; WAF country policy and Bot Fight Mode remain separate controls.
changed_paths:
  - .github/workflows/cloudflare-oteryn-edge-audit.yml
  - .github/workflows/cloudflare-oteryn-endpoints.yml
  - .github/workflows/cloudflare-zone-edge-audit.yml
  - deploy/synology/PUBLIC_ENDPOINTS.md
  - deploy/synology/scripts/health-check.sh
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/reports/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/architecture/adr/0020-use-single-level-gateway-public-hostname.md
  - docs/architecture/adr/README.md
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - docs/operations/CLOUDFLARE_EDGE_AUDIT.md
  - docs/operations/CLOUDFLARE_ENDPOINT_MANAGEMENT.md
  - docs/operations/CLOUDFLARE_ZONE_EDGE_AUDIT.md
  - scripts/operations/cloudflare-oteryn-edge-audit.py
  - scripts/operations/cloudflare-oteryn-endpoints.sh
  - scripts/operations/cloudflare-zone-edge-audit.sh
  - tests/operations/cloudflare-oteryn-edge-audit/mock_server.py
  - tests/operations/cloudflare-oteryn-edge-audit/run.sh
  - tests/operations/cloudflare-oteryn-endpoints/check-legacy-hostname.sh
  - tests/operations/cloudflare-oteryn-endpoints/mock_cloudflare.py
  - tests/operations/cloudflare-oteryn-endpoints/run.sh
  - tests/operations/cloudflare-zone-edge-audit/run.sh
validation:
  - command: Cloudflare Oteryn Endpoints run 30710780495
    result: PASS
    evidence: migration, safe retirement, idempotency and legacy-host guard passed on 5083f0aa6f903db58accb0662d82dacd2554930b
  - command: Cloudflare Oteryn Edge Audit run 30710780509
    result: PASS
    evidence: wildcard-aware canonical and retired host classification passed
  - command: Cloudflare Zone Edge Audit run 30710780471
    result: PASS
    evidence: schema-version-2 GET-only audit and separate token wiring passed
  - command: Agent Governance run 30710780532
    result: PASS
    evidence: checkpoint and active-task governance passed
  - command: CI run 30710780512
    result: PASS
    evidence: formatting, static analysis and application tests passed
  - command: Phase 7 run 30710780506
    result: PASS
    evidence: production-like deployment, security, data, health, backup, restore and rollback checks passed
  - command: Build Synology Staging Images run 30710780556
    result: PASS
    evidence: package validation and platform, Gateway and deploy-runner image builds passed
  - command: Edge Security Emulation run 30710780492
    result: PASS
    evidence: exact-head edge security emulation passed
  - command: Game Auth Ticket Concurrency run 30710780521
    result: PASS
    evidence: independent-process MariaDB concurrency proof passed
  - command: Platform DB Outage Validation run 30710780499
    result: PASS
    evidence: fail-closed outage and recovery checks passed
blockers:
  - none for merge or trusted-main Tunnel/DNS audit
  - WAF/Bot apply remains blocked on Zone WAF Edit and Bot Management Edit
next_action: Merge PR #427, run trusted-main endpoint audit, apply only when legacy state is safe, then re-run edge audits and public DNS/TLS/HTTP acceptance.
```

## Report

`docs/agents/reports/OTERYN-20260801-cloudflare-edge-audit.md`
