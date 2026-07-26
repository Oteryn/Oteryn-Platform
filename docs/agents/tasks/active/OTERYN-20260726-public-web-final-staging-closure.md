---
task_id: OTERYN-20260726-public-web-final-staging-closure
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/EXECUTION_MODE_ROUTING.md
  - docs/agents/tasks/TASK_TEMPLATE.md
  - docs/agents/tasks/active/OTERYN-20260725-public-web-programme-closure.md
  - docs/agents/tasks/archive/OTERYN-20260726-source-backed-wiki-content.md
  - deploy/synology/README.md
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/deploy-synology-staging.yml
search_first:
  - active tasks and open pull requests touching Synology staging deployment, temporary dispatch workflows, Wiki launch-content provisioning or Issue 145 closure
  - latest trusted main and exact Platform/Gateway image-tag behavior
  - latest successful Synology staging deployment and LAN game configuration evidence
  - existing deployment environment, runner, secret and cleanup boundaries
  - eligible staging Wiki publisher selection without logging identity data
  - public staging routes required for a bounded live Chromium smoke
optional_reads:
  - docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md
  - docs/architecture/WIKI_IMPLEMENTATION_PLAN.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/TEST_STRATEGY.md
---

# OTERYN-20260726-public-web-final-staging-closure

## Goal

Deploy the exact final Issue #145 trusted-main SHA to the existing Synology staging environment, provide QR-first genuine MFA enrollment, install the reviewed Wiki launch-content package through one eligible publisher and retain sanitized live Chromium smoke evidence without production, router, DSM, Internet-exposure or external-repository writes.

## Acceptance criteria

- [x] The first reviewed merge SHA built and published exact `sha-<full-sha>` Platform and Game Gateway images before deployment dispatch.
- [x] The repaired deployment recreates the private proxy after Platform and completes healthily.
- [ ] The exact trusted-main merge containing QR-first MFA is deployed to Synology staging.
- [ ] The deployed Platform locally renders the TOTP provisioning URI as an inline SVG QR code while the anonymous MFA route remains protected.
- [ ] Exactly one enabled MFA-confirmed Identity with all four exact Wiki permissions is selected internally without logging its email; zero or multiple candidates fail closed.
- [ ] `wiki:launch-content:install` installs or verifies content version `2026-07-26.1` without overwriting editorial changes.
- [ ] A real Chromium instance on the Synology host network receives HTTP 200 from the localized homepage, Wiki index, EN/PL launch articles, sitemap and robots surfaces and verifies expected public headings/content.
- [x] Sanitized reports contain only commit SHA, image tag, workflow/run identifiers, job conclusions and artifact names.
- [ ] Temporary one-shot workflows and inert triggers are removed after successful evidence is reconciled, tasks are archived and Issue #145 closes only from exact deployment evidence.
- [x] No production, router, DSM, Internet-exposure, Canary/login-server repository or external-repository write occurs.

## Ownership

```yaml
owned_paths:
  - .github/workflows/one-shot-public-web-final-staging.yml
  - .github/workflows/one-shot-mfa-qr-staging-deploy.yml
  - deploy/synology/.public-web-final-staging-trigger
  - deploy/synology/scripts/deploy.sh
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260725-public-web-programme-closure.md
  - docs/agents/tasks/active/OTERYN-20260726-public-web-final-staging-closure.md
  - docs/agents/tasks/archive/OTERYN-20260726-public-web-final-staging-closure.md
  - docs/agents/tasks/archive/OTERYN-20260725-public-web-programme-closure.md
modules:
  - Deployment
  - PublicPortal
  - Identity
  - Wiki
  - Testing
  - AgentGovernance
dependencies:
  - Issue 145
  - PR 208 merge f8002191f0e5270dc4191227fd01d5e709ee5ab6
  - PR 209 merge a262996eda36fc9430fe1883ea637ffd2f6ff698
  - PR 212 merge b161983c4bf42ba21d00287bfad0418a605dd99c
  - PR 214 merge 671ac9fed05f51cc3989ff0aed2d37c99bc6d933
  - existing Deploy Synology Staging workflow
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T00:20:00+02:00
head: 8b85bb10c6b75beaa81f33c0ffe55ee4b8330889
branch: chore/OTERYN-20260726-mfa-qr-staging-deploy
pr: 215
status: validating
context_routes:
  - agent-governance
  - deployment
  - web-cms
  - identity
  - database
  - admin-rbac
  - security
  - testing
  - accessibility
owned_paths:
  - final staging, QR deployment and cleanup paths listed in Ownership
proven:
  - PR 212 repaired the stale private-proxy upstream and its exact deployment run 30214436534 completed successfully
  - the final closure then failed closed because staging had zero MFA-confirmed Identities and zero eligible Wiki publishers
  - PR 214 merged QR-first local SVG TOTP enrollment as 671ac9fed05f51cc3989ff0aed2d37c99bc6d933
  - all required PR 214 checks passed on exact head aa49338225a5a3cb5917681e9ddd385f1f081327
  - the bounded MFA QR one-shot waits for exact images, dispatches only the existing guarded Synology workflow and verifies the renderer inside the exact deployed Platform container
  - the QR verification uses a synthetic non-user TOTP URI and records no QR bytes, identity email, password or secret
  - the anonymous `/mfa` route must remain HTTP 302 after deployment
  - all preparator and inspection helpers plus the temporary PR marker were removed before final review
  - no production, router, DSM, Internet-exposure or external-repository action occurred
derived:
  - deploying QR before genuine MFA confirmation removes the usability blocker without fabricating MFA state or granting a synthetic publisher
  - final Wiki publication remains intentionally fail-closed until exactly one existing enabled account completes genuine MFA
unknown:
  - exact PR 215 merge SHA and resulting QR one-shot/deployment run identifiers
  - post-deployment confirmed-MFA count
conflicts: []
first_failure:
  marker: no-confirmed-mfa-publisher
  evidence: final staging live-smoke found two enabled Identities, zero confirmed-MFA Identities and zero eligible Wiki publishers after a healthy deployment
rejected_hypotheses:
  - exact images or deployment health are still broken: the repaired deployment completed successfully
  - manual secret transcription is acceptable as the primary enrollment path: QR-first enrollment is merged and must be deployed
  - MFA can be fabricated for closure: genuine confirmation by one existing Identity remains required
  - a synthetic publisher can install Wiki content: publisher selection remains exact-permission and MFA guarded
changed_paths:
  - .github/workflows/one-shot-mfa-qr-staging-deploy.yml
  - deploy/synology/.public-web-final-staging-trigger
  - docs/agents/tasks/active/OTERYN-20260726-public-web-final-staging-closure.md
validation:
  - command: PR 214 exact-head required checks
    result: PASS
    evidence: CI, Governance, Phase 7, Acceptance, DB outage, concurrency and Synology image build passed on aa49338225a5a3cb5917681e9ddd385f1f081327
  - command: QR staging one-shot review
    result: PASS
    evidence: workflow uses trusted-main marker gating, exact-SHA image verification, existing deployment dispatch and bounded non-secret in-container QR checks
blockers: []
next_action: Complete all exact-head PR 215 checks, merge with marker [mfa-qr-staging], require a sanitized PASS report, then have exactly one existing staging account complete genuine MFA before resuming the guarded final Wiki closure.
```

## Notes

Production remains tracked exclusively by Issue #91. Final cleanup and task archival must occur only after exact-SHA QR deployment evidence and the later full final-staging closure report are PASS.
