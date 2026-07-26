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

Deploy the exact final Issue #145 trusted-main SHA to the existing Synology staging environment through a temporary reviewed one-shot dispatcher, install the exact reviewed Wiki launch-content package through one existing eligible publisher and retain sanitized live Chromium smoke evidence without production, router, DSM, Internet-exposure or external-repository writes.

## Acceptance criteria

- [x] The reviewed merge SHA builds and publishes exact `sha-<full-sha>` Platform and Game Gateway images before deployment dispatch.
- [x] The existing `Deploy Synology Staging` workflow deploys exact tag `sha-b161983c4bf42ba21d00287bfad0418a605dd99c` healthily with the previously proven immutable compatible Canary digest and LAN-only game configuration.
- [x] Deployment runs only on trusted `main`, environment `synology-staging` and runner label `oteryn-staging`.
- [x] The deployed Platform and Gateway containers report the exact expected image tag with zero ambiguous service selection.
- [ ] Exactly one enabled MFA-confirmed Identity with all four exact Wiki permissions is selected internally without logging its email; zero or multiple eligible publishers fail closed.
- [ ] `wiki:launch-content:install` installs or verifies content version `2026-07-26.1` without overwriting editorial changes.
- [ ] A real Chromium instance on the Synology host network receives HTTP 200 from the localized homepage, Wiki index, EN/PL launch articles, sitemap and robots surfaces and verifies expected public headings/content.
- [x] Sanitized evidence records only commit SHA, image tags, workflow/run identifiers, route/status assertions and timestamps.
- [ ] The temporary dispatcher, inert image-build trigger and read-only diagnostics are removed after successful evidence is reconciled, the task is archived and Issue #145 closes only from exact deployment evidence.
- [x] No production, router, DSM, Internet-exposure, Canary/login-server repository or external-repository write occurs.

## Ownership

```yaml
owned_paths:
  - .github/workflows/one-shot-public-web-final-staging.yml
  - .github/workflows/inspect-public-web-final-staging-pass.yml
  - deploy/synology/.public-web-final-staging-trigger
  - deploy/synology/scripts/deploy.sh
  - database/migrations/2026_07_26_183200_grant_wiki_permissions_to_editor_roles.php
  - tests/Feature/Admin/PublicModulePermissionReservationTest.php
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260725-public-web-programme-closure.md
  - docs/agents/tasks/active/OTERYN-20260726-public-web-final-staging-closure.md
  - docs/agents/tasks/archive/OTERYN-20260726-public-web-final-staging-closure.md
  - docs/agents/tasks/archive/OTERYN-20260725-public-web-programme-closure.md
modules:
  - Deployment
  - PublicPortal
  - Wiki
  - AdminRbac
  - Testing
  - AgentGovernance
dependencies:
  - Issue 145
  - PR 208 merge f8002191f0e5270dc4191227fd01d5e709ee5ab6
  - PR 209 merge a262996eda36fc9430fe1883ea637ffd2f6ff698
  - PR 210 merge a59a815472ab089572b6680a1f5fb4d9adcc3b44
  - PR 211 merge 62dedb894ffc3af55ba10a0717a6a892b87f1370
  - PR 212 merge b161983c4bf42ba21d00287bfad0418a605dd99c
  - existing Deploy Synology Staging workflow
blockers:
  - exactly one existing enabled staging Identity must complete genuine MFA confirmation
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-26T18:35:00Z
head: f0b6bb0504db3a445d42b484d572261fb53ecd7a
branch: chore/OTERYN-20260726-public-web-programme-final-cleanup
pr: 213
status: blocked
context_routes:
  - agent-governance
  - deployment
  - web-cms
  - database
  - admin-rbac
  - security
  - testing
  - accessibility
owned_paths:
  - final staging repair, Wiki RBAC and later cleanup paths listed in Ownership
proven:
  - PR 212 exact head 15381fa1b36e859b16c0b584b77cc6443d28f8d9 passed all six required workflows before squash merge
  - PR 212 squash-merged as trusted main b161983c4bf42ba21d00287bfad0418a605dd99c with the guarded final-staging marker
  - one-shot run 30214414070 resolved exact images and successfully monitored deployment run 30214436534
  - deployment run 30214436534 succeeded after deploy.sh force-recreated only internal-proxy following Platform recreation
  - exact Platform and Gateway images sha-b161983c4bf42ba21d00287bfad0418a605dd99c are deployed
  - the live-smoke job verified exact deployed images before the Wiki publication step
  - the live-smoke job failed closed before content mutation with Expected exactly one eligible staging Wiki publisher found 0
  - sanitized staging inspection found two enabled Identities, zero confirmed-MFA Identities, zero administrator-role assignments and zero eligible Wiki publishers
  - existing content_editor and platform_admin role bundles do not contain the four reserved Wiki permissions
  - ADR 0006 requires an explicit role-permission decision and forbids wildcard or implicit platform-admin authority
  - ADR 0015 requires one existing enabled MFA-confirmed publisher and rejects a synthetic system publisher
  - routes/console.php and AdminRoleManager provide the existing one-time audited admin:bootstrap boundary, which requires confirmed MFA and closes after the first assignment
  - PR 213 adds explicit idempotent role grants: content_editor receives Wiki access/article/category management; platform_admin receives those three plus Wiki publication
  - PR 213 adds regression coverage proving wiki.publish remains restricted to platform_admin
  - the guarded one-shot is prepared to call admin:bootstrap only when there are no role assignments and exactly one enabled MFA-confirmed Identity; zero or multiple candidates fail closed
  - the bootstrap candidate email is held only in process memory, is not printed and is unset after the audited command
  - no Identity, MFA confirmation or role assignment is created by migration
  - no production, router, DSM, Internet-exposure or external-repository action occurred
derived:
  - the reverse-proxy deployment defect is resolved; the remaining closure blocker is genuine administrator identity state, not runtime health
  - merging or rerunning before one Identity completes MFA would deterministically produce another fail-closed report and must be avoided
  - after genuine MFA confirmation, the existing audited bootstrap can create the first platform_admin assignment and the new explicit Wiki role grants can make that Identity eligible for launch-content publication
unknown:
  - which existing staging Identity the operator will use for MFA confirmation; identity details must not be logged or stored in the task
  - exact final one-shot/deployment run identifiers until the MFA prerequisite is satisfied and PR 213 is merged
conflicts: []
first_failure:
  marker: staging-wiki-publisher-mfa-boundary
  evidence: one-shot run 30214414070 dispatch succeeded, deployment run 30214436534 succeeded, but live-smoke job 89826340682 exited 32 because eligible publisher count was zero; sanitized inspection proved confirmed MFA count zero
rejected_hypotheses:
  - deployment remains unhealthy: deployment run 30214436534 succeeded and exact images were verified by live-smoke
  - assign a role by migration: ADR 0006 forbids administrator role assignment by migration
  - set MFA state automatically: confirmed MFA is an independent user security ceremony and cannot be fabricated by deployment
  - use a synthetic or system publisher: ADR 0015 explicitly rejects it
  - grant Wiki publication to content_editor: publication remains restricted to platform_admin as the more conservative explicit role decision
  - rerun immediately: with confirmed MFA count zero it would fail identically without advancing evidence
changed_paths:
  - .github/workflows/one-shot-public-web-final-staging.yml
  - .github/workflows/inspect-public-web-final-staging-pass.yml
  - database/migrations/2026_07_26_183200_grant_wiki_permissions_to_editor_roles.php
  - tests/Feature/Admin/PublicModulePermissionReservationTest.php
  - docs/agents/tasks/active/OTERYN-20260726-public-web-final-staging-closure.md
validation:
  - command: PR 212 exact-head required workflows
    result: PASS
    evidence: Agent Governance, CI, concurrency, DB outage, Phase 7 and Synology image build passed on 15381fa1b36e859b16c0b584b77cc6443d28f8d9
  - command: repaired exact-SHA deployment
    result: PASS
    evidence: one-shot run 30214414070 monitored successful deployment run 30214436534 for b161983c4bf42ba21d00287bfad0418a605dd99c
  - command: Wiki launch-content publication and Chromium smoke
    result: BLOCKED
    evidence: live-smoke job 89826340682 failed closed before mutation because no enabled MFA-confirmed eligible publisher exists; Chromium was skipped
  - command: sanitized staging RBAC inspection
    result: PASS
    evidence: identity_total 2, identity_enabled 2, identity_mfa_confirmed 0, identity_wiki_publisher_eligible 0, role_assignment_total 0
blockers:
  - exactly one existing enabled staging Identity must complete genuine MFA confirmation through the normal account security flow
next_action: Complete MFA enrollment and confirmation on exactly one existing enabled staging account; do not provide the email, OTP, recovery code or MFA secret in GitHub or chat. After that, verify the sanitized confirmed-MFA count is exactly one, complete PR 213 validation, merge with the guarded marker and require an exact-SHA PASS report before cleanup.
```

## Notes

Production remains tracked exclusively by Issue #91. Final cleanup and task archival must occur only after the final exact-SHA report is PASS. The current blocker cannot be bypassed safely or autonomously because MFA confirmation is a user security ceremony.
