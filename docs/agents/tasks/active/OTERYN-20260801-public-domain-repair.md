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
  - open tasks and PRs overlapping Synology deployment and public endpoint paths
  - canonical APP_URL requestless URL generation and deployment health checks
optional_reads:
  - PR #383
  - PR #385
  - PR #335
  - Issue #91
---

# OTERYN-20260801-public-domain-repair

## Goal

Repair the repository-owned public-domain defects proven by PR #387, deploy the exact repair to the established Marketplace-aware Synology staging path, and prepare exact reversible operator changes for edge infrastructure without weakening proxy trust, origin isolation, authentication controls or sensitive-response caching.

## Acceptance criteria

- [x] Canonical requestless Platform URLs use `https://oteryn.molehill.cloud` while loopback origins remain private.
- [x] The guarded public staging workflow rejects a non-canonical public `APP_URL`.
- [x] The Marketplace-aware staging path pins the browser-facing Platform and scheduler to the canonical HTTPS origin and Secure cookies.
- [x] Bounded staging health checks verify canonical requestless URLs, Gateway endpoint identity, no cross-routing and sensitive login response cache controls.
- [x] Exact Cloudflare/DNS/Synology operator changes and rollback are documented without secrets.
- [ ] The new exact head passes all required workflow families.
- [ ] The exact merged image is deployed through the established Marketplace-aware Synology staging control and produces sanitized `STAGING_PROVEN` evidence.
- [x] `PRODUCTION_PROVEN` remains false until Issue #91 is completed.

## Ownership

```yaml
owned_paths:
  - .github/workflows/deploy-synology-staging.yml
  - deploy/synology/.env.example
  - deploy/synology/README.md
  - deploy/synology/compose.marketplace.yml
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/health-check.sh
  - tests/Feature/PublicCanonicalUrlTest.php
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
  - docs/agents/reports/OTERYN-20260801-public-domain-repair.md
modules:
  - public-web
  - identity
  - game-gateway
  - edge-transport
  - synology-staging
dependencies:
  - PR #387 source validation package
  - docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md
  - Character Bazaar Staging Control
  - Issue #91 production go-live gate
blockers:
  - none for repository merge and bounded staging deployment
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-01T09:20:00Z
session_id: chatgpt-20260801-public-domain-repair-002
policy_version: 2
phase: implementation_and_staging_verification
execution_mode: chat-github-connector
repository_mutation_authorization: PROVEN
external_mutation_scope_authorization: PROVEN
external_operator_access: UNKNOWN
staging_deployment_authorization: PROVEN
context_pressure: medium
decomposition_decision: continue
branch: fix/OTERYN-20260801-public-domain-repair
head: 6f413f2b6a7564d0a06ad7d5e0b29978544e338d
pr: 388
status: validating
context_routes:
  - agent-governance
  - security
  - auth-identity
  - api
  - testing
owned_paths:
  - .github/workflows/deploy-synology-staging.yml
  - deploy/synology/.env.example
  - deploy/synology/README.md
  - deploy/synology/compose.marketplace.yml
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/health-check.sh
  - tests/Feature/PublicCanonicalUrlTest.php
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
  - docs/agents/reports/OTERYN-20260801-public-domain-repair.md
proven:
  - Task-start main was 7dac56d3f3f4606be958c875f278edbe410e6b54.
  - PR #387 is open and draft at c8ca2fc995fbbc4a0f3c7268872d3843db950af8 with PUBLIC DOMAIN LAUNCH BLOCKED / FAIL.
  - PR #387 first public failure is gateway-public-tls-handshake-failure.
  - PR #335 owns only deploy/synology/compose.yml and deploy/synology/scripts/boot-repair.sh; those paths remain unchanged by PR #388.
  - Canonical roles remain WWW to loopback port 8000 and Gateway to loopback port 8080.
  - The repair sets guarded public staging APP_URL to https://oteryn.molehill.cloud and rejects any other value.
  - The repair preserves loopback origin bindings and enables Secure cookies for the guarded public staging workflow.
  - The repair adds requestless login, password-reset and signed-route canonical-origin regression coverage.
  - The repair extends Synology health checks for exact Gateway identity, bounded invalid login, private no-store headers, canonical requestless URLs and negative cross-routing.
  - The established live staging deployment path is Character Bazaar Staging Control, not the standalone Deploy Synology Staging workflow.
  - Historical Character Bazaar Staging Control rendered APP_URL=http://127.0.0.1:8000 and SESSION_SECURE_COOKIE=false before the Marketplace Compose override.
  - The repair now pins Marketplace Platform and scheduler APP_URL to https://oteryn.molehill.cloud and keeps the final browser-facing Secure cookie override true.
  - The shared environment loader permits only the exact canonical origin or the exact historical loopback value for Character Bazaar Staging Control, rejects unexpected overrides, and exports canonical HTTPS plus Secure cookies to all deployment scripts.
  - Changing deploy/synology/compose.marketplace.yml activates the established push-path staging control when the squash merge commit includes [character-bazaar-staging].
  - Exact repair candidate 5b14f54c340360bca1c15dc3af7099da2628b3e5 passed all nine automatic workflows before the Marketplace-path gap was found.
  - The owner explicitly authorized the bounded staging deployment and recorded reversible edge plan on 2026-08-01.
  - No Cloudflare, DNS, production, Canary or PR #387 evidence mutation occurred.
derived:
  - The previous candidate repaired the standalone workflow but would not have corrected the established Marketplace-aware staging path.
  - The added Marketplace Compose and loader policy close that staging-path gap without exposing loopback origins or broadening proxy trust.
  - A squash merge titled with [character-bazaar-staging] will trigger exact-image publication and the established guarded staging deployment on main.
unknown:
  - Exact currently effective Cloudflare certificate, WAF, Access, bot, redirect and HSTS configuration.
  - Exact supported native-client minimum TLS version.
  - Whether the public edge has been changed independently since PR #387.
  - Results of required workflows on implementation head 6f413f2b6a7564d0a06ad7d5e0b29978544e338d.
conflicts:
  - Historical Marketplace staging used a loopback APP_URL in its ephemeral file while the canonical public application origin is HTTPS; the final Compose layer and loader policy now resolve this deterministically.
first_failure:
  marker: gateway-public-tls-handshake-failure
  evidence: PR #387 runs 30690877286 and 30690957415 failed TLS negotiation for login.oteryn.molehill.cloud before HTTP
rejected_hypotheses:
  - PR #335 does not own the selected repair paths; its changed-file list is limited to compose.yml and boot-repair.sh.
  - A broad proxy-trust change is not required; the existing exact Synology proxy trust remains unchanged.
  - The standalone Deploy Synology Staging workflow alone is insufficient while Marketplace staging is enabled.
changed_paths:
  - .github/workflows/deploy-synology-staging.yml
  - deploy/synology/.env.example
  - deploy/synology/README.md
  - deploy/synology/compose.marketplace.yml
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/health-check.sh
  - tests/Feature/PublicCanonicalUrlTest.php
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
  - docs/agents/reports/OTERYN-20260801-public-domain-repair.md
validation:
  - command: prior exact-head workflow suite on 5b14f54c340360bca1c15dc3af7099da2628b3e5
    result: PASS
    evidence: all nine required workflow families passed before the Marketplace-path correction
  - command: required workflow suite on implementation head 6f413f2b6a7564d0a06ad7d5e0b29978544e338d
    result: PENDING
    evidence: GitHub Actions triggered by the new commits
  - command: trusted-main image publication and Character Bazaar Staging Control deploy-enable
    result: NOT_RUN
    evidence: requires green exact-head checks followed by squash merge with the guarded marker
deployment_evidence: Historical run 30669701842 is STAGING_PROVEN only; the current repair candidate has not yet been deployed.
rollback: Repository rollback is PR revert; the runtime deployment snapshots Platform/Gateway/Canary image references and supports runtime-image rollback; external rollback restores the recorded certificate, tunnel, WAF, Access, bot, redirect and HSTS snapshots.
blockers:
  - none for the next repository validation step
next_action: Inspect all required workflow results for the new exact head, repair any failure, then squash merge with [character-bazaar-staging] and verify the trusted-main image and staging-control runs.
```

## Report

`docs/agents/reports/OTERYN-20260801-public-domain-repair.md`

## Notes

The repository and staging task may complete without claiming public edge or production proof. Cloudflare/DNS changes remain a separately evidenced operator action; `PUBLIC_DOMAIN_LAUNCH_READY` and `PRODUCTION_PROVEN` stay false until those external checks pass.
