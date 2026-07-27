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
  - docs/architecture/adr/0006-admin-rbac-and-audit-policy.md
  - docs/architecture/adr/0015-wiki-launch-content-provisioning.md
  - deploy/synology/README.md
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/deploy-synology-staging.yml
search_first:
  - active tasks and open pull requests touching final Synology staging, Wiki RBAC, temporary workflows or Issue 145 closure
  - latest trusted main and exact Platform/Gateway image-tag behavior
  - latest sanitized final-staging reports and live-smoke job evidence
  - existing first-administrator bootstrap and Wiki launch-content installation boundaries
  - public routes required for bounded live Chromium smoke
optional_reads:
  - docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md
  - docs/architecture/WIKI_IMPLEMENTATION_PLAN.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/TEST_STRATEGY.md
---

# OTERYN-20260726-public-web-final-staging-closure

## Goal

Deploy the exact final Issue #145 trusted-main SHA to the existing Synology staging environment, bootstrap the sole genuine MFA-confirmed staging Identity through the existing audited one-time administrator command, install the reviewed Wiki launch content and retain sanitized live Chromium evidence without production, router, DSM, Internet-exposure or external-repository writes.

## Acceptance criteria

- [x] Exact trusted-main Platform and Gateway images are built before every guarded deployment.
- [x] QR-first MFA is deployed and verified inside the deployment health check.
- [x] Exactly one enabled staging Identity has genuine confirmed MFA and zero administrator-role assignments existed before bootstrap.
- [x] `content_editor` receives only Wiki access/article/category management and `platform_admin` additionally receives exact `wiki.publish`; no wildcard is introduced.
- [x] The existing audited `admin:bootstrap` command ran only after the zero-assignment and sole genuine MFA-candidate gate passed.
- [x] Exactly one enabled MFA-confirmed Identity with all four exact Wiki permissions is selected internally without logging its email.
- [x] `wiki:launch-content:install` installed or verified content version `2026-07-26.1` without overwriting editorial changes.
- [ ] A real Chromium instance on the Synology host network receives HTTP 200 from the localized homepage, Wiki index, EN/PL launch articles, sitemap and robots surfaces and verifies expected public headings/content.
- [x] Sanitized reports contain only commit SHA, image tags, workflow/run identifiers, counts, job conclusions and artifact names.
- [ ] Temporary one-shot workflows and inert triggers are removed after successful evidence is reconciled; tasks are archived and Issue #145 closes only from exact deployment evidence.
- [x] No production, router, DSM, Internet-exposure, Canary/login-server repository or external-repository write occurs.

## Ownership

```yaml
owned_paths:
  - .github/workflows/one-shot-public-web-final-staging.yml
  - .github/workflows/one-shot-public-web-final-staging-smoke-retry.yml
  - .github/workflows/one-shot-mfa-qr-staging-deploy.yml
  - deploy/synology/.public-web-final-staging-trigger
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/health-check.sh
  - database/migrations/2026_07_26_183200_grant_wiki_permissions_to_editor_roles.php
  - tests/Feature/Admin/PublicModulePermissionReservationTest.php
  - tests/Feature/EditorialMedia/WikiEditorialMediaSecurityTest.php
  - tests/Feature/Wiki/WikiAuthorizationTest.php
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
  - AdminRbac
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
  - PR 220 merge 83c1d3b13eb3623930f5f068266333dae9380c24
  - PR 221 merge 37eb31d60aa8a47914745cd326aff6b313851dd0
  - PR 230 merge d7984a2def655a01b513cdbc823117f37b90d5d4
  - current PR 232
  - existing Deploy Synology Staging workflow
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T13:44:00+02:00
head: 1ca854fb7356ee0633a206e364ed93cd9f54d817
branch: fix/OTERYN-20260727-final-staging-smoke-volume
pr: 232
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
  - final staging, named-volume smoke retry and cleanup paths listed in Ownership
proven:
  - PR 230 exact head 479df87ea133aa68fceca5678d390e2e002827f0 passed Governance, CI, concurrency, DB outage, Phase 7, Synology image build and Acceptance E2E/Visual UX before merge
  - PR 230 merged as d7984a2def655a01b513cdbc823117f37b90d5d4
  - exact image build 30262166816 and deployment 30262204007 completed successfully for d7984a2def655a01b513cdbc823117f37b90d5d4
  - one-shot 30262167013 live-smoke job 89965363040 verified exact deployed images
  - the same job completed the audited first-administrator bootstrap successfully
  - the same job installed or verified reviewed Wiki launch content successfully through the sole eligible MFA-confirmed publisher
  - the only live-smoke failure was Docker exit 125 because runner-container path /tmp/tmp.JhNx1wMHeU did not exist in the host Docker daemon namespace
  - Issue 145 report comment 5090809928 correctly recorded FAIL rather than claiming staging closure
  - PR 232 replaces the host-path bind with a Docker named volume, streams smoke.cjs through stdin and reads evidence.json back through a container
  - no Identity email, password, TOTP secret, recovery code, production, router, DSM, Internet-exposure or external-repository action occurred
derived:
  - the RBAC, MFA, bootstrap, publisher and Wiki-content gates no longer block closure
  - a named Docker volume removes the runner-container versus host-daemon filesystem mismatch without weakening browser assertions
  - the retry remains exact-SHA because deploy/synology trigger changes force new images and the retry workflow verifies exact running image tags
unknown:
  - PR 232 exact-head validation result
  - retry merge SHA, one-shot run, deployment run and Chromium evidence result
conflicts: []
first_failure:
  marker: runner-container-bind-mount
  evidence: one-shot 30262167013 live-smoke job 89965363040 ended with Docker exit 125 because /tmp/tmp.JhNx1wMHeU was not visible to the host daemon; application, deployment, bootstrap and Wiki installation steps had already passed
rejected_hypotheses:
  - deployment failed: deployment run 30262204007 completed successfully
  - administrator bootstrap failed: the bootstrap step completed successfully
  - Wiki content installation failed: the install-or-verify step completed successfully
  - Chromium found a route or heading defect: the Playwright container never started because Docker rejected the bind mount first
  - a user secret is required for retry: the existing confirmed MFA and role assignment are already persisted
changed_paths:
  - .github/workflows/one-shot-public-web-final-staging-smoke-retry.yml
  - deploy/synology/.public-web-final-staging-trigger
  - docs/agents/tasks/active/OTERYN-20260726-public-web-final-staging-closure.md
validation:
  - command: PR 230 exact-head required checks
    result: PASS
    evidence: seven required workflows passed on 479df87ea133aa68fceca5678d390e2e002827f0
  - command: exact-SHA deployment and pre-browser final gates
    result: PASS
    evidence: build 30262166816, deployment 30262204007, bootstrap and Wiki install-or-verify steps all succeeded for d7984a2def655a01b513cdbc823117f37b90d5d4
  - command: first live Chromium staging smoke
    result: FAIL
    evidence: job 89965363040 failed before Playwright start with host-daemon bind-mount error for runner-local /tmp
blockers: []
next_action: Complete PR 232 exact-head checks, merge with marker [public-web-final-staging], require a sanitized PASS report, then remove all temporary one-shot workflows/triggers, archive both closure tasks and close Issue 145.
```

## Notes

Production remains tracked exclusively by Issue #91. Final cleanup and task archival must occur only after the final exact-SHA staging closure report is PASS.
