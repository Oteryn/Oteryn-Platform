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
- [ ] Temporary one-shot workflows and inert triggers are removed after successful evidence is reconciled; tasks are archived and Issue #145 closes only from exact deployment evidence.
- [x] No production, router, DSM, Internet-exposure, Canary/login-server repository or external-repository write occurs.

## Ownership

```yaml
owned_paths:
  - .github/workflows/one-shot-public-web-final-staging.yml
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
  - superseded PR 213 reviewed RBAC/bootstrap changes
  - current PR 230
  - existing Deploy Synology Staging workflow
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T13:24:00+02:00
head: 91cd8e59e7d797ffbda1121a68c1d52279def964
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
  - final staging, Wiki RBAC and cleanup paths listed in Ownership
proven:
  - PR 221 exact SHA 37eb31d60aa8a47914745cd326aff6b313851dd0 deployed successfully through one-shot 30224899245 and deployment 30225044085
  - Issue 145 sanitized report 5085918120 records MFA QR Synology staging deployment PASS
  - the deployment health check verified the inline SVG QR renderer, deployed view/CSS markers and protected anonymous MFA route
  - fresh PR 230 is based directly on trusted main SHA 37eb31d60aa8a47914745cd326aff6b313851dd0
  - staging prerequisite run 30248201054 job 89919947353 reported two enabled Identities, exactly one confirmed-MFA Identity and zero administrator-role assignments
  - the prerequisite output contained counts only and no Identity email, password, TOTP secret, QR bytes or recovery codes
  - the reviewed PR 213 migration and exact tests were reconciled onto current main
  - the current main one-shot blob was identical to PR 213 base, so its reviewed guarded bootstrap version was transferred without conflict
  - temporary prerequisite, preparation and CI-inspection workflows were removed from the final diff
  - CI run 30249747324 isolated its only error to an invalid mixed-to-string cast in a test helper; commit 2fea5fe4885f973aff65c05b71b00e8743687238 now validates every database value with is_string before returning list<string>
  - Governance run 30249747328 isolated its only error to unsupported validation result FAIL_FIXED; the checkpoint now uses the supported result FAIL
  - no production, router, DSM, Internet-exposure or external-repository action occurred
derived:
  - the one-time administrator bootstrap prerequisite is satisfied exactly
  - migration plus bootstrap must pass all exact-head repository checks before merge
  - final staging remains fail closed for zero or multiple MFA candidates or eligible Wiki publishers
unknown:
  - exact final trusted-main merge SHA and final staging run identifiers
  - final Wiki installation and Chromium evidence result
conflicts: []
first_failure:
  marker: phpstan-role-key-list
  evidence: CI runs 30249031419 and 30249747324 showed the test helper needed explicit runtime string validation; no runtime authorization code failed
rejected_hypotheses:
  - genuine MFA is still absent: sanitized staging inspection proved exactly one confirmed-MFA Identity
  - an administrator role already exists: the same inspection proved zero assignments
  - a synthetic publisher or fabricated MFA is needed: the existing genuine account satisfies the exact prerequisite
  - current main changed the final-staging one-shot since PR 213 base: both versions used blob dda377adcde255bac6f2b73b5164000d7fdd7eb8
  - the CI failure affected runtime authorization: it was isolated to a test-helper static return type
changed_paths:
  - .github/workflows/one-shot-public-web-final-staging.yml
  - database/migrations/2026_07_26_183200_grant_wiki_permissions_to_editor_roles.php
  - tests/Feature/Admin/PublicModulePermissionReservationTest.php
  - tests/Feature/EditorialMedia/WikiEditorialMediaSecurityTest.php
  - tests/Feature/Wiki/WikiAuthorizationTest.php
  - docs/agents/tasks/active/OTERYN-20260726-public-web-final-staging-closure.md
validation:
  - command: QR-first exact-SHA deployment
    result: PASS
    evidence: build 30224899239, one-shot 30224899245 and deployment 30225044085 all succeeded for 37eb31d60aa8a47914745cd326aff6b313851dd0
  - command: sanitized staging MFA prerequisite
    result: PASS
    evidence: run 30248201054 job 89919947353 reported enabled 2, confirmed MFA 1, administrator assignments 0
  - command: PR 230 exact-head CI before final type guard
    result: FAIL
    evidence: run 30249747324 reported only Cannot cast mixed to string in the test helper; the guarded is_string implementation is committed for the next exact-head run
  - command: PR 230 exact-head Governance before checkpoint result normalization
    result: FAIL
    evidence: run 30249747328 rejected unsupported validation result FAIL_FIXED; the checkpoint now records supported values only
blockers: []
next_action: Complete all exact-head PR 230 checks, merge with marker [public-web-final-staging], require final staging PASS evidence, then remove one-shot workflows/triggers, archive both closure tasks and close Issue 145.
```

## Notes

Production remains tracked exclusively by Issue #91. Final cleanup and task archival must occur only after the final exact-SHA staging closure report is PASS.
