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
- [ ] The current exact head passes all required workflow families.
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
  - none for repository validation, merge and bounded staging deployment
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-01T09:23:00Z
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
head: 1440848ea0ad1eadd78f084f1f1c65c555f0e897
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
  - PR #387 source head c8ca2fc995fbbc4a0f3c7268872d3843db950af8 proved the first public failure as gateway-public-tls-handshake-failure.
  - Canonical routes remain WWW to loopback 8000 and Gateway to loopback 8080.
  - PR #335-owned compose.yml and boot-repair.sh remain untouched.
  - The repository repair enforces canonical requestless URLs, Secure cookies and bounded Gateway/cross-routing checks.
  - Character Bazaar Staging Control is the established Marketplace-aware live staging path.
  - The Marketplace Compose override pins Platform and scheduler APP_URL to https://oteryn.molehill.cloud.
  - The shared environment loader accepts only the canonical origin or exact historical loopback value for Character Bazaar Staging Control, rejects other overrides, then exports canonical HTTPS and Secure cookies.
  - A squash merge changing compose.marketplace.yml with [character-bazaar-staging] triggers exact-image publication and guarded staging deployment.
  - Prior exact candidate 5b14f54c340360bca1c15dc3af7099da2628b3e5 passed all nine workflow families before the Marketplace-path gap was discovered.
  - Character Bazaar Staging Validation 39, Synology Production Target Preflight 705, Platform DB Outage Validation 2958 and Game Auth Ticket Concurrency 2529 passed on 1440848ea0ad1eadd78f084f1f1c65c555f0e897.
  - Agent Governance 3792 failed only because the checkpoint used unsupported result PENDING; this checkpoint changes it to NOT_RUN.
  - The owner explicitly authorized bounded staging deployment and the recorded reversible edge plan on 2026-08-01.
  - No Cloudflare, DNS, production, Canary or PR #387 evidence mutation occurred.
derived:
  - The previous candidate repaired the standalone workflow but not the established Marketplace-aware staging path.
  - The added Marketplace policy closes that gap without public origin exposure or broader proxy trust.
unknown:
  - Final results of remaining workflow families on the post-checkpoint exact head.
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
  - command: prior exact-head workflow suite on 5b14f54c340360bca1c15dc3af7099da2628b3e5
    result: PASS
    evidence: all nine required workflow families passed before the Marketplace-path correction
  - command: required workflow suite on the current implementation and checkpoint
    result: NOT_RUN
    evidence: four families passed on 1440848ea0ad1eadd78f084f1f1c65c555f0e897; remaining and post-checkpoint runs are pending inspection
  - command: trusted-main image publication and Character Bazaar Staging Control deploy-enable
    result: NOT_RUN
    evidence: requires green exact-head checks and squash merge with the guarded marker
deployment_evidence: Historical run 30669701842 is STAGING_PROVEN only; the current candidate is not deployed yet.
rollback: Repository rollback is PR revert; runtime deployment snapshots Platform, Gateway and Canary image references; external rollback restores recorded edge state.
blockers:
  - none for exact-head validation
next_action: Inspect all workflow results on the new exact head, repair any failure, then squash merge with [character-bazaar-staging] and verify trusted-main image and staging-control runs.
```

## Report

`docs/agents/reports/OTERYN-20260801-public-domain-repair.md`

## Notes

Staging completion does not prove the public edge or production. `PUBLIC_DOMAIN_LAUNCH_READY` and `PRODUCTION_PROVEN` remain false until external acceptance checks pass.
