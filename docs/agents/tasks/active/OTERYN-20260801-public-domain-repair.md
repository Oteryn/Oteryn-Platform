---
task_id: OTERYN-20260801-public-domain-repair
required_reads:
  - AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - deploy/synology/PUBLIC_ENDPOINTS.md
  - docs/operations/PRODUCTION_READINESS_CHECKLIST.md
search_first:
  - PR #387 public-domain validation
  - merged PRs #388, #392 and #396
  - staging run 30695167157 and artifact 8817085021
  - Cloudflare endpoint audit 30699270139 and apply 30700054602
  - PR #407 post-Cloudflare public validation
  - validation run 30701999967 and artifact 8819120004
optional_reads:
  - PR #401
  - PR #402
  - PR #403
  - Issue #91
---

# OTERYN-20260801-public-domain-repair

## Goal

Repair repository-owned public-domain defects, deploy the exact repair through Marketplace-aware Synology staging, reconcile the canonical Cloudflare Tunnel and DNS entries, and prove the remaining public edge state without weakening security boundaries.

## Acceptance criteria

- [x] Requestless Platform URLs use `https://oteryn.molehill.cloud` while origins remain loopback-only.
- [x] Marketplace Platform and scheduler use the canonical HTTPS origin and Secure cookies.
- [x] Health checks cover Gateway identity, malformed login, private cache controls, canonical URLs and negative cross-routing.
- [x] Exact source `3eb109b505f7d1c8718cffb823de6d9d5166717c` is `STAGING_PROVEN`.
- [x] Canonical Cloudflare Tunnel entries and both DNS records were audited and reconciled.
- [x] A fresh public validation ran after the Cloudflare endpoint apply.
- [ ] Gateway canonical hostname presents usable hostname-verified TLS.
- [ ] Intended public WWW routes are reachable without blanket Cloudflare interstitials.
- [ ] Plain HTTP redirects to canonical HTTPS before challenge/block processing.
- [ ] HSTS has a reviewed effective state after all included hostnames have valid TLS.
- [ ] Controlled password-recovery delivery is proven through the public edge.
- [x] `PRODUCTION_PROVEN` remains false until Issue #91 is completed.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
  - docs/agents/reports/OTERYN-20260801-public-domain-repair.md
  - docs/agents/reports/OTERYN-20260801-public-edge-revalidation.md
  - docs/agents/reports/OTERYN-20260801-public-edge-post-cloudflare-validation.md
modules:
  - public-web
  - identity
  - game-gateway
  - edge-transport
  - synology-staging
dependencies:
  - PR #387 source validation package
  - merged PRs #388, #392 and #396
  - Character Bazaar Staging Control
  - merged Cloudflare automation PRs #401 and #402
  - Issue #91 production go-live gate
blockers:
  - certificate, challenge-policy, redirect and HSTS controls are outside the completed Tunnel/DNS reconciler
cross_repository_tasks:
  - OTERYN-20260801-cloudflare-edge-policy-automation
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-01T13:40:00Z
session_id: chatgpt-20260801-public-domain-repair-007
session_role: validator
policy_version: 2
phase: post_cloudflare_validation_failed
execution_mode: chat-github-connector
execution_reason: live GitHub state, one-shot runner validation and artifact review were sufficient
context_pressure: low
context_growth: stable
decomposition_decision: split
branch: audit/OTERYN-20260801-public-domain-post-cloudflare-validation
head: d1b24ea763351c57602a5862843b352b96666b3d
pr: 407
status: blocked
context_routes:
  - agent-governance
  - security
  - auth-identity
  - api
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
  - docs/agents/reports/OTERYN-20260801-public-edge-post-cloudflare-validation.md
proven:
  - PR #388 merged as 82abef518f91d72d392db4420bb335773087c3e1, PR #392 as b249e5e9cb864ba01376efb273be323b90bcd500 and PR #396 as 3eb109b505f7d1c8718cffb823de6d9d5166717c.
  - Staging run 30695167157 and artifact 8817085021 prove exact source 3eb109b505f7d1c8718cffb823de6d9d5166717c, canonical URLs, Gateway identity, cache controls and negative cross-routing on Synology staging.
  - Cloudflare audit run 30699270139 reported tunnel drift with both DNS records current and no mutation.
  - Explicitly authorized Cloudflare apply run 30700054602 reported tunnel current, both DNS records current and mutation limited to the tunnel, with built-in post-apply verification.
  - Post-apply validation run 30701999967 job 91374523427 completed from a West US GitHub-hosted runner.
  - Sanitized artifact 8819120004 has digest sha256:0d776dca5fd73d5faf05c971aaa51cdbeb8aa498883b08873c12c6d07e843579.
  - Both canonical names resolve through Cloudflare to the expected anycast addresses.
  - WWW verifies over TLS 1.3 but all representative routes still return Cloudflare 403 interstitials for machine and Chrome-like user agents.
  - Gateway still fails TLS 1.2 and TLS 1.3 before HTTP and no certificate is extractable.
  - Plain HTTP on both hosts still returns Cloudflare 403 rather than redirecting to HTTPS.
  - WWW still emits Strict-Transport-Security max-age=0 with includeSubDomains and preload tokens.
  - The consumed one-shot workflow and probe were removed after artifact capture.
derived:
  - Repository, Synology staging, Tunnel ingress and canonical DNS are not the remaining blockers.
  - Remaining public failures belong to certificate coverage and Cloudflare edge-policy controls not managed by the endpoint reconciler.
unknown:
  - Exact Cloudflare certificate attachment/product and issuance state for login.oteryn.molehill.cloud.
  - Exact WAF, Bot, Access, redirect and HSTS rule identifiers affecting the two canonical hosts.
  - Exact supported native-client minimum TLS version.
  - Controlled password-recovery delivery result through the public edge.
conflicts:
  - Staging and Tunnel/DNS are proven current while the public edge is directly proven failing; neither may be promoted to PUBLIC_DOMAIN_LAUNCH_READY or PRODUCTION_PROVEN.
first_failure:
  marker: gateway-public-tls-handshake-failure
  evidence: PR #387 observations, run 30696983913 and fresh post-apply run 30701999967 all fail before Gateway HTTP
rejected_hypotheses:
  - Tunnel hostname-to-origin drift was the sole public blocker: run 30700054602 repaired it, but run 30701999967 reproduced every material public failure.
  - DNS was the blocker: audit and apply both reported both canonical DNS records current.
  - Final images, host bindings or canonical URL generation were the blocker: staging run 30695167157 passed all three boundaries.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
  - docs/agents/reports/OTERYN-20260801-public-edge-post-cloudflare-validation.md
validation:
  - command: Cloudflare Oteryn Endpoints audit run 30699270139
    result: PASS
    evidence: tunnel=drift, dns_www=current, dns_login=current, mutation=none
  - command: Cloudflare Oteryn Endpoints apply run 30700054602
    result: PASS
    evidence: tunnel=current, dns_www=current, dns_login=current, mutation=tunnel with post-apply verification
  - command: Public Edge Post-Cloudflare Validation run 30701999967 job 91374523427
    result: FAIL
    evidence: artifact 8819120004; Gateway TLS failure, WWW 403, no HTTP redirects and HSTS max-age=0
  - command: temporary validation workflow cleanup
    result: PASS
    evidence: one-shot workflow and probe removed after artifact capture
validation_level: focused
heavy_validation_runs: 0
deployment_evidence: STAGING_PROVEN artifact 8817085021 for exact source 3eb109b505f7d1c8718cffb823de6d9d5166717c; production_environment_proven false.
public_edge_evidence: artifact 8819120004; digest sha256:0d776dca5fd73d5faf05c971aaa51cdbeb8aa498883b08873c12c6d07e843579; public acceptance failed after endpoint apply.
rollback: Repository rollback is a revert of merged repair commits. Synology retains prior image snapshots. Cloudflare endpoint automation preserved unrelated ingress and DNS state; future edge-policy automation must capture and restore exact affected rule state.
blockers:
  - no guarded automation currently exists for certificate coverage, hostname-scoped challenge policy, HTTP redirect ordering or HSTS
next_action: Create task OTERYN-20260801-cloudflare-edge-policy-automation with audit-first, fail-closed ownership of certificate, challenge, redirect and HSTS controls for only the two canonical Oteryn hosts.
```

## Reports

- `docs/agents/reports/OTERYN-20260801-public-domain-repair.md`
- `docs/agents/reports/OTERYN-20260801-public-edge-revalidation.md`
- `docs/agents/reports/OTERYN-20260801-public-edge-post-cloudflare-validation.md`

## Notes

Repository repair, Synology staging, Tunnel routing and DNS reconciliation are complete. Public launch remains blocked on separately controlled Cloudflare certificate and edge-policy work. PR #407 is evidence-only and must not be merged with temporary workflow files.
