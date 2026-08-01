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
- [x] Execute and independently corroborate sanitized public DNS, TLS and HTTP observations.
- [x] Remove the consumed temporary validation workflows.

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
  - canonical Gateway HTTPS fails TLS negotiation before HTTP
  - representative WWW routes are intercepted by Cloudflare 403 responses for anonymous automated clients
  - HTTP does not redirect to HTTPS before edge blocking
  - WWW edge returns HSTS max-age=0
  - controlled password-recovery delivery remains unavailable
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-01T08:06:49Z
session_id: chatgpt-20260801-public-domain-validator-001
policy_version: 2
phase: complete
execution_mode: chat-github-connector
execution_reason: repository inspection, exact staging evidence and two bounded GitHub-hosted public-edge observations completed the evidence-only task without implementation or deployment mutation
context_pressure: medium
decomposition_decision: phased
head: ad2f0f4d566c3294dfd8630a68ae6e337a02d6f2
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
  - Exact staging workflow 30669701842 used Platform and Gateway revision 6bfbc5f351758392d144baf0d2877a290ec69535 and Canary digest sha256:784e5dbdcc64e311c48c51cd94aa206e2efa1e5eefb2f4ef40170d5aac55031f.
  - Exact staging bindings were Platform 127.0.0.1:8000 and Gateway 127.0.0.1:8080.
  - Exact staging probes passed Platform health, Gateway health/readiness/version, Canary health and forwarded canonical HTTPS login-form generation.
  - Gateway source exposes GET /health, GET /ready, GET /version and POST /v1/login only.
  - Gateway /v1/login source enforces bounded JSON input, sanitized errors and no-store/no-cache response headers.
  - Platform source uses explicit proxy trust, secure HttpOnly SameSite=Lax host-only session-cookie configuration in the staging overlay, web security headers and no-store policies on sensitive authentication APIs.
  - Password reset uses the standard Laravel broker; email-change notifications use named routes; Wiki previews use temporary signed routes.
  - Public observation run 30690877286 succeeded on head 19e62011f5920c89d22aa70738b3ea66ab61ef20 and produced artifact 8815612315 with ZIP digest sha256:174ff9dd5c1a098a49277926aca100b41f7a3761e7e67595f98b92097c7ea909.
  - Independent corroboration run 30690957415 succeeded on head b66012b086f03b2cf70f1c194cb4c72593bc2426 and produced artifact 8815638539 with ZIP digest sha256:b5b3effb61e350c4a5fd59ff2949c9f38f265b42f3c81787bf894745d738a1d8.
  - Both canonical names resolved through Cloudflare to IPv4 104.21.2.166 and 172.67.186.250 and IPv6 2606:4700:3031::6815:2a6 and 2606:4700:3033::ac43:bafa during the observations.
  - oteryn.molehill.cloud negotiated TLS 1.3 with a valid certificate covering molehill.cloud and *.molehill.cloud; forced TLS 1.2 was rejected.
  - Representative WWW HTTP and HTTPS requests returned Cloudflare 403 responses in both observations with validator and Chrome-like user agents.
  - WWW HTTP did not redirect to HTTPS before Cloudflare blocking.
  - WWW HTTPS returned Strict-Transport-Security max-age=0 with includeSubDomains and preload tokens.
  - login.oteryn.molehill.cloud failed TLS 1.2 and TLS 1.3 handshakes before HTTP in both observations and with independent Python/OpenSSL and curl clients.
  - Gateway /health, /ready, /version and /v1/login were therefore not externally usable through the canonical HTTPS hostname during the observations.
  - No secret, credential, token, cookie value, valid Game Login Ticket or state-changing request was used.
  - The two temporary public-domain evidence workflows were removed after artifact capture.
derived:
  - The Gateway TLS failure is consistent with the observed wildcard certificate not covering the two-label hostname login.oteryn.molehill.cloud; exact Cloudflare certificate configuration was not directly inspected.
  - Request-bound absolute URLs preserve the public HTTPS origin when requests traverse the explicitly trusted proxy path; this was directly proven only for the exact staging login form.
  - No path ownership conflict prevents this evidence PR.
unknown:
  - Exact currently externally deployed Platform SHA.
  - Exact currently externally deployed Gateway and Canary versions.
  - Interactive human-browser behavior after Cloudflare JavaScript or managed challenge completion.
  - Effective public Gateway application behavior after TLS and edge routing are repaired.
  - Password-recovery sender, delivered link hostname/scheme and completion behavior.
  - Current native-client end-to-end behavior through login.oteryn.molehill.cloud.
conflicts:
  - Exact staging APP_URL was http://127.0.0.1:8000 while the canonical public application URL is https://oteryn.molehill.cloud.
first_failure:
  marker: gateway-public-tls-handshake-failure
  evidence: runs 30690877286 and 30690957415 reproduced TLS handshake failure for login.oteryn.molehill.cloud with TLS 1.2 and TLS 1.3 before any HTTP response
rejected_hypotheses:
  - Local sandbox DNS failure was not accepted as evidence that the public names were absent; GitHub-hosted probes resolved both names.
  - A Gateway root 404 is not treated as a defect because no root endpoint is contracted.
  - Historical staging evidence is not promoted to current production evidence.
  - Cloudflare numeric codes heuristically matched in challenge HTML are not reported as actual Cloudflare error codes.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-validation.md
  - docs/agents/reports/OTERYN-20260801-public-domain-validation.md
validation:
  - command: repository, task, branch, PR and ownership preflight
    result: PASS
    evidence: task/branch/PR initially absent; task-start main 7dac56d3f3f4606be958c875f278edbe410e6b54; no final owned-path overlap
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
    evidence: exact staging identities, bindings, health and forwarded HTTPS login action recorded
  - command: Public Domain Evidence run 30690877286 job 91345253565
    result: PASS
    evidence: sanitized DNS, certificate, headers, redirect, representative WWW and Gateway observations in artifact 8815612315
  - command: Public Domain Corroboration run 30690957415 job 91345468758
    result: PASS
    evidence: independent region, Chrome-like user agent, curl and forced TLS 1.2/1.3 reproduced the public failures in artifact 8815638539
  - command: canonical Gateway HTTPS TLS negotiation
    result: FAIL
    evidence: standards-compliant TLS 1.2 and TLS 1.3 clients received handshake failure before HTTP
  - command: representative WWW anonymous automated reachability
    result: FAIL
    evidence: Cloudflare 403 for all tested representative routes in both observations
  - command: public HTTP-to-HTTPS redirect
    result: FAIL
    evidence: both hostnames returned Cloudflare 403 over HTTP without redirect
  - command: WWW HSTS enforcement
    result: FAIL
    evidence: max-age=0 response disables persisted HSTS
  - command: controlled login/logout and password-recovery mailbox validation
    result: BLOCKED
    evidence: no controlled public identity/mailbox was available and WWW entry routes were intercepted by Cloudflare
  - command: temporary evidence workflow cleanup
    result: PASS
    evidence: both one-shot workflow files removed after durable artifacts were created
blockers:
  - Public-domain launch is blocked by Gateway TLS failure, WWW Cloudflare interception, missing HTTP-to-HTTPS redirect, disabled HSTS, canonical APP_URL conflict and unproven password-recovery delivery.
next_action: Repair the exact Gateway certificate/hostname configuration and public routing, define the intended WWW Cloudflare access policy, place HTTPS redirect before blocking, enable positive reviewed HSTS after all included subdomains have valid TLS, configure canonical HTTPS requestless URL generation, then repeat the sanitized public probes and one controlled redacted password-recovery flow before executing Issue #91.
```

## Report

`docs/agents/reports/OTERYN-20260801-public-domain-validation.md`

## Notes

The evidence-collection task is complete, but public-domain launch acceptance failed. Implementation, deployment, Cloudflare, DNS, Synology, credential, mailbox and production mutations were not performed. `PRODUCTION_PROVEN` remains false.
