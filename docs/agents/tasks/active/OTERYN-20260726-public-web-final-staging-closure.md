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
  - latest sanitized MFA prerequisite and QR deployment evidence
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
- [x] Exactly one enabled staging Identity has genuine confirmed MFA and zero administrator-role assignments exist before bootstrap.
- [ ] `content_editor` receives only Wiki access/article/category management and `platform_admin` additionally receives exact `wiki.publish`; no wildcard is introduced.
- [ ] The existing audited `admin:bootstrap` command runs only when there are zero role assignments and exactly one enabled MFA-confirmed Identity; all other counts fail closed.
- [ ] Exactly one enabled MFA-confirmed Identity with all four exact Wiki permissions is selected internally without logging its email.
- [ ] `wiki:launch-content:install` installs or verifies content version `2026-07-26.1` without overwriting editorial changes.
- [ ] A real Chromium instance on the Synology host network receives HTTP 200 from the localized homepage, Wiki index, EN/PL launch articles, sitemap and robots surfaces and verifies expected public headings/content.
- [x] Sanitized reports contain only commit SHA, image tags, workflow/run identifiers, counts, job conclusions and artifact names.
- [ ] Temporary inspection/preparation/one-shot workflows and inert triggers are removed after successful evidence is reconciled; tasks are archived and Issue #145 closes only from exact deployment evidence.
- [x] No production, router, DSM, Internet-exposure, Canary/login-server repository or external-repository write occurs.

## Ownership

```yaml
owned_paths:
  - .github/workflows/one-shot-public-web-final-staging.yml
  - .github/workflows/one-shot-mfa-qr-staging-deploy.yml
  - .github/workflows/inspect-public-web-final-staging-prerequisite.yml
  - .github/workflows/prepare-public-web-final-staging-resume.yml
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
  - superseded PR 213 reviewed RBAC/bootstrap changes
  - current PR 230
  - existing Deploy Synology Staging workflow
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T10:15:00+02:00
head: 7cdad8a207c6ae2d4d965f135c5355fb61cc923b
branch: fix/OTERYN-20260727-public-web-final-staging-resume
pr: 230
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
  - final staging, Wiki RBAC, temporary validation and cleanup paths listed in Ownership
proven:
  - PR 221 exact SHA 37eb31d60aa8a47914745cd326aff6b313851dd0 deployed successfully through one-shot 30224899245 and deployment 30225044085
  - Issue 145 sanitized report 5085918120 records MFA QR Synology staging deployment PASS
  - the deployment health check verified the inline SVG QR renderer, deployed view/CSS markers and protected anonymous MFA route
  - fresh PR 230 is based directly on trusted main SHA 37eb31d60aa8a47914745cd326aff6b313851dd0
  - staging prerequisite run 30248201054 job 89919947353 reported two enabled Identities, exactly one confirmed-MFA Identity and zero administrator-role assignments
  - the prerequisite output contained counts only and no Identity email, password, TOTP secret, QR bytes or recovery codes
  - the reviewed PR 213 migration and exact tests were reconciled onto current main
  - the current main one-shot blob was identical to PR 213 base, so its reviewed guarded bootstrap version was transferred without conflict
  - no production, router, DSM, Internet-exposure or external-repository action occurred
derived:
  - the one-time administrator bootstrap prerequisite is now satisfied exactly
  - migration plus bootstrap must still pass all exact-head repository checks before merge
  - final staging remains fail closed for zero or multiple MFA candidates or eligible Wiki publishers
unknown:
  - exact final PR 230 head after cleanup and validation
  - final trusted-main merge SHA and final staging run identifiers
  - final Wiki installation and Chromium evidence result
conflicts: []
first_failure:
  marker: superseded-branch-conflict
  evidence: PR 213 could not produce a current merge ref after the later QR deployment merges; its bounded changes were reconciled onto fresh PR 230 instead of force-updating the stale branch
rejected_hypotheses:
  - genuine MFA is still absent: sanitized staging inspection proved exactly one confirmed-MFA Identity
  - an administrator role already exists: the same inspection proved zero assignments
  - a synthetic publisher or fabricated MFA is needed: the existing genuine account satisfies the exact prerequisite
  - current main changed the final-staging one-shot since PR 213 base: both versions used blob dda377adcde255bac6f2b73b5164000d7fdd7eb8
changed_paths:
  - .github/workflows/one-shot-public-web-final-staging.yml
  - database/migrations/2026_07_26_183200_grant_wiki_permissions_to_editor_roles.php
  - tests/Feature/Admin/PublicModulePermissionReservationTest.php
  - tests/Feature/EditorialMedia/WikiEditorialMediaSecurityTest.php
  - tests/Feature/Wiki/WikiAuthorizationTest.php
  - docs/agents/tasks/active/OTERYN-20260726-public-web-final-staging-closure.md
  - temporary inspection and preparation workflows pending deletion
validation:
  - command: QR-first exact-SHA deployment
    result: PASS
    evidence: build 30224899239, one-shot 30224899245 and deployment 30225044085 all succeeded for 37eb31d60aa8a47914745cd326aff6b313851dd0
  - command: sanitized staging MFA prerequisite
    result: PASS
    evidence: run 30248201054 job 89919947353 reported enabled 2, confirmed MFA 1, administrator assignments 0
blockers: []
next_action: Remove temporary inspection/preparation workflows, complete all exact-head PR 230 checks, merge with marker [public-web-final-staging], require final staging PASS evidence, then archive both closure tasks and close Issue 145.
```

## Notes

Production remains tracked exclusively by Issue #91. Final cleanup and task archival must occur only after the final exact-SHA staging closure report is PASS.
