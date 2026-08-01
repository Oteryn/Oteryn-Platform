---
task_id: OTERYN-20260801-cloudflare-edge-audit
required_reads:
  - AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - deploy/synology/PUBLIC_ENDPOINTS.md
  - docs/operations/CLOUDFLARE_ENDPOINT_MANAGEMENT.md
  - docs/operations/CLOUDFLARE_EDGE_AUDIT.md
search_first:
  - PR #401 guarded Tunnel and DNS reconciliation
  - PR #420 dedicated edge-audit user token
  - PR #422 sanitized rule-scope evidence
  - PR #425 fixed-scope apply preflight
  - PR #424 final read-only edge checkpoint
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
- [ ] An accepted ADR and all public-endpoint contracts use `gateway.molehill.cloud`.
- [ ] Tunnel/DNS automation migrates to the new hostname, preserves unrelated configuration and retires only the exact safe legacy route/record.
- [ ] Edge auditing recognizes Universal `*.molehill.cloud` coverage for the single-level Gateway hostname.
- [ ] Deterministic endpoint and edge-audit tests pass.
- [ ] Exact-head repository workflow gates pass.
- [ ] Trusted-main Cloudflare audit proves the bounded migration plan.
- [ ] Authorized live Tunnel/DNS migration succeeds with post-write verification.
- [ ] Public TLS/HTTP probes validate the new Gateway hostname.
- [ ] The smallest remaining WAF/Bot repair is implemented only after the required zone-scoped Edit permissions are available.
- [ ] `PUBLIC_DOMAIN_LAUNCH_READY` and `PRODUCTION_PROVEN` remain false until all applicable production gates pass.

## Ownership

```yaml
owned_paths:
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - deploy/synology/PUBLIC_ENDPOINTS.md
  - deploy/synology/README.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/architecture/adr/README.md
  - docs/architecture/adr/0020-use-single-level-gateway-public-hostname.md
  - .github/workflows/cloudflare-oteryn-endpoints.yml
  - scripts/operations/cloudflare-oteryn-endpoints.sh
  - tests/operations/cloudflare-oteryn-endpoints/**
  - docs/operations/CLOUDFLARE_ENDPOINT_MANAGEMENT.md
  - .github/workflows/cloudflare-oteryn-edge-audit.yml
  - scripts/operations/cloudflare-oteryn-edge-audit.py
  - tests/operations/cloudflare-oteryn-edge-audit/**
  - docs/operations/CLOUDFLARE_EDGE_AUDIT.md
  - docs/agents/reports/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
modules:
  - operations
  - edge-security
  - game-gateway
  - synology-staging
dependencies:
  - GitHub environment production-cloudflare
  - existing account token for Tunnel/DNS
  - dedicated edge-audit token for read-only zone inspection
blockers:
  - none for repository implementation and Tunnel/DNS migration
  - WAF/Bot apply remains blocked until the dedicated token has the exact required Edit permissions
cross_repository_tasks:
  - native client endpoint rollout remains a separately controlled consumer update
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-01T17:12:00Z
head: fe92c4f66d5bbae280c2ae44ad01e392519a2069
branch: fix/OTERYN-20260801-gateway-single-level-host-cutover
pr: none
status: implementing
policy_version: 2
task_kind: implementation
phase: gateway_hostname_cutover
execution_mode: chat-github-connector
execution_reason: repository state inspection, bounded multi-file edits and GitHub-native validation
context_pressure: medium
decomposition_decision: phased
context_routes:
  - agent-governance
  - security
  - api
  - testing
owned_paths:
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - deploy/synology/PUBLIC_ENDPOINTS.md
  - deploy/synology/README.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/architecture/adr/README.md
  - docs/architecture/adr/0020-use-single-level-gateway-public-hostname.md
  - .github/workflows/cloudflare-oteryn-endpoints.yml
  - scripts/operations/cloudflare-oteryn-endpoints.sh
  - tests/operations/cloudflare-oteryn-endpoints/**
  - docs/operations/CLOUDFLARE_ENDPOINT_MANAGEMENT.md
  - scripts/operations/cloudflare-oteryn-edge-audit.py
  - tests/operations/cloudflare-oteryn-edge-audit/**
  - docs/operations/CLOUDFLARE_EDGE_AUDIT.md
  - docs/agents/reports/OTERYN-20260801-cloudflare-edge-audit.md
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
proven:
  - Main is fe92c4f66d5bbae280c2ae44ad01e392519a2069 after PR #424.
  - Universal certificate packs currently cover molehill.cloud and one-label wildcard hostnames but do not cover login.oteryn.molehill.cloud.
  - Advanced Certificate Manager quota is zero.
  - The owner approved gateway.molehill.cloud as the replacement public Game Gateway hostname.
  - The Gateway remains a separate HTTP service bound to Synology loopback 127.0.0.1:8080.
  - Existing Tunnel/DNS automation can reconcile fixed hostnames while preserving unrelated ingress configuration.
  - The broad country-based WAF block and Bot Fight Mode remain independent from certificate coverage.
derived:
  - gateway.molehill.cloud can use existing Universal wildcard certificate coverage without the paid Advanced Certificate Manager add-on.
  - The legacy hostname must be removed deliberately from Tunnel and DNS because merely changing a constant would preserve it as unrelated configuration.
  - WAF/Bot repair must remain separate from the hostname migration and preserve unrelated zone policy.
unknown:
  - Exact live Tunnel/DNS drift for gateway.molehill.cloud until trusted-main audit runs.
  - Exact public TLS/HTTP behavior of gateway.molehill.cloud after migration.
  - Native-client repository/configuration location for the final consumer endpoint update.
conflicts:
  - Existing contracts designate login.oteryn.molehill.cloud while the owner has now superseded that hostname decision with gateway.molehill.cloud.
first_failure:
  marker: legacy-multilevel-gateway-hostname-not-covered
  evidence: runs 30708559130 and 30709108382 prove missing certificate coverage and zero Advanced Certificate Manager quota
rejected_hypotheses:
  - A path-based Gateway under the WWW hostname is required; a dedicated single-level hostname preserves service separation and avoids path rewriting.
  - Advanced Certificate Manager is required; the approved single-level Gateway hostname is eligible for Universal wildcard coverage.
  - Tunnel/DNS convergence alone repairs public 403 responses; the WAF country block and Bot Fight Mode are separately proven controls.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-edge-audit.md
validation:
  - command: repository and open-PR overlap inspection
    result: PASS
    evidence: no open PR owns the endpoint automation, public endpoint contract or edge-audit implementation paths
  - command: implementation tests
    result: NOT_RUN
    evidence: implementation has not yet been committed
blockers:
  - none for the current implementation phase
next_action: Implement the accepted single-level Gateway hostname contract, guarded legacy migration and wildcard-aware edge audit, then run focused and exact-head validation.
```

## Report

`docs/agents/reports/OTERYN-20260801-cloudflare-edge-audit.md`
