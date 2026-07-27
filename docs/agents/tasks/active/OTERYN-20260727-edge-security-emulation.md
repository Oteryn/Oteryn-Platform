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

- [x] A local authoritative DNS fixture proves intended A/CNAME records and NXDOMAIN behavior for unconfigured names.
- [x] A public edge TLS fixture proves hostname verification, supported TLS versions and HTTP-to-HTTPS redirect behavior.
- [x] Edge-to-origin TLS uses a separately generated trust chain and authenticated client certificate.
- [x] Direct origin access without the edge client certificate fails closed.
- [x] Cloudflare-style response metadata and forwarded-client normalization are proven without trusting spoofed inbound headers.
- [x] Deterministic WAF checks block traversal, XSS and SQL-injection probes, unsupported methods and oversized bodies.
- [x] A bounded rate-limit probe produces HTTP 429 under controlled burst load.
- [x] The optional administrator Access gate denies missing/invalid assertions while successful edge admission still leaves Platform auth/MFA/RBAC authoritative.
- [x] A real current-SHA Laravel `/health` response succeeds through DNS/TLS/edge/origin composition.
- [x] Sanitized exact-SHA evidence is uploaded and recorded as `STAGING_PROVEN` only.
- [x] Issue #91 and every actual production DNS/TLS/Cloudflare/WAF/origin fact remain `UNKNOWN` pending a real environment.
- [x] No production, router, DSM, public DNS, Cloudflare account, secret or external-repository action occurs.

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
updated_at: 2026-07-27T15:36:00+02:00
head: d0e1aebf16992b27925c0f0cd59a5a1c04f65319
branch: test/OTERYN-20260727-edge-security-emulation
pr: 236
status: ready
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
  - Issue 91 remains open and requires actual production evidence before PRODUCTION_PROVEN
  - PR 236 adds a reserved-domain CoreDNS fixture, ephemeral public and origin trust chains, mTLS-authenticated origin pull, Cloudflare-style edge metadata, WAF and rate-limit assertions, Access admission and current-SHA Laravel health composition
  - Edge Security Emulation run 30270571670 passed on SHA 791c350b93406eabe50702f8860b0515678a80bb
  - artifact edge-security-emulation-evidence-30270571670 digest sha256:17ba33a26793a7f8d536acbcb78097e961a27bff65a690874fb72b884634e0b7 records all required outcomes and production_environment_proven false
  - Cloudflare Access admission remains independent from Platform auth confirmed MFA exact RBAC and audit
  - production-like evidence now composes the edge-emulation PASS while actual production DNS TLS Cloudflare WAF Access firewall and origin facts remain UNKNOWN
  - no production router DSM public DNS Cloudflare account secret or external-repository action occurred
derived:
  - the deterministic edge-security procedure and composition are complete at STAGING_PROVEN
  - the absence of a real provider environment no longer blocks repository-owned edge procedure validation
  - Issue 91 remains the sole production execution tracker
unknown: []
conflicts: []
first_failure:
  marker: http-redirect-target
  evidence: edge run 30268938970 first failed after Nginx and CoreDNS startup because the header assertion compared CRLF curl output without normalization; later corrections normalized captured headers, preserved the public host contract, accepted parser-level traversal denial and scoped rate-limit process waits
rejected_hypotheses:
  - DNS TLS mTLS or Laravel startup caused the first failure: those stages completed before the initial header assertion
  - HTTP 400 on a raw traversal path means the protection failed: Nginx rejected the request at its parser boundary before the WAF rule and the final harness accepts only controlled 400 or 403 denial
  - emulation can close Issue 91: actual production environment evidence remains mandatory
changed_paths:
  - .github/workflows/edge-security-emulation.yml
  - tests/edge-emulation/Corefile
  - tests/edge-emulation/db.oteryn.test
  - tests/edge-emulation/access_verifier.py
  - tests/edge-emulation/bin/curl
  - tests/edge-emulation/run.sh
  - docs/operations/EDGE_SECURITY_EMULATION_EVIDENCE.md
  - docs/operations/PRODUCTION_LIKE_VALIDATION_EVIDENCE.md
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260727-edge-security-emulation.md
validation:
  - command: repository and Issue 91 boundary review
    result: PASS
    evidence: production checklist topology baseline verification record and Access guidance preserve fail-closed production semantics
  - command: Edge Security Emulation 30268938970
    result: FAIL
    evidence: first unmet invariant http-redirect-target after successful application Nginx and DNS startup
  - command: Edge Security Emulation 30269692490
    result: FAIL
    evidence: public host preservation was correct but the test expected the internal origin hostname
  - command: Edge Security Emulation 30270031473
    result: FAIL
    evidence: raw traversal was denied with HTTP 400 at the Nginx parser boundary rather than HTTP 403 at the WAF rule
  - command: Edge Security Emulation 30270571670
    result: PASS
    evidence: exact SHA 791c350b93406eabe50702f8860b0515678a80bb and sanitized artifact digest sha256:17ba33a26793a7f8d536acbcb78097e961a27bff65a690874fb72b884634e0b7
blockers: []
next_action: Pass all required workflows on the final PR head, merge PR 236 and archive this completed task record.
```

## Notes

The emulation uses reserved test hostnames and ephemeral keys only. It proves the reviewed topology and controls under CI but does not imply ownership or configuration of a real DNS zone, certificate, Cloudflare account, firewall or production origin.
