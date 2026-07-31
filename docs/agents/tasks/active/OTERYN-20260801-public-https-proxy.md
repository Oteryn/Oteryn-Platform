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

- [ ] The Synology deployment passes exactly one derived Docker bridge gateway to Laravel as `TRUSTED_PROXIES`; wildcard and broad inferred trust are not used.
- [ ] An HTTPS `APP_URL` forces `SESSION_SECURE_COOKIE=true` in the ephemeral deployment environment.
- [ ] Deploy, rollback and health-check paths use the same public-web Compose overlay.
- [ ] The staging health check proves the rendered login form action is `https://oteryn.molehill.cloud/login?locale=en` when host-originated forwarded headers are supplied.
- [ ] Deployment-package validation covers proxy environment rendering and public HTTPS settings.
- [ ] Required exact-head checks pass before merge.

## Ownership

```yaml
owned_paths:
  - deploy/synology/compose.public-web.yml
  - deploy/synology/.env.example
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/health-check.sh
  - deploy/synology/scripts/rollback.sh
  - deploy/synology/PUBLIC_ENDPOINTS.md
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/build-synology-staging-images.yml
  - docs/agents/tasks/active/OTERYN-20260801-public-https-proxy.md
modules:
  - deployment
  - security
  - identity
  - public-web
dependencies:
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - PR #133 trusted reverse-proxy middleware
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-31T22:05:00Z
head: UNKNOWN
branch: fix/OTERYN-20260801-public-https-proxy
pr: none
status: implementing
context_routes:
  - security
  - auth-identity
  - testing
  - ci-repair
owned_paths:
  - deploy/synology/compose.public-web.yml
  - deploy/synology/.env.example
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/health-check.sh
  - deploy/synology/scripts/rollback.sh
  - deploy/synology/PUBLIC_ENDPOINTS.md
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/build-synology-staging-images.yml
  - docs/agents/tasks/active/OTERYN-20260801-public-https-proxy.md
proven:
  - The canonical public route is HTTPS oteryn.molehill.cloud to host loopback HTTP 127.0.0.1:8000.
  - Laravel already trusts only explicitly configured IP/CIDR proxy entries and rejects wildcard trust.
  - The current Synology workflow writes SESSION_SECURE_COOKIE=false and does not pass TRUSTED_PROXIES to Platform.
  - The login page is a native POST form whose absolute action depends on trusted forwarded scheme and host metadata.
derived:
  - The public no-op login symptom is consistent with the HTTPS page rendering an HTTP form action that CSP form-action self blocks.
  - The narrow host-origin trust target can be derived from the exact Compose private-network gateway used by Docker port forwarding.
unknown:
  - Exact final CI result on the implementation head.
conflicts:
  - Open PR #335 lists deploy/synology/compose.yml, but its compose restart-policy delta is already present on main; this task avoids that path and does not modify its unique boot-repair script.
first_failure:
  marker: public-login-submit-no-op
  evidence: owner report against https://oteryn.molehill.cloud/login?locale=en plus current workflow configuration
rejected_hypotheses:
  - The locale query parameter is not causal; it only selects localization.
  - A JavaScript handler is not causal; the login form uses native HTML POST.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260801-public-https-proxy.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: implementation in progress
blockers:
  - none
next_action: Add the public-web Compose overlay and shared exact Docker-gateway resolver, then wire deploy, rollback, health checks and workflow validation to it.
```

## Notes

The public origins remain loopback-only. This task must not trust `*`, a whole private subnet, Cloudflare public ranges or direct untrusted clients.
