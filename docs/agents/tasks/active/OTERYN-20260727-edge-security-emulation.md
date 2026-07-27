---
task_id: OTERYN-20260727-edge-security-emulation
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/adr/0007-phase7-completion-and-production-go-live-separation.md
  - docs/operations/PRODUCTION_READINESS_CHECKLIST.md
  - docs/operations/PRODUCTION_TOPOLOGY_EVIDENCE.md
  - docs/operations/PRODUCTION_VERIFICATION_EVIDENCE.md
  - docs/operations/CLOUDFLARE_ACCESS_ADMIN.md
search_first:
  - existing edge DNS TLS WAF Access and origin-bypass validation
  - active tasks and open pull requests touching production-like validation or Issue 91
  - exact production-boundary language that must remain UNKNOWN
optional_reads: []
---

# OTERYN-20260727-edge-security-emulation

## Goal

Add a deterministic repository-owned production-like edge profile that emulates DNS, public TLS termination, Cloudflare-style proxy metadata, WAF/rate limiting, optional Access denial and authenticated origin pulls without claiming that any actual production provider or hostname exists.

## Acceptance criteria

- [ ] A local authoritative DNS fixture proves intended A/CNAME records and NXDOMAIN behavior for unconfigured names.
- [ ] A public edge TLS fixture proves hostname verification, supported TLS versions and HTTP-to-HTTPS redirect behavior.
- [ ] Edge-to-origin TLS uses a separately generated trust chain and authenticated client certificate.
- [ ] Direct origin access without the edge client certificate fails closed.
- [ ] Cloudflare-style response metadata and forwarded-client normalization are proven without trusting spoofed inbound headers.
- [ ] Deterministic WAF checks block traversal, XSS and SQL-injection probes, unsupported methods and oversized bodies.
- [ ] A bounded rate-limit probe produces HTTP 429 under controlled burst load.
- [ ] The optional administrator Access gate denies missing/invalid assertions while successful edge admission still leaves Platform auth/MFA/RBAC authoritative.
- [ ] A real current-SHA Laravel `/health` response succeeds through DNS/TLS/edge/origin composition.
- [ ] Sanitized exact-SHA evidence is uploaded and recorded as `STAGING_PROVEN` only.
- [ ] Issue #91 and every actual production DNS/TLS/Cloudflare/WAF/origin fact remain `UNKNOWN` pending a real environment.
- [ ] No production, router, DSM, public DNS, Cloudflare account, secret or external-repository action occurs.

## Ownership

```yaml
owned_paths:
  - .github/workflows/edge-security-emulation.yml
  - tests/edge-emulation/**
  - docs/operations/EDGE_SECURITY_EMULATION_EVIDENCE.md
  - docs/operations/PRODUCTION_LIKE_VALIDATION_EVIDENCE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-edge-security-emulation.md
  - docs/agents/tasks/archive/OTERYN-20260727-edge-security-emulation.md
modules:
  - Deployment
  - Security
  - Testing
  - AgentGovernance
dependencies:
  - open Issue 91 production boundary
  - existing Phase 7 production-like validation
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T15:22:00+02:00
head: 4afdc2c4ed39ec5a50c461af8ec1195a6b23a4bf
branch: test/OTERYN-20260727-edge-security-emulation
pr: 236
status: validating
context_routes:
  - agent-governance
  - deployment
  - security
  - testing
owned_paths:
  - .github/workflows/edge-security-emulation.yml
  - tests/edge-emulation/**
  - docs/operations/EDGE_SECURITY_EMULATION_EVIDENCE.md
  - docs/operations/PRODUCTION_LIKE_VALIDATION_EVIDENCE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-edge-security-emulation.md
  - docs/agents/tasks/archive/OTERYN-20260727-edge-security-emulation.md
proven:
  - main base cab40863bd5058209cdcbee1342a54acc814ec01 has no other active task after completed public-web archival
  - Issue 91 remains open and requires actual production evidence before PRODUCTION_PROVEN
  - the repository previously proved application and dependency boundaries but left final DNS TLS WAF Access and origin exposure UNKNOWN
  - PR 236 contains a reserved-domain CoreDNS fixture, ephemeral public and origin trust chains, mTLS-authenticated origin pull, Cloudflare-style edge metadata, WAF and rate-limit assertions, Access admission and current-SHA Laravel health composition
  - Cloudflare Access admission remains independent from Platform auth confirmed MFA exact RBAC and audit
  - workflow run 30269153474 reached and exercised the complete edge harness before artifact publication
  - no production router DSM public DNS Cloudflare account secret or external-repository action occurred
derived:
  - successful exact-head validation can add STAGING_PROVEN procedure evidence without changing actual production UNKNOWN facts
unknown:
  - exact final-head workflow result after CRLF-safe header capture and pipefail correction
conflicts: []
first_failure:
  marker: http-redirect-target
  evidence: edge run 30268938970 first failed after Nginx and CoreDNS startup because the header assertion compared CRLF curl output without normalization; run 30269153474 diagnostics artifact 8654060873 preserved the bounded marker
rejected_hypotheses:
  - DNS TLS mTLS or Laravel startup caused the first failure: those stages completed before the redirect-header assertion
  - emulation can close Issue 91: actual production environment evidence remains mandatory
changed_paths:
  - .github/workflows/edge-security-emulation.yml
  - tests/edge-emulation/Corefile
  - tests/edge-emulation/db.oteryn.test
  - tests/edge-emulation/access_verifier.py
  - tests/edge-emulation/bin/curl
  - tests/edge-emulation/run.sh
  - docs/operations/EDGE_SECURITY_EMULATION_EVIDENCE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-edge-security-emulation.md
validation:
  - command: repository and Issue 91 boundary review
    result: PASS
    evidence: production checklist topology baseline verification record and Access guidance preserve fail-closed production semantics
  - command: Edge Security Emulation 30268938970
    result: FAIL
    evidence: first unmet invariant http-redirect-target after successful application Nginx and DNS startup
  - command: Edge Security Emulation 30269153474
    result: FAIL
    evidence: validation output was masked by tee before pipefail was enforced; diagnostics artifact 8654060873 exposed the same CRLF-sensitive header marker
blockers: []
next_action: Obtain an exact-head PASS from the corrected edge workflow and persist its sanitized artifact evidence.
```

## Notes

The emulation uses reserved test hostnames and ephemeral keys only. It may prove the reviewed topology and controls under CI, but it must not imply ownership or configuration of a real DNS zone, certificate, Cloudflare account, firewall or production origin.
