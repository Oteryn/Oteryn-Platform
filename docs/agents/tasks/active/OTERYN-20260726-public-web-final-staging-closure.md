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

- [x] Exact trusted-main Platform and Gateway images are built before every guarded deployment.
- [x] The repaired deployment recreates the private proxy after Platform and completes healthily.
- [x] Trusted-main SHAs containing QR-first MFA deployed healthily through runs `30223164413` and `30223546917`.
- [ ] The deployment job itself verifies the local inline SVG TOTP QR, deployed view/CSS markers and protected anonymous MFA route.
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
  - deploy/synology/scripts/health-check.sh
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
  - PR 215 merge 348f483938fc8358132128fc79d229e38b98045b
  - PR 219 merge cb14a5c5209e868b0d99c42f3d1601505d1dd6d7
  - existing Deploy Synology Staging workflow
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T00:58:00+02:00
head: ad36ed18b7e0325953afdba323c9f3180035b2d8
branch: fix/OTERYN-20260727-mfa-qr-inline-healthcheck
pr: null
status: implementing
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
  - PR 214 merged QR-first local SVG TOTP enrollment and all required checks passed
  - PR 215 and PR 219 exact images built and deployed successfully without rollback
  - first verifier failure was an incorrect renderDataUri method name; App Identity Mfa MfaQrCode exposes dataUri
  - corrected deployment run 30223546917 completed successfully
  - the corrected follow-on verifier remained queued because the self-hosted runner behaves as a one-job execution boundary
  - repository runner-status API is unavailable to the GitHub App and no repo-managed runner recovery path exists
  - deploy.sh already invokes health-check.sh inside the successful deployment job before reporting deployment health
  - health-check.sh now verifies dataUri with a synthetic non-user TOTP URI, SVG properties, deployed QR view/CSS markers and anonymous `/mfa` HTTP 302
  - the one-shot no longer creates a second self-hosted job; deployment success includes the inline QR health check
  - SHA-specific one-shot concurrency prevents the orphaned previous verifier from blocking the new corrected run
  - no production, router, DSM, Internet-exposure or external-repository action occurred
derived:
  - inline deployment verification is the correct durable model for a one-job runner
  - a successful new deployment report is sufficient proof that exact images, standard health checks and QR smoke passed in the same job
  - final Wiki publication remains intentionally fail-closed until exactly one existing enabled account completes genuine MFA
unknown:
  - inline-health-check merge SHA and resulting one-shot/deployment run identifiers
  - post-deployment confirmed-MFA count
conflicts: []
first_failure:
  marker: follow-on-runner-unavailable
  evidence: corrected deployment run 30223546917 passed, while one-shot verify job 89850351045 remained queued and a duplicate read-only self-hosted job also remained queued
rejected_hypotheses:
  - exact images or deployment are unhealthy: both corrected image and deployment runs passed
  - the public QR API is wrong: dataUri is the implemented and unit-tested API
  - another follow-on job will reliably obtain the runner: two independent follow-on jobs remained queued
  - MFA can be fabricated for closure: genuine confirmation by one existing Identity remains required
changed_paths:
  - .github/workflows/one-shot-mfa-qr-staging-deploy.yml
  - deploy/synology/.public-web-final-staging-trigger
  - deploy/synology/scripts/health-check.sh
  - docs/agents/tasks/active/OTERYN-20260726-public-web-final-staging-closure.md
validation:
  - command: PR 219 exact-head required checks
    result: PASS
    evidence: Governance, CI, concurrency, DB outage, Phase 7 and Synology image/package build passed on 53a85e930e5398ecc210fbf9662ba6a5b9f92f93
  - command: corrected QR staging deployment
    result: PASS
    evidence: exact image run 30223521657 and deployment run 30223546917 completed successfully
  - command: corrected follow-on QR verifier
    result: BLOCKED
    evidence: job 89850351045 remained queued after deployment, consistent with a one-job self-hosted runner boundary
blockers: []
next_action: Validate the four-file inline-health-check change, merge it with marker [mfa-qr-staging], require a sanitized PASS report, then have exactly one existing staging account complete genuine MFA before resuming the guarded final Wiki closure.
```

## Notes

Production remains tracked exclusively by Issue #91. Final cleanup and task archival must occur only after exact-SHA QR deployment evidence and the later full final-staging closure report are PASS.
