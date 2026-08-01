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

Repair the repository-owned public-domain defects proven by PR #387, deploy the exact repair through Marketplace-aware Synology staging, and retain a reversible operator plan for unavailable edge infrastructure without weakening security boundaries.

## Acceptance criteria

- [x] Requestless Platform URLs use `https://oteryn.molehill.cloud` while origins remain loopback-only.
- [x] Public staging rejects an unexpected `APP_URL`.
- [x] Marketplace Platform and scheduler use the canonical HTTPS origin and Secure cookies.
- [x] Health checks cover Gateway identity, malformed login, private cache controls, canonical URLs and negative cross-routing.
- [x] Cloudflare/DNS/Synology changes and rollback are documented without secrets.
- [x] The implementation exact head passes all required workflow families.
- [ ] The merged image is deployed by Character Bazaar Staging Control with sanitized `STAGING_PROVEN` evidence.
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
updated_at: 2026-08-01T09:27:00Z
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
head: d9eeb035335091fd7dbfbba7b7d5fc070aea5027
pr: 388
status: ready
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
  - PR #387 source head c8ca2fc995fbbc4a0f3c7268872d3843db950af8 proved gateway-public-tls-handshake-failure as the first public failure.
  - Canonical routes remain WWW to loopback 8000 and Gateway to loopback 8080.
  - PR #335-owned compose.yml and boot-repair.sh remain untouched.
  - The repair enforces canonical requestless URLs, Secure cookies and bounded Gateway/cross-routing checks.
  - Character Bazaar Staging Control is the established Marketplace-aware live staging path.
  - Marketplace Compose pins Platform and scheduler APP_URL to https://oteryn.molehill.cloud.
  - The shared loader accepts only the canonical origin or exact historical loopback value for Character Bazaar Staging Control, rejects other overrides, then exports canonical HTTPS and Secure cookies.
  - A squash merge changing compose.marketplace.yml with [character-bazaar-staging] triggers exact-image publication and guarded staging deployment.
  - Exact implementation head d9eeb035335091fd7dbfbba7b7d5fc070aea5027 passed all nine workflow families.
  - CI 3995, Agent Governance 3795, Phase 7 3033, Build Synology Staging Images 1574, Edge Security 1454, Synology Preflight 706, DB Outage 2960, Bazaar Validation 40 and Auth Concurrency 2531 passed.
  - The owner explicitly authorized bounded staging deployment and the recorded reversible edge plan on 2026-08-01.
  - No Cloudflare, DNS, production, Canary or PR #387 evidence mutation occurred.
derived:
  - The previous candidate repaired the standalone workflow but not the established Marketplace-aware staging path.
  - The Marketplace policy closes that gap without public origin exposure or broader proxy trust.
unknown:
  - Effective Cloudflare certificate, WAF, Access, bot, redirect and HSTS configuration.
  - Exact supported native-client minimum TLS version.
  - Whether the public edge changed independently after PR #387.
conflicts:
  - Historical Marketplace staging rendered loopback APP_URL; the final Compose layer and guarded loader now resolve it to the canonical public origin.
first_failure:
  marker: gateway-public-tls-handshake-failure
  evidence: PR #387 runs 30690877286 and 30690957415 failed TLS negotiation before HTTP
rejected_hypotheses:
  - PR #335 does not own the selected repair paths.
  - Broad proxy trust is not required.
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
  - command: required workflow suite on d9eeb035335091fd7dbfbba7b7d5fc070aea5027
    result: PASS
    evidence: CI 3995, Governance 3795, Phase 7 3033, Images 1574, Edge 1454, Preflight 706, DB 2960, Bazaar 40 and Concurrency 2531
  - command: trusted-main image publication and Character Bazaar Staging Control deploy-enable
    result: NOT_RUN
    evidence: requires squash merge with the guarded marker
deployment_evidence: Historical run 30669701842 is STAGING_PROVEN only; the current candidate is not deployed yet.
rollback: Repository rollback is PR revert; runtime deployment snapshots Platform, Gateway and Canary image references; external rollback restores recorded edge state.
blockers:
  - none for merge and staging deployment
next_action: Pass checks on this documentation-only head, then squash merge with [character-bazaar-staging] and verify trusted-main image and staging-control runs.
```

## Report

`docs/agents/reports/OTERYN-20260801-public-domain-repair.md`

## Notes

Staging completion does not prove the public edge or production. `PUBLIC_DOMAIN_LAUNCH_READY` and `PRODUCTION_PROVEN` remain false until external acceptance checks pass.
