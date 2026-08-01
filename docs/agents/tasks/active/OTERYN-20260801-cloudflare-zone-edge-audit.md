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

- [ ] Every Cloudflare request made by the live audit uses HTTP GET.
- [ ] The workflow can access Cloudflare credentials only from `main` through the protected `production-cloudflare` environment.
- [ ] Pull requests run deterministic validation only and receive no Cloudflare secret.
- [ ] Certificate coverage for both canonical hostnames is classified from sanitized API data.
- [ ] TLS, Always Use HTTPS, HSTS, Browser Integrity Check and security-level settings are classified read-only.
- [ ] Relevant zone rulesets, Bot configuration and Access applications are inspected read-only or recorded as `UNKNOWN` with the exact missing API capability.
- [ ] Output contains no token, resource IDs, rule expressions, private hostnames or full API responses.
- [ ] The live result is persisted as sanitized evidence and linked to Issue #91 / PR #405.
- [ ] Exact remediation diff, risk and rollback are presented before any separate apply authorization.

## Ownership

```yaml
owned_paths:
  - .github/workflows/cloudflare-zone-edge-audit.yml
  - scripts/operations/cloudflare-zone-edge-audit.sh
  - tests/operations/cloudflare-zone-edge-audit/**
  - docs/operations/CLOUDFLARE_ZONE_EDGE_AUDIT.md
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-zone-edge-audit.md
modules:
  - production-operations
  - edge-security
  - ci
dependencies:
  - issue-91
  - PR-401-cloudflare-endpoint-automation
  - PR-405-production-go-live-gate
blockers:
  - live zone-edge API permissions are not yet proven
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-01T13:53:00Z
head: 8188089a8e307d5480747868a8a55c90cca72698
branch: agent/cloudflare-zone-edge-audit
pr: 409
status: validating
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
proven:
  - Existing merged automation manages only the fixed Tunnel ingress and two canonical DNS records.
  - Independent public evidence proves WWW Cloudflare 403 challenge content, unusable Gateway TLS, absent HTTP redirect and HSTS max-age zero.
  - Owner explicitly authorized a separate read-only Cloudflare zone-edge audit on 2026-08-01.
  - No open pull request found with overlapping Cloudflare zone-edge audit ownership.
derived:
  - Tunnel and DNS convergence cannot prove separately controlled certificate, zone-setting, WAF, Bot or Access state.
unknown:
  - Effective certificate-pack coverage for login.oteryn.molehill.cloud
  - Effective zone TLS, redirect and HSTS configuration
  - Effective challenge source among zone security settings, rulesets, Bot controls and Access
  - Whether the existing production-cloudflare token has all required read capabilities
conflicts: []
first_failure:
  marker: none
  evidence: implementation preflight complete
rejected_hypotheses:
  - Tunnel and DNS current state proves public edge readiness: direct public probes after convergence still fail.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260801-cloudflare-zone-edge-audit.md
  - .github/workflows/cloudflare-zone-edge-audit.yml
  - scripts/operations/cloudflare-zone-edge-audit.sh
  - tests/operations/cloudflare-zone-edge-audit/mock_curl.py
  - tests/operations/cloudflare-zone-edge-audit/run.sh
  - docs/operations/CLOUDFLARE_ZONE_EDGE_AUDIT.md
validation:
  - command: repository Issue #91 PR #401 PR #387 and PR #405 preflight
    result: PASS
    evidence: bounded repository and live GitHub state inspected on 2026-08-01
  - command: bash tests/operations/cloudflare-zone-edge-audit/run.sh
    result: PASS
    evidence: syntax, workflow boundary, dynamic GET-only request log, certificate wildcard semantics, sanitized output and token-redaction checks passed locally
blockers:
  - live zone-edge API permissions are not yet proven
next_action: inspect all PR 409 checks on the exact head, repair any failure, then merge only if the repository merge gate passes
```

## Notes

- Trust boundary: the Cloudflare token exists only in the protected GitHub environment and is never printed.
- Authentication/authorization invariant: repository events cannot access the token; live audit is fixed to the configured account and zone IDs.
- Canary/schema/session compatibility: unchanged.
- Rollback: not applicable to the audit because external mutation is forbidden.
- Production-only configuration: account/zone identifiers and token remain outside Git and outside sanitized output.
