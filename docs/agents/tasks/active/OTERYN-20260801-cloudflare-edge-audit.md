---
task_id: OTERYN-20260801-cloudflare-edge-audit
required_reads:
  - AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - deploy/synology/PUBLIC_ENDPOINTS.md
  - docs/architecture/adr/0020-use-single-level-gateway-public-hostname.md
  - docs/operations/CLOUDFLARE_ENDPOINT_MANAGEMENT.md
  - docs/operations/CLOUDFLARE_EDGE_AUDIT.md
search_first:
  - PR #401 guarded Tunnel and DNS reconciliation
  - PR #420 dedicated edge-audit user token
  - PR #422 sanitized rule-scope evidence
  - PR #425 fixed-scope apply preflight
  - PR #424 final read-only edge checkpoint
  - PR #427 single-level Gateway hostname cutover
  - runs 30708559130 and 30709108382
optional_reads:
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
  - Issue #91
---

# OTERYN-20260801-cloudflare-edge-audit

## Goal

Cut the public Game Gateway over from the uncovered multi-level hostname `login.oteryn.molehill.cloud` to the owner-approved single-level hostname `gateway.molehill.cloud`, preserving the dedicated Gateway origin, avoiding Advanced Certificate Manager, retiring the legacy edge route safely, and continuing the bounded WAF/Bot repair.

## Acceptance criteria

- [x] Permission-complete read-only Cloudflare edge state is recorded.
- [x] The owner approved `gateway.molehill.cloud` as the canonical public Game Gateway hostname.
- [x] ADR 0020 and all current public-endpoint contracts use `gateway.molehill.cloud`.
- [x] Tunnel/DNS automation migrates to the new hostname, preserves unrelated configuration and retires only the exact safe legacy route/record.
- [x] Edge auditing recognizes one-label Universal `*.molehill.cloud` coverage for the canonical Gateway hostname and rejects coverage for the retired two-label hostname.
- [x] Synology health validation uses the canonical Gateway Host header.
- [x] Current runtime/configuration reuse of the retired hostname is blocked by CI while historical evidence and explicit migration code remain allowed.
- [x] Deterministic endpoint, primary edge-audit and zone-edge-audit tests pass.
- [x] Exact-head repository workflow gates pass for implementation head `5083f0aa6f903db58accb0662d82dacd2554930b`.
- [ ] Trusted-main Cloudflare endpoint audit proves the bounded live migration plan.
- [ ] Authorized live Tunnel/DNS migration succeeds with post-write verification.
- [ ] Trusted-main edge audits prove active wildcard coverage and current policy state after migration.
- [ ] Public DNS/TLS/HTTP probes validate `gateway.molehill.cloud`.
- [ ] Native-client endpoint configuration is updated in its separately controlled consumer repository or release channel.
- [ ] The smallest remaining WAF/Bot repair is implemented only after the required zone-scoped Edit permissions are available.
- [ ] `PUBLIC_DOMAIN_LAUNCH_READY` and `PRODUCTION_PROVEN` remain false until all applicable production gates pass.

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
  - GitHub environment production-cloudflare
  - existing account token for Tunnel/DNS
  - dedicated edge-audit token for zone inspection
blockers:
  - none for repository implementation or bounded Tunnel/DNS migration
  - WAF/Bot apply remains externally blocked until the dedicated token has Zone WAF Edit and Bot Management Edit
cross_repository_tasks:
  - native client endpoint rollout remains a separately controlled consumer update
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-01T17:41:00Z
head: 5083f0aa6f903db58accb0662d82dacd2554930b
branch: fix/OTERYN-20260801-gateway-single-level-host-cutover
pr: 427
status: ready_for_merge
policy_version: 2
task_kind: implementation
phase: repository_implementation_validated
execution_mode: chat-github-connector
execution_reason: repository implementation, deterministic emulation, exact-head CI and guarded production continuation
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
  - Main was fe92c4f66d5bbae280c2ae44ad01e392519a2069 when branch work began.
  - The owner approved gateway.molehill.cloud as the replacement public Game Gateway hostname.
  - Universal certificate packs support one-label wildcard coverage while the retired multi-level hostname was not covered.
  - Advanced Certificate Manager quota was zero and is no longer required by the accepted hostname design.
  - The Gateway remains a separate service bound to Synology loopback 127.0.0.1:8080.
  - The endpoint migration preserves unrelated Tunnel ingress and retires legacy DNS only when it is exactly one CNAME to the same managed tunnel target.
  - The new canonical Tunnel/DNS state is verified before legacy DNS deletion and apply is idempotent.
  - Both read-only edge audits classify gateway.molehill.cloud as canonical and login.oteryn.molehill.cloud as retired.
  - Synology health validation sends gateway.molehill.cloud as the Gateway Host header.
  - Implementation head 5083f0aa6f903db58accb0662d82dacd2554930b passed every triggered repository workflow.
derived:
  - No paid Advanced Certificate Manager add-on or SSL and Certificates Edit scope is required for the hostname migration.
  - The account token can perform the bounded Tunnel/DNS migration after trusted-main audit confirms safe live state.
  - Zone WAF and Bot remediation remains independent from hostname/certificate migration.
unknown:
  - Exact live Tunnel/DNS drift for gateway.molehill.cloud until the trusted-main endpoint audit runs.
  - Exact public DNS/TLS/HTTP behavior after migration.
  - Exact native-client consumer repository or release configuration requiring the endpoint update.
conflicts:
  - Historical evidence and migration code intentionally mention login.oteryn.molehill.cloud while current runtime/configuration use is forbidden.
first_failure:
  marker: legacy-multilevel-gateway-hostname-not-covered
  evidence: runs 30708559130 and 30709108382 proved missing certificate coverage and zero Advanced Certificate Manager quota
rejected_hypotheses:
  - A path-based Gateway under the WWW hostname is required; the dedicated single-level hostname preserves service separation without path rewriting.
  - Advanced Certificate Manager is required; gateway.molehill.cloud is eligible for existing one-label wildcard coverage.
  - The retired DNS record can be deleted unconditionally; ambiguous, non-CNAME or externally targeted legacy state must fail closed.
  - Tunnel/DNS convergence alone repairs public 403 responses; the country-based WAF rule and Bot Fight Mode remain separate proven controls.
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
    evidence: bounded migration, exact legacy retirement, idempotency and retired-host reuse guard succeeded on head 5083f0aa6f903db58accb0662d82dacd2554930b
  - command: Cloudflare Oteryn Edge Audit run 30710780509
    result: PASS
    evidence: wildcard-aware canonical/retired host classification and GET-only guarantees succeeded
  - command: Cloudflare Zone Edge Audit run 30710780471
    result: PASS
    evidence: schema-version-2 canonical/retired host audit and separate token wiring succeeded
  - command: Agent Governance run 30710780532
    result: PASS
    evidence: checkpoint validator and active-task governance succeeded
  - command: CI run 30710780512
    result: PASS
    evidence: formatting, static analysis and complete application test suite succeeded
  - command: Phase 7 Production-Like Validation run 30710780506
    result: PASS
    evidence: deployment, security, database, Redis, SMTP, health, backup, restore and rollback validations succeeded
  - command: Build Synology Staging Images run 30710780556
    result: PASS
    evidence: deployment package validation and platform, Gateway and deploy-runner image builds succeeded
  - command: Edge Security Emulation run 30710780492
    result: PASS
    evidence: exact-head edge security emulation succeeded
  - command: Game Auth Ticket Concurrency run 30710780521
    result: PASS
    evidence: independent-process MariaDB concurrency proof succeeded
  - command: Platform DB Outage Validation run 30710780499
    result: PASS
    evidence: fail-closed outage and recovery validation succeeded
blockers:
  - none for merge or trusted-main Tunnel/DNS audit
  - WAF/Bot mutation requires externally adding Zone WAF Edit and Bot Management Edit to the dedicated token
next_action: Merge PR #427, run the trusted-main Cloudflare endpoint audit, apply only if the legacy record is proven safe to retire, then re-run edge audits and public DNS/TLS/HTTP acceptance.
```

## Report

`docs/agents/reports/OTERYN-20260801-cloudflare-edge-audit.md`
