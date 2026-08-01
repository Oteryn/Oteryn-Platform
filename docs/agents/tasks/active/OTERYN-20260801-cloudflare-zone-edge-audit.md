---
task_id: OTERYN-20260801-cloudflare-zone-edge-audit
project_lane: oteryn-platform-core
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - deploy/synology/PUBLIC_ENDPOINTS.md
  - docs/operations/PRODUCTION_READINESS_CHECKLIST.md
search_first:
  - active Cloudflare zone edge certificate WAF Access redirect HSTS tasks and pull requests
  - existing Cloudflare API automation
optional_reads:
  - PR #401
  - PR #402
  - PR #387
  - PR #405
  - Issue #91
---

# OTERYN-20260801-cloudflare-zone-edge-audit

## Goal

Add and execute a separately protected, fail-closed, read-only Cloudflare zone-edge audit for the two canonical Oteryn hostnames, covering edge certificate coverage, zone TLS settings, HTTP-to-HTTPS and HSTS controls, WAF/Bot/challenge rules and Cloudflare Access applications without changing Tunnel ingress, DNS or any external state.

## Acceptance criteria

- [x] Every Cloudflare request made by the live audit uses HTTP GET.
- [x] The workflow can access Cloudflare credentials only from `main` through the protected `production-cloudflare` environment.
- [x] Pull requests run deterministic validation only and receive no Cloudflare secret.
- [ ] Certificate coverage for both canonical hostnames is classified from sanitized API data.
- [ ] TLS, Always Use HTTPS, HSTS, Browser Integrity Check and security-level settings are classified read-only.
- [x] Relevant zone rulesets, Bot configuration and Access applications are inspected read-only or recorded as `UNKNOWN` with the exact missing API capability.
- [x] Output contains no token, resource IDs, rule expressions, private hostnames or full API responses.
- [x] The live result is persisted as sanitized evidence and linked to Issue #91 / PR #405.
- [x] Exact remediation diff, risk and rollback are presented before any separate apply authorization.

## Ownership

```yaml
owned_paths:
  - .github/workflows/cloudflare-zone-edge-audit.yml
  - scripts/operations/cloudflare-zone-edge-audit.sh
  - tests/operations/cloudflare-zone-edge-audit/**
  - docs/operations/CLOUDFLARE_ZONE_EDGE_AUDIT.md
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-zone-edge-audit.md
  - docs/agents/evidence/OTERYN-20260801-cloudflare-zone-edge-audit/**
modules:
  - production-operations
  - edge-security
  - ci
dependencies:
  - issue-91
  - PR-401-cloudflare-endpoint-automation
  - PR-405-production-go-live-gate
blockers:
  - active Cloudflare token lacks every required zone-edge read capability
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-01T14:03:00Z
head: d30d111957a82400660ef461a7a7981c31edbd00
branch: agent/cloudflare-zone-edge-audit-evidence
pr: 415
status: blocked
context_routes:
  - agent-governance
  - security
  - testing
owned_paths:
  - .github/workflows/cloudflare-zone-edge-audit.yml
  - scripts/operations/cloudflare-zone-edge-audit.sh
  - tests/operations/cloudflare-zone-edge-audit/**
  - docs/operations/CLOUDFLARE_ZONE_EDGE_AUDIT.md
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-zone-edge-audit.md
  - docs/agents/evidence/OTERYN-20260801-cloudflare-zone-edge-audit/**
proven:
  - Existing merged automation manages only the fixed Tunnel ingress and two canonical DNS records.
  - Independent public evidence proves WWW Cloudflare 403 challenge content, unusable Gateway TLS, absent HTTP redirect and HSTS max-age zero.
  - Owner explicitly authorized a separate read-only Cloudflare zone-edge audit on 2026-08-01.
  - No open pull request found with overlapping Cloudflare zone-edge audit ownership.
  - PR 409 merged the protected GET-only audit as main SHA cff0ee1b8ecfd1d795e2636d488be6d1d1d0b4ea after every required exact-head check passed.
  - Live run 30702827936 job 91376722540 artifact 8819370547 executed with mutation none and secrets emitted false.
  - The account-owned token is active, but all nine certificate, settings, rulesets, Bot, Access and Page Rule reads returned HTTP 403.
derived:
  - Tunnel and DNS convergence cannot prove separately controlled certificate, zone-setting, WAF, Bot or Access state.
unknown:
  - Effective certificate-pack coverage for login.oteryn.molehill.cloud
  - Effective zone TLS, redirect and HSTS configuration
  - Effective challenge source among zone security settings, rulesets, Bot controls and Access
conflicts: []
first_failure:
  marker: zone-edge-read-permissions-denied
  evidence: run 30702827936 artifact 8819370547 records HTTP 403 for all nine requested zone-edge read surfaces while token verification is active
rejected_hypotheses:
  - Tunnel and DNS current state proves public edge readiness: direct public probes after convergence still fail.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-zone-edge-audit.md
  - .github/workflows/cloudflare-zone-edge-audit.yml
  - scripts/operations/cloudflare-zone-edge-audit.sh
  - tests/operations/cloudflare-zone-edge-audit/mock_curl.py
  - tests/operations/cloudflare-zone-edge-audit/run.sh
  - docs/operations/CLOUDFLARE_ZONE_EDGE_AUDIT.md
  - docs/agents/evidence/OTERYN-20260801-cloudflare-zone-edge-audit/index.md
validation:
  - command: repository Issue #91 PR #401 PR #387 and PR #405 preflight
    result: PASS
    evidence: bounded repository and live GitHub state inspected on 2026-08-01
  - command: bash tests/operations/cloudflare-zone-edge-audit/run.sh
    result: PASS
    evidence: syntax, workflow boundary, dynamic GET-only request log, certificate wildcard semantics, sanitized output and token-redaction checks passed locally and in PR run 30702716567
  - command: protected Cloudflare Zone Edge Audit run 30702827936
    result: BLOCKED
    evidence: job 91376722540 artifact 8819370547 digest sha256:d0e303b88b5ecc39a80c7020d8da9741e05f31b70caaca2ce47fa80d13a56a67; token active but all nine reads HTTP 403; mutation none
blockers:
  - live zone-edge API permissions are not yet proven
next_action: obtain explicit owner authorization to create a separate least-privilege read-only Cloudflare token and protected CLOUDFLARE_ZONE_AUDIT_TOKEN secret, then rerun the unchanged GET-only audit
```

## Notes

- Trust boundary: the Cloudflare token exists only in the protected GitHub environment and is never printed.
- Authentication/authorization invariant: repository events cannot access the token; live audit is fixed to the configured account and zone IDs.
- Canary/schema/session compatibility: unchanged.
- Rollback: not applicable to the audit because external mutation is forbidden.
- Production-only configuration: account/zone identifiers and token remain outside Git and outside sanitized output.
