---
task_id: OTERYN-20260801-public-domain-validation
required_reads:
  - AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - deploy/synology/PUBLIC_ENDPOINTS.md
  - docs/testing/PRODUCTION_SMOKE_CHECKLIST.md
  - docs/operations/PRODUCTION_READINESS_CHECKLIST.md
  - docs/agents/tasks/archive/OTERYN-20260801-public-https-proxy.md
search_first:
  - public hostname APP_URL proxy cookie cache generated URL
  - Game Gateway routes health readiness version login session
  - open tasks and PRs owning audit evidence or Synology deployment paths
optional_reads:
  - PR #381
  - PR #383
  - PR #385
  - Issue #91
---

# OTERYN-20260801-public-domain-validation

## Goal

Produce one sanitized evidence package that classifies launch-relevant public-hostname, HTTPS, reverse-proxy, generated-URL, cookie, caching and Game Gateway behavior for `https://oteryn.molehill.cloud` and `https://login.oteryn.molehill.cloud` without changing application implementation, deployment or external infrastructure.

## Acceptance criteria

- [x] Record the current repository head and observed deployment identity without unsupported inference.
- [x] Verify or explicitly classify both canonical hostname roles.
- [x] Collect bounded direct or durable evidence for representative WWW hostname behavior.
- [x] Discover actual Gateway endpoint contracts and collect bounded evidence or an exact blocker.
- [x] Classify generated URLs, cookie attributes, sensitive-response caching and proxy HTTPS behavior.
- [x] Prove password-recovery hostname/delivery or record the exact blocker.
- [x] Record every finding with severity, confidence, evidence, impact, ownership and recommendation.
- [x] Persist a compact final report and one draft evidence PR.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-validation.md
  - docs/agents/reports/OTERYN-20260801-public-domain-validation.md
modules:
  - public-web
  - identity
  - game-gateway
  - edge-transport
dependencies:
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - PR #381 route and surface inventory
  - PR #383 and PR #385 public HTTPS proxy evidence
  - Issue #91 production go-live gate
blockers:
  - current external DNS, TLS and HTTP behavior cannot be observed from the execution environment
  - no controlled public identity and mailbox are available for password-recovery delivery validation
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 2
updated_at: 2026-08-01T07:32:07Z
session_id: chatgpt-20260801-public-domain-validator-001
policy_version: 2
phase: discovery_and_evidence
execution_mode: chat-github-connector
execution_reason: repository inspection, bounded documentation writes and existing workflow evidence were sufficient; implementation and deployment were not authorized
context_pressure: medium
decomposition_decision: phased
head: 41e1c4ab8d2a46fe4f477bba6d8324b315ad9eb2
branch: audit/OTERYN-20260801-public-domain-validation
pr: 387
status: blocked
context_routes:
  - agent-governance
  - security
  - auth-identity
  - api
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-validation.md
  - docs/agents/reports/OTERYN-20260801-public-domain-validation.md
proven:
  - Task-start main was 7dac56d3f3f4606be958c875f278edbe410e6b54.
  - Canonical repository contracts map oteryn.molehill.cloud to Platform WWW on Synology loopback port 8000 and login.oteryn.molehill.cloud to Game Gateway on loopback port 8080.
  - PR #381 provides a reusable 27-surface and 228-named-route portal inventory without owning this task's paths.
  - PR #335 owns a distinct Synology boot-repair path and does not conflict with this documentation-only task.
  - Exact staging workflow 30669701842 used Platform and Gateway revision 6bfbc5f351758392d144baf0d2877a290ec69535 and Canary digest sha256:784e5dbdcc64e311c48c51cd94aa206e2efa1e5eefb2f4ef40170d5aac55031f.
  - Exact staging bindings were Platform 127.0.0.1:8000 and Gateway 127.0.0.1:8080.
  - Exact staging probes passed Platform health, Gateway health/readiness/version, Canary health and forwarded canonical HTTPS login-form generation.
  - Gateway source exposes GET /health, GET /ready, GET /version and POST /v1/login only.
  - Gateway /v1/login source enforces bounded JSON input, sanitized errors and no-store/no-cache response headers.
  - Platform source uses explicit proxy trust, secure HttpOnly SameSite=Lax host-only session-cookie configuration in the staging overlay, web security headers and no-store policies on sensitive authentication APIs.
  - Password reset uses the standard Laravel broker; email-change notifications use named routes; Wiki previews use temporary signed routes.
  - Sanitized workflow artifact 8808580115 has ZIP digest sha256:f5ea1efb02b8508d3b54765c2e7d15551dfab9d44c6a6c80ea3a299b970c0d44 and payload digest sha256:2b94d392f97d2afa179ce32ec618f11b61c0bb38829a4ca8637efb6e6b84ab6d.
derived:
  - Request-bound absolute URLs are expected to preserve the public HTTPS origin when the request traverses the explicitly trusted proxy path; this was directly proven only for the staging login form.
  - No path ownership conflict prevents this evidence PR.
unknown:
  - Exact currently externally deployed Platform SHA.
  - Exact currently externally deployed Gateway and Canary versions.
  - Current public DNS records, TLS certificates, redirects, response headers, cookies, cache status and cross-routing for both hostnames.
  - Current public reachability of representative WWW routes and Gateway routes.
  - Effective Cloudflare or reverse-proxy rate limiting for Gateway POST /v1/login.
  - Password-recovery sender, delivered link hostname/scheme and completion behavior.
  - Current native-client end-to-end behavior through login.oteryn.molehill.cloud.
conflicts:
  - Exact staging APP_URL was http://127.0.0.1:8000 while the canonical public application URL is https://oteryn.molehill.cloud.
first_failure:
  marker: external-probe-unavailable
  evidence: web fetches failed; local resolver could not resolve either canonical hostname; direct public DNS/TLS/HTTP evidence could not be collected
rejected_hypotheses:
  - Prompt-authoring context and image tags are not accepted as current deployment identity.
  - A Gateway root 404 is not treated as a defect because no root endpoint is contracted.
  - Historical staging evidence is not promoted to current production evidence.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-validation.md
  - docs/agents/reports/OTERYN-20260801-public-domain-validation.md
validation:
  - command: repository, task, branch, PR and ownership preflight
    result: PASS
    evidence: task/branch/PR initially absent; main 7dac56d3f3f4606be958c875f278edbe410e6b54; no owned-path overlap
  - command: required durable reads and PR #381/#383/#385 plus Issue #91 inspection
    result: PASS
    evidence: bounded contracts, predecessor evidence and overlapping work reviewed
  - command: source discovery for proxy, URL, cookie, cache, security-header, password recovery and signed-preview behavior
    result: PASS
    evidence: effective source contracts recorded in the report
  - command: source discovery for Gateway public routes, bounded errors, no-store headers and dependency TLS policy
    result: PASS
    evidence: four public endpoints and response behavior recorded in the report
  - command: workflow 30669701842 log and artifact 8808580115 inspection
    result: PASS
    evidence: exact staging identities, bindings, health and forwarded HTTPS login action recorded; artifact checksums verified
  - command: direct current public DNS, TLS, HTTP and browser probes for both canonical hostnames
    result: BLOCKED
    evidence: execution environment could not resolve or fetch the public hostnames
  - command: controlled login/logout and password-recovery mailbox validation
    result: BLOCKED
    evidence: no controlled public identity/mailbox was available
  - command: initial PR head d4c7d4490ba3aea0a9e85cd990aca8a4a0fbc7b0 required workflows
    result: PASS
    evidence: Agent Governance, CI, Edge Security Emulation, Game Auth Ticket Concurrency, Platform DB Outage Validation and Phase 7 Production-Like Validation completed successfully
blockers:
  - Current public-edge behavior and exact external deployment identity require an Internet-capable trusted probe; password-recovery delivery additionally requires a controlled identity and mailbox.
next_action: Run one sanitized read-only external probe of both canonical domains from a trusted Internet-capable runner, including redacted controlled-mailbox password recovery, and bind the results to exact Platform, Gateway and Canary identities.
```

## Report

`docs/agents/reports/OTERYN-20260801-public-domain-validation.md`

## Notes

Implementation, deployment, production mutation and external infrastructure changes were not performed. Large logs, screenshots, traces and binaries remain outside Git. The report explicitly records `PRODUCTION_PROVEN: false`.
