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
  - Issue 91 production boundary
  - exact edge-emulation PASS evidence
optional_reads: []
---

# OTERYN-20260727-edge-security-emulation

## Goal

Add a deterministic repository-owned production-like edge profile that emulates DNS, public TLS termination, Cloudflare-style proxy metadata, WAF/rate limiting, optional Access denial and authenticated origin pulls without claiming that any actual production provider or hostname exists.

## Acceptance criteria

- [x] Authoritative DNS A/CNAME and NXDOMAIN behavior are proven with reserved `.test` names.
- [x] Public edge hostname verification, TLS 1.2/1.3 and HTTP-to-HTTPS redirect are proven.
- [x] Edge-to-origin TLS uses a separate trust chain and authenticated client certificate.
- [x] Direct origin access without the edge certificate fails closed.
- [x] Forwarded-client normalization and Cloudflare-style metadata are proven.
- [x] Traversal, XSS, SQL injection, unsupported methods and oversized bodies are denied.
- [x] Controlled burst traffic produces HTTP 429.
- [x] Missing or invalid Access assertions are denied while Platform auth/MFA/RBAC remains authoritative after admission.
- [x] Current-SHA Laravel `/health` succeeds through the complete emulated path.
- [x] Sanitized exact-SHA evidence is recorded as `STAGING_PROVEN` only.
- [x] Actual production DNS, TLS, Cloudflare/WAF/Access, firewall and origin facts remain `UNKNOWN` under Issue #91.
- [x] No production, router, DSM, public DNS, Cloudflare account, secret or external-repository action occurred.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20260727-edge-security-emulation.md
modules:
  - Deployment
  - Security
  - Testing
  - AgentGovernance
dependencies:
  - open Issue 91 production boundary
  - PR 236 merge ee8293d8bbf33c9bc89ca105a0273728bb222f4d
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T15:44:00+02:00
head: ee8293d8bbf33c9bc89ca105a0273728bb222f4d
branch: docs/OTERYN-20260727-edge-security-archive
pr: none
status: ready
context_routes:
  - agent-governance
  - deployment
  - security
  - testing
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20260727-edge-security-emulation.md
proven:
  - PR 236 merged as ee8293d8bbf33c9bc89ca105a0273728bb222f4d after all six required exact-head workflows passed
  - Edge Security Emulation run 30270571670 passed on implementation SHA 791c350b93406eabe50702f8860b0515678a80bb
  - artifact edge-security-emulation-evidence-30270571670 digest sha256:17ba33a26793a7f8d536acbcb78097e961a27bff65a690874fb72b884634e0b7 records all required outcomes with production_environment_proven false
  - final PR head 95b85e7e02a32198f7221745ac47588c28dd12f6 passed Edge Security 30270949292, Governance 30270950325, CI 30270949846, concurrency 30270949252, DB outage 30270949376 and Phase 7 30270950269
  - actual production DNS TLS Cloudflare WAF Access firewall certificate lifecycle and origin exposure remain UNKNOWN and tracked only by Issue 91
  - no production router DSM public DNS Cloudflare account secret or external-repository action occurred
derived:
  - deterministic edge procedure and configuration composition are complete at STAGING_PROVEN
  - repository-owned validation no longer depends on the existence of the future production provider environment
  - Issue 91 remains the sole production execution tracker
unknown: []
conflicts: []
first_failure:
  marker: http-redirect-target
  evidence: run 30268938970 first failed on CRLF-sensitive header comparison; later corrections normalized headers, preserved the public host contract, accepted parser-level traversal denial and scoped rate-limit waits before exact PASS
rejected_hypotheses:
  - DNS TLS mTLS or Laravel startup caused the first failure: those stages completed before the initial header assertion
  - HTTP 400 traversal denial means protection failed: the Nginx parser rejected the raw request before the WAF rule and the final harness accepts only controlled 400 or 403 denial
  - emulation closes Issue 91: direct real-environment evidence remains mandatory
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
  - docs/agents/tasks/archive/OTERYN-20260727-edge-security-emulation.md
validation:
  - command: Edge Security Emulation 30270571670
    result: PASS
    evidence: implementation SHA 791c350b93406eabe50702f8860b0515678a80bb and artifact digest sha256:17ba33a26793a7f8d536acbcb78097e961a27bff65a690874fb72b884634e0b7
  - command: PR 236 final exact-head required workflows
    result: PASS
    evidence: Edge Security 30270949292, Governance 30270950325, CI 30270949846, concurrency 30270949252, DB outage 30270949376 and Phase 7 30270950269 on 95b85e7e02a32198f7221745ac47588c28dd12f6
  - command: PR 236 merge
    result: PASS
    evidence: squash merge ee8293d8bbf33c9bc89ca105a0273728bb222f4d
blockers: []
next_action: Preserve this archived record as completion evidence.
```

## Notes

The emulation proves the reviewed controlled topology and procedures only. It does not imply ownership or effective configuration of any real DNS zone, certificate, Cloudflare account, firewall or production origin.
