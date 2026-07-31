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
- [ ] Required exact-head checks pass before merge.
- [ ] Marketplace-aware Synology staging deployment proves the public HTTPS form-action check on the exact merged image.

## Ownership

```yaml
owned_paths:
  - deploy/synology/compose.marketplace.yml
  - deploy/synology/docker/platform-entrypoint.sh
  - deploy/synology/docker/platform.Dockerfile
  - deploy/synology/scripts/health-check.sh
  - docs/agents/tasks/active/OTERYN-20260801-public-https-proxy.md
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
updated_at: 2026-07-31T22:20:00Z
head: c91d9d5670fae811f9ba45fe43420f15149c0c0b
branch: fix/OTERYN-20260801-public-https-proxy
pr: 383
status: validating
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
  - docs/agents/tasks/active/OTERYN-20260801-public-https-proxy.md
proven:
  - The canonical public route is HTTPS oteryn.molehill.cloud to host loopback HTTP 127.0.0.1:8000.
  - Laravel already trusts only explicitly configured IP/CIDR proxy entries and rejects wildcard trust.
  - The previous Synology runtime passed neither a trusted proxy address nor a secure session-cookie override to the final Marketplace-enabled Platform container.
  - The login page is a native POST form whose absolute action depends on trusted forwarded scheme and host metadata.
  - PR 383 derives the exact container default gateway at process start, validates it as RFC1918 IPv4 and exports only that address as TRUSTED_PROXIES.
  - PR 383 adds a host-network deployment probe for the canonical public HTTPS login form action.
  - Agent Governance run 30669462169 passed on implementation head c91d9d5670fae811f9ba45fe43420f15149c0c0b.
  - Character Bazaar Staging Validation run 30669462174 passed on implementation head c91d9d5670fae811f9ba45fe43420f15149c0c0b.
derived:
  - The public no-op login symptom is consistent with the HTTPS page rendering an HTTP form action that CSP form-action self blocks.
  - Docker host-loopback traffic should enter the Platform container from its exact default bridge gateway; the staging probe will fail closed if this host-specific assumption is wrong.
unknown:
  - Final results of CI, Synology image build, Phase 7, edge, DB-outage and concurrency workflows.
  - Exact live staging result after merge and Marketplace-aware deploy-enable.
conflicts:
  - Open PR #335 lists deploy/synology/compose.yml, but its compose restart-policy delta is already present on main; PR 383 does not touch that path or its unique boot-repair script.
first_failure:
  marker: public-login-submit-no-op
  evidence: owner report against https://oteryn.molehill.cloud/login?locale=en plus previous deployment configuration
rejected_hypotheses:
  - The locale query parameter is not causal; it only selects localization.
  - A JavaScript handler is not causal; the login form uses native HTML POST.
  - Trusting all Cloudflare ranges or a private subnet is unnecessary and broader than the loopback-origin boundary.
changed_paths:
  - deploy/synology/compose.marketplace.yml
  - deploy/synology/docker/platform-entrypoint.sh
  - deploy/synology/docker/platform.Dockerfile
  - deploy/synology/scripts/health-check.sh
  - docs/agents/tasks/active/OTERYN-20260801-public-https-proxy.md
validation:
  - command: GitHub Actions Agent Governance run 30669462169
    result: PASS
    evidence: exact implementation head c91d9d5670fae811f9ba45fe43420f15149c0c0b
  - command: GitHub Actions Character Bazaar Staging Validation run 30669462174
    result: PASS
    evidence: exact implementation head c91d9d5670fae811f9ba45fe43420f15149c0c0b
  - command: remaining exact-head workflows
    result: NOT_RUN
    evidence: queued or in progress
blockers:
  - none
next_action: Inspect all exact-head workflow results for PR 383 and repair any failing image, shell or deployment-package check before readiness.
```

## Notes

The public origins remain loopback-only. This task does not trust `*`, a whole private subnet, Cloudflare public ranges or direct untrusted clients. The final rollout must use the Marketplace-aware staging control because Character Bazaar is enabled.
