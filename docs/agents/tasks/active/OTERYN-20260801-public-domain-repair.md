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
  - PR #387 public-domain validation report and findings
  - merged PRs #388, #392 and #396
  - Character Bazaar Staging Control run 30695167157 and artifact 8817085021
  - PR #399 public edge revalidation
  - Public Edge Revalidation run 30696983913 and artifact 8817569426
optional_reads:
  - PR #383
  - PR #385
  - PR #335
  - Issue #91
---

# OTERYN-20260801-public-domain-repair

## Goal

Repair the repository-owned public-domain defects proven by PR #387, deploy the exact repair through Marketplace-aware Synology staging, and retain a reversible operator plan for unavailable edge infrastructure without weakening security boundaries.

## Acceptance criteria

- [x] Requestless Platform URLs use `https://oteryn.molehill.cloud` while origins remain loopback-only.
- [x] Public staging rejects an unexpected full deployment `APP_URL`.
- [x] Partial Marketplace state loads without requiring deployment-only keys.
- [x] Marketplace Platform and scheduler use the canonical HTTPS origin and Secure cookies.
- [x] Health checks cover Gateway identity, malformed login, private cache controls, canonical URLs and negative cross-routing.
- [x] Protocol probes execute from the NAS host network namespace rather than the containerized runner loopback.
- [x] PR #396 exact head passed every applicable workflow.
- [x] Exact merge SHA `3eb109b505f7d1c8718cffb823de6d9d5166717c` was deployed and verified with sanitized `STAGING_PROVEN` evidence.
- [x] Cloudflare/DNS/Synology edge changes and rollback are documented without secrets.
- [x] Post-staging public DNS/TLS/HTTP behavior was independently revalidated and recorded.
- [ ] Authorized Cloudflare/DNS edge changes and public acceptance probes are complete.
- [x] `PRODUCTION_PROVEN` remains false until Issue #91 is completed.

## Ownership

```yaml
owned_paths:
  - deploy/synology/scripts/health-check.sh
  - tests/Feature/SynologyStagingNetworkBoundaryTest.php
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
  - docs/agents/reports/OTERYN-20260801-public-domain-repair.md
  - docs/agents/reports/OTERYN-20260801-public-edge-revalidation.md
modules:
  - public-web
  - identity
  - game-gateway
  - edge-transport
  - synology-staging
dependencies:
  - PR #387 source validation package
  - merged PR #388
  - merged PR #392
  - merged PR #396
  - Character Bazaar Staging Control
  - Issue #91 production go-live gate
blockers:
  - usable Cloudflare and DNS operator access for the documented edge plan and public acceptance probes
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-01T11:08:00Z
session_id: chatgpt-20260801-public-domain-repair-006
policy_version: 2
phase: external_edge_blocked_revalidated
execution_mode: chat-github-connector
repository_mutation_authorization: PROVEN
external_mutation_scope_authorization: PROVEN
external_operator_access: UNKNOWN
staging_deployment_authorization: PROVEN
context_pressure: low
decomposition_decision: continue_when_access_available
branch: docs/OTERYN-20260801-public-edge-revalidation
head: 1781536f5a4c288766cbb8b4651bf1be80bab8f8
pr: 400
status: blocked
context_routes:
  - agent-governance
  - security
  - auth-identity
  - api
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
  - docs/agents/reports/OTERYN-20260801-public-edge-revalidation.md
proven:
  - PR #388 merged as 82abef518f91d72d392db4420bb335773087c3e1 after all required exact-head workflows passed.
  - PR #392 merged as b249e5e9cb864ba01376efb273be323b90bcd500 after all path-applicable exact-head workflows passed.
  - PR #396 exact head b61cfc1ac2f5900d3ad9e78e2433bede8f7eec88 passed CI 4018, Governance 3809, Phase 7 3053, Images 1582, Edge 1474, DB 2980 and Concurrency 2551.
  - PR #396 merged as 3eb109b505f7d1c8718cffb823de6d9d5166717c.
  - Exact Platform and Gateway images tagged sha-3eb109b505f7d1c8718cffb823de6d9d5166717c were published and resolved.
  - Character Bazaar Staging Control run 30695167157 number 7 completed successfully on runner oteryn-synology-staging.
  - Run 30695167157 verified exact bindings, Gateway identity and version, malformed login cache controls, negative cross-routing, MFA QR behavior, canonical forwarded login action, canonical requestless URLs and Canary LAN reachability.
  - Character Bazaar enablement, transfer privilege boundary and final staging state persistence passed.
  - Sanitized artifact 8817085021 has digest sha256:5523ee4c0a49a156e23a894e808915a9a1f5b424b961168eb732774e6056efbb.
  - The artifact classifies exact source 3eb109b505f7d1c8718cffb823de6d9d5166717c as STAGING_PROVEN with Marketplace enabled, one scheduler and production_environment_proven false.
  - Public Edge Revalidation run 30696983913 number 1 completed from a West US GitHub-hosted runner at 2026-08-01T11:05:03.497039+00:00.
  - Sanitized public artifact 8817569426 has digest sha256:a090c5562ac2ed529f214fc5dd2d1f765b27facbb63a56f3838a46a4ba66c4a1.
  - Both canonical names still resolve to the same Cloudflare anycast addresses observed by PR #387.
  - WWW still verifies over TLS 1.3 only, presents molehill.cloud and wildcard SAN coverage, and returns Cloudflare 403 for every representative route.
  - Gateway still fails both TLS 1.2 and TLS 1.3 before HTTP and no certificate can be extracted.
  - Plain HTTP still returns Cloudflare 403 rather than redirecting to HTTPS.
  - WWW still emits Strict-Transport-Security max-age=0 with includeSubDomains and preload tokens.
  - Issue #91 comment 5151151647 records the post-staging public revalidation without promoting staging evidence to production proof.
  - No production, Cloudflare, DNS, Canary-source, OTClient or PR #387 evidence mutation occurred.
derived:
  - Repository-owned public-domain configuration and Synology staging deployment are complete.
  - The remaining public failures are entirely outside the deployed application and remain in the separately controlled Cloudflare/DNS edge.
  - Public launch and production proof remain blocked on operator access and the public acceptance sequence.
unknown:
  - Effective Cloudflare certificate attachment, WAF, Access, Bot, redirect and HSTS rule identifiers and configuration state.
  - Exact supported native-client minimum TLS version.
  - Controlled password-recovery delivery result through the public edge.
conflicts:
  - Staging is proven while the public edge is directly proven failing; STAGING_PROVEN must not be promoted to PUBLIC_DOMAIN_LAUNCH_READY or PRODUCTION_PROVEN.
first_failure:
  marker: gateway-public-tls-handshake-failure
  evidence: PR #387 runs 30690877286 and 30690957415 plus post-staging run 30696983913 failed TLS negotiation before HTTP
rejected_hypotheses:
  - The final Gateway image or host binding is not the current blocker; run 30695167157 proved both.
  - Canonical application URL generation is not the current blocker; run 30695167157 proved requestless and forwarded URLs.
  - Marketplace staging state and host-network health checks are not current blockers; final run and artifact passed.
  - The public edge was not repaired independently after staging deployment; run 30696983913 directly reproduced all material PR #387 failures.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
  - docs/agents/reports/OTERYN-20260801-public-edge-revalidation.md
validation:
  - command: path-applicable workflow suite on PR #396 exact head b61cfc1ac2f5900d3ad9e78e2433bede8f7eec88
    result: PASS
    evidence: CI 4018, Governance 3809, Phase 7 3053, Images 1582, Edge 1474, DB 2980 and Concurrency 2551
  - command: exact trusted-main image build for 3eb109b505f7d1c8718cffb823de6d9d5166717c
    result: PASS
    evidence: exact Platform and Gateway tags resolved before deployment
  - command: Character Bazaar Staging Control deploy-enable for 3eb109b505f7d1c8718cffb823de6d9d5166717c
    result: PASS
    evidence: run 30695167157 number 7 and artifact 8817085021
  - command: public DNS TLS HTTP redirect HSTS and route revalidation after staging deployment
    result: FAIL
    evidence: run 30696983913 number 1 and artifact 8817569426 reproduced Gateway TLS failure, WWW 403, no HTTP redirect and max-age=0 HSTS
  - command: authorized Cloudflare configuration mutation and controlled password-recovery acceptance
    result: NOT_RUN
    evidence: no usable Cloudflare or DNS operator connector exists in this session
deployment_evidence: STAGING_PROVEN artifact 8817085021 for exact source 3eb109b505f7d1c8718cffb823de6d9d5166717c; digest sha256:5523ee4c0a49a156e23a894e808915a9a1f5b424b961168eb732774e6056efbb; production_environment_proven false.
public_edge_evidence: Public Edge Revalidation artifact 8817569426; digest sha256:a090c5562ac2ed529f214fc5dd2d1f765b27facbb63a56f3838a46a4ba66c4a1; public acceptance failed without mutation.
rollback: Repository rollback is a revert of merged repair commits. Synology retains prior image snapshots for explicit guarded runtime rollback. External rollback restores the captured certificate, tunnel, WAF, Access, Bot, redirect and HSTS state.
blockers:
  - Cloudflare and DNS operator access is unavailable, so certificate coverage, edge policy, redirects, HSTS and password-recovery acceptance cannot be repaired or completed.
next_action: Provide usable Cloudflare and DNS operator access, capture current state, apply the report plan in reversible order, and rerun the exact public acceptance probes without changing production application code.
```

## Reports

- `docs/agents/reports/OTERYN-20260801-public-domain-repair.md`
- `docs/agents/reports/OTERYN-20260801-public-edge-revalidation.md`

## Notes

Repository and staging work are complete. The task remains active and blocked because the public edge is directly proven failing and `PUBLIC_DOMAIN_LAUNCH_READY` plus `PRODUCTION_PROVEN` remain false pending external edge work.
