---
task_id: OTERYN-20260801-public-https-proxy
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - deploy/synology/PUBLIC_ENDPOINTS.md
search_first:
  - deploy/synology trusted proxy APP_URL secure cookie
  - open PRs touching deploy/synology
optional_reads: []
---

# OTERYN-20260801-public-https-proxy

## Goal

Make the owner-designated public Platform hostname generate and submit HTTPS login forms correctly through the Synology-hosted Cloudflare Tunnel while preserving loopback-only origin bindings and fail-closed proxy trust.

## Acceptance criteria

- [x] The Synology Platform image derives exactly one RFC1918 Docker default gateway and exports it to Laravel as `TRUSTED_PROXIES`; wildcard and broad CIDR trust are rejected.
- [x] The Docker image build executes deterministic gateway-decoding and private-address self-tests.
- [x] Marketplace staging overrides the historical insecure cookie setting with `SESSION_SECURE_COOKIE=true`.
- [x] The deployment health check sends the canonical forwarded host/proto/port from the Docker host network and requires `https://oteryn.molehill.cloud/login?locale=en` as the rendered form action.
- [x] All required exact-head checks passed before merge.
- [x] Marketplace-aware Synology staging deployment proved the public HTTPS form-action check on the exact merged image.
- [x] Temporary read-only observer PR #384 was closed without merge after evidence capture.

## Ownership

```yaml
owned_paths:
  - deploy/synology/compose.marketplace.yml
  - deploy/synology/docker/platform-entrypoint.sh
  - deploy/synology/docker/platform.Dockerfile
  - deploy/synology/scripts/health-check.sh
  - docs/agents/tasks/archive/OTERYN-20260801-public-https-proxy.md
modules:
  - deployment
  - security
  - identity
  - public-web
dependencies:
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - PR #133 trusted reverse-proxy middleware
  - Character Bazaar Staging Control
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-31T22:40:00Z
head: 6bfbc5f351758392d144baf0d2877a290ec69535
branch: docs/OTERYN-20260801-public-https-proxy-closeout
pr: 383
status: ready
context_routes:
  - security
  - auth-identity
  - testing
  - ci-repair
owned_paths:
  - deploy/synology/compose.marketplace.yml
  - deploy/synology/docker/platform-entrypoint.sh
  - deploy/synology/docker/platform.Dockerfile
  - deploy/synology/scripts/health-check.sh
  - docs/agents/tasks/archive/OTERYN-20260801-public-https-proxy.md
proven:
  - The canonical public route remains HTTPS oteryn.molehill.cloud to host loopback HTTP 127.0.0.1:8000.
  - Laravel trusts only the exact RFC1918 Docker default gateway derived inside the Platform container; wildcard and broad CIDR trust remain forbidden.
  - The exact Platform image executes a deterministic entrypoint self-test during build.
  - Marketplace staging forces SESSION_SECURE_COOKIE=true on the final browser-facing Platform container.
  - PR #383 exact head 1b3bc11228e84f092b84895b5963b16716748c77 passed all eight required workflow families.
  - PR #383 squash-merged as 6bfbc5f351758392d144baf0d2877a290ec69535 with the guarded Character Bazaar staging marker.
  - Trusted-main image build run 30669701871 succeeded for exact merge SHA 6bfbc5f351758392d144baf0d2877a290ec69535.
  - Marketplace-aware staging control run 30669701842 completed deploy-enable successfully on runner oteryn-synology-staging.
  - The deployment log contains `Verified public HTTPS login form action through the host-loopback proxy boundary.`
  - Platform, Gateway, Canary, public HTTPS login, MFA QR and the LAN game endpoint staging probes passed.
  - Character Bazaar staging enablement passed with Marketplace enabled and exactly one scheduler.
  - Sanitized artifact 8808580115 has digest sha256:f5ea1efb02b8508d3b54765c2e7d15551dfab9d44c6a6c80ea3a299b970c0d44 and classification STAGING_PROVEN.
  - Temporary observer PR #384 closed without merge and performed no dispatch or Synology mutation.
derived:
  - The original browser no-op was caused by an untrusted forwarded HTTPS origin producing a blocked HTTP form action; the exact live staging probe now proves the corrected HTTPS action generation.
unknown:
  - End-user credential validity is not inferred from infrastructure evidence; a normal fresh-session login remains the final user interaction check.
  - Production remains unproven.
conflicts:
  - Open PR #335 still owns its distinct Synology boot-repair script; this task did not modify that path.
first_failure:
  marker: public-login-submit-no-op
  evidence: owner report against https://oteryn.molehill.cloud/login?locale=en plus previous deployment configuration
rejected_hypotheses:
  - The locale query parameter was not causal; it only selected localization.
  - A JavaScript handler was not causal; the login form uses native HTML POST.
  - Trusting all Cloudflare ranges or a private subnet was unnecessary and broader than the loopback-origin boundary.
changed_paths:
  - deploy/synology/compose.marketplace.yml
  - deploy/synology/docker/platform-entrypoint.sh
  - deploy/synology/docker/platform.Dockerfile
  - deploy/synology/scripts/health-check.sh
  - docs/agents/tasks/archive/OTERYN-20260801-public-https-proxy.md
validation:
  - command: Agent Governance run 30669521657
    result: PASS
    evidence: exact PR head 1b3bc11228e84f092b84895b5963b16716748c77
  - command: Character Bazaar Staging Validation run 30669521634
    result: PASS
    evidence: exact PR head 1b3bc11228e84f092b84895b5963b16716748c77
  - command: Edge Security Emulation run 30669521648
    result: PASS
    evidence: exact PR head 1b3bc11228e84f092b84895b5963b16716748c77
  - command: Game Auth Ticket Concurrency run 30669521651
    result: PASS
    evidence: exact PR head 1b3bc11228e84f092b84895b5963b16716748c77
  - command: Platform DB Outage Validation run 30669521636
    result: PASS
    evidence: exact PR head 1b3bc11228e84f092b84895b5963b16716748c77
  - command: Build Synology Staging Images run 30669521641
    result: PASS
    evidence: exact PR head; deployment package and Platform image self-test passed
  - command: Phase 7 Production-Like Validation run 30669521667
    result: PASS
    evidence: exact PR head 1b3bc11228e84f092b84895b5963b16716748c77
  - command: CI run 30669521639
    result: PASS
    evidence: exact PR head 1b3bc11228e84f092b84895b5963b16716748c77
  - command: trusted-main Build Synology Staging Images run 30669701871
    result: PASS
    evidence: exact merge SHA images published
  - command: Character Bazaar Staging Control run 30669701842
    result: PASS
    evidence: exact deploy-enable, HTTPS login action probe and Marketplace re-enable
blockers:
  - none
next_action: Retest the public login with a fresh browser session; open a separate focused identity task only if valid credentials still fail after cookies are cleared.
```

## Notes

The public origins remain loopback-only. This task did not trust `*`, a whole private subnet, Cloudflare public ranges or direct untrusted clients. Evidence is `STAGING_PROVEN`; `production_environment_proven` remains false.
