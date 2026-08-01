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
  - PR #388 merged repair and push-triggered deployment evidence
  - Character Bazaar Staging Control exact failure logs
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
- [x] Cloudflare/DNS/Synology changes and rollback are documented without secrets.
- [x] PR #392 implementation head passes every workflow applicable to its changed paths.
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
  - merged PR #388
  - Character Bazaar Staging Control
  - Issue #91 production go-live gate
blockers:
  - none for repository merge and bounded staging retry
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-01T09:43:00Z
session_id: chatgpt-20260801-public-domain-repair-003
policy_version: 2
phase: staging_failure_repair
execution_mode: chat-github-connector
repository_mutation_authorization: PROVEN
external_mutation_scope_authorization: PROVEN
external_operator_access: UNKNOWN
staging_deployment_authorization: PROVEN
context_pressure: medium
decomposition_decision: continue
branch: fix/OTERYN-20260801-marketplace-state-loader
head: 921876f9eb16cf81e967d95ded6c99dc6fc0a11e
pr: 392
status: ready
context_routes:
  - agent-governance
  - security
  - auth-identity
  - api
  - testing
owned_paths:
  - deploy/synology/scripts/lib.sh
  - tests/Feature/PublicCanonicalUrlTest.php
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
  - docs/agents/reports/OTERYN-20260801-public-domain-repair.md
proven:
  - PR #388 merged as 82abef518f91d72d392db4420bb335773087c3e1 after all nine exact-head workflow families passed.
  - Build Synology Staging Images run 30693873144 number 1576 completed successfully and published exact Platform and Gateway images.
  - Character Bazaar Staging Control run 30693873142 number 5 failed before Docker deployment while rendering the ephemeral environment.
  - The runner loaded partial durable file marketplace.env through load_oteryn_env_file before loading the complete deployment .env.
  - marketplace.env intentionally contains only Marketplace state and no APP_URL.
  - The previous loader applied Character Bazaar public-origin validation to every loaded file and therefore rejected the partial state file.
  - No Docker deployment step executed in failed run 30693873142; runtime state was not mutated by that attempt.
  - PR #392 scopes canonical migration to a file named exactly .env and adds an executable regression test for partial state plus full environment behavior.
  - Exact implementation head 921876f9eb16cf81e967d95ded6c99dc6fc0a11e passed all seven workflows applicable to its changed paths.
  - CI 4009, Agent Governance 3806, Phase 7 3045, Build Synology Staging Images 1578, Edge Security 1466, DB Outage 2972 and Auth Concurrency 2543 passed.
  - Character Bazaar Staging Validation did not trigger because its path filter excludes lib.sh, PublicCanonicalUrlTest.php and this task record.
  - Synology Production Target Preflight did not trigger because its path filter excludes lib.sh, PublicCanonicalUrlTest.php and this task record.
  - Canonical routes remain WWW to loopback 8000 and Gateway to loopback 8080.
  - PR #335-owned compose.yml and boot-repair.sh remain untouched.
  - No Cloudflare, DNS, production, Canary or PR #387 evidence mutation occurred.
derived:
  - The failure was a repository-owned loader scope defect, not a secret, runner, image-publication or Docker runtime failure.
  - Restricting the migration to the complete .env preserves fail-closed validation while allowing the intentionally partial durable state file.
unknown:
  - Result of the next trusted-main staging retry.
  - Effective Cloudflare certificate, WAF, Access, bot, redirect and HSTS configuration.
  - Exact supported native-client minimum TLS version.
conflicts:
  - The shared loader previously treated a partial state file as complete deployment configuration; PR #392 separates those roles by exact filename.
first_failure:
  marker: marketplace-partial-state-rejected
  evidence: run 30693873142 step Render ephemeral staging control environment rejected missing APP_URL before Docker deployment
rejected_hypotheses:
  - Exact images were not missing; run 30693873142 resolved both image tags and image build run 30693873144 passed.
  - The staging runner and GHCR credentials were not the cause; tool validation and registry login passed.
  - No rollback is required because Execute guarded staging action was skipped.
changed_paths:
  - deploy/synology/scripts/lib.sh
  - tests/Feature/PublicCanonicalUrlTest.php
  - docs/agents/tasks/active/OTERYN-20260801-public-domain-repair.md
validation:
  - command: exact PR #388 workflow suite before merge
    result: PASS
    evidence: CI 3997, Governance 3797, Phase 7 3034, Images 1575, Edge 1455, Preflight 707, DB 2961, Bazaar 41 and Concurrency 2532
  - command: trusted-main image publication for 82abef518f91d72d392db4420bb335773087c3e1
    result: PASS
    evidence: run 30693873144 number 1576
  - command: Character Bazaar Staging Control deploy-enable for 82abef518f91d72d392db4420bb335773087c3e1
    result: FAIL
    evidence: run 30693873142 number 5 failed before Docker due partial state validation
  - command: path-applicable workflow suite on 921876f9eb16cf81e967d95ded6c99dc6fc0a11e
    result: PASS
    evidence: CI 4009, Governance 3806, Phase 7 3045, Images 1578, Edge 1466, DB 2972 and Concurrency 2543
deployment_evidence: Current exact repair is not deployed; failed run 30693873142 performed no Docker mutation.
rollback: No runtime rollback is needed for run 30693873142; repository rollback remains PR revert and later successful deployments retain image snapshots.
blockers:
  - none for merge and bounded staging retry
next_action: Pass the path-applicable workflow suite on this checkpoint-only head, squash merge with [character-bazaar-staging], then verify exact image publication and sanitized STAGING_PROVEN evidence.
```

## Report

`docs/agents/reports/OTERYN-20260801-public-domain-repair.md`

## Notes

Staging completion does not prove the public edge or production. `PUBLIC_DOMAIN_LAUNCH_READY` and `PRODUCTION_PROVEN` remain false until external acceptance checks pass.
