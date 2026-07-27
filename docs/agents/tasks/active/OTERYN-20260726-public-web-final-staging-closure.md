---
task_id: OTERYN-20260726-public-web-final-staging-closure
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/architecture/adr/0006-admin-rbac-and-audit-policy.md
  - docs/architecture/adr/0015-wiki-launch-content-provisioning.md
  - deploy/synology/README.md
  - .github/workflows/deploy-synology-staging.yml
search_first:
  - Issue 145 final PASS evidence and cleanup state
  - active tasks and open pull requests touching final staging or task archival
  - production boundary tracked by Issue 91
optional_reads:
  - docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md
  - docs/architecture/WIKI_IMPLEMENTATION_PLAN.md
---

# OTERYN-20260726-public-web-final-staging-closure

## Goal

Complete exact-SHA Synology staging acceptance for Issue #145, preserve sanitized evidence, remove temporary execution infrastructure and hand the completed programme back to the production boundary without production, router, DSM, Internet-exposure or external-repository writes.

## Acceptance criteria

- [x] Exact trusted-main Platform and Gateway images were built and deployed through the existing reviewed Synology workflow.
- [x] QR-first MFA and the protected enrollment route were verified by deployment health checks.
- [x] Exactly one genuine MFA-confirmed staging Identity passed the fail-closed first-administrator gate.
- [x] Exact Wiki role bundles were granted without wildcard authority.
- [x] The audited one-time administrator bootstrap completed.
- [x] Reviewed Wiki launch content version `2026-07-26.1` was installed or verified idempotently.
- [x] A real Chromium instance verified the localized homepage, Wiki index, EN/PL launch articles, sitemap and robots surfaces.
- [x] Sanitized exact-SHA evidence was uploaded and Issue #145 received an explicit PASS report.
- [x] Temporary one-shot workflows and the inert trigger are removed in cleanup PR #233.
- [x] The programme coordination task is archived in cleanup PR #233.
- [ ] Issue #145 is closed after cleanup PR #233 merges.
- [ ] This final task record is archived after the closed-Issue checkpoint is persisted.
- [x] No production, router, DSM, Internet-exposure, Canary/login-server repository or external-repository write occurred.

## Ownership

```yaml
owned_paths:
  - .github/workflows/one-shot-public-web-final-staging.yml
  - .github/workflows/one-shot-public-web-final-staging-smoke-retry.yml
  - .github/workflows/one-shot-mfa-qr-staging-deploy.yml
  - deploy/synology/.public-web-final-staging-trigger
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260726-public-web-final-staging-closure.md
  - docs/agents/tasks/archive/OTERYN-20260726-public-web-final-staging-closure.md
  - docs/agents/tasks/active/OTERYN-20260725-public-web-programme-closure.md
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
  - Issue 91 production boundary
  - PR 230 merge d7984a2def655a01b513cdbc823117f37b90d5d4
  - PR 232 merge 415aa3febd04c8d9c61082d4a7451352bf084013
  - current cleanup PR 233
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T14:12:00+02:00
head: 6cc64ac32a2bfb8722bf0ce0aa4c3ce210e5c260
branch: chore/OTERYN-20260727-public-web-final-cleanup
pr: 233
status: ready
context_routes:
  - agent-governance
  - deployment
  - web-cms
  - identity
  - admin-rbac
  - security
  - testing
  - accessibility
owned_paths:
  - temporary final-staging workflows and trigger
  - ACTIVE_WORK and both public-web closure task lifecycles
proven:
  - PR 230 merged as d7984a2def655a01b513cdbc823117f37b90d5d4 after all seven exact-head checks passed
  - PR 230 deployment run 30262204007 succeeded and its live-smoke job completed the audited administrator bootstrap plus reviewed Wiki installation before a runner-filesystem harness failure
  - PR 232 merged as 415aa3febd04c8d9c61082d4a7451352bf084013 after all six exact-head checks passed
  - exact image build 30263299006, deployment 30263361980 and named-volume one-shot 30263298962 completed successfully
  - one-shot live-smoke job 89968912201 verified the sole eligible Wiki publisher and all six public Chromium assertions
  - Issue 145 comment 5090951110 records Final Synology staging closure result PASS for exact SHA 415aa3febd04c8d9c61082d4a7451352bf084013
  - superseded PR 213 and temporary audit PR 231 are closed without merge
  - cleanup PR 233 removes all three temporary one-shot workflows and the inert trigger
  - cleanup PR 233 archives the programme coordination task and narrows ACTIVE_WORK to this final lifecycle
  - Governance 30264318485, concurrency 30264318446, image build 30264318448, DB outage 30264318458, CI 30264318404 and Phase 7 30264318427 all passed on cleanup head 6cc64ac32a2bfb8722bf0ce0aa4c3ce210e5c260
  - no Identity email, password, TOTP secret, recovery code, production, router, DSM, Internet-exposure or external-repository action occurred
derived:
  - Issue 145 completion criteria are satisfied at STAGING_PROVEN level
  - cleanup PR 233 satisfies its merge gate
  - Issue 91 remains the only production execution tracker
unknown:
  - cleanup PR 233 merge SHA
  - Issue 145 closed timestamp
conflicts: []
first_failure:
  marker: runner-container-bind-mount
  evidence: one-shot 30262167013 job 89965363040 failed before Playwright start because runner-local /tmp was invisible to the host Docker daemon; PR 232 replaced it with a named Docker volume and passed
rejected_hypotheses:
  - application or content caused the first smoke failure: deployment, bootstrap and Wiki installation had already passed before Docker rejected the bind mount
  - a second administrator or fabricated MFA was required: the genuine sole MFA-confirmed account passed the audited bootstrap gate
  - staging PASS proves production: production remains explicitly unproven and tracked only by Issue 91
changed_paths:
  - .github/workflows/one-shot-public-web-final-staging.yml
  - .github/workflows/one-shot-public-web-final-staging-smoke-retry.yml
  - .github/workflows/one-shot-mfa-qr-staging-deploy.yml
  - deploy/synology/.public-web-final-staging-trigger
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260725-public-web-programme-closure.md
  - docs/agents/tasks/archive/OTERYN-20260725-public-web-programme-closure.md
  - docs/agents/tasks/active/OTERYN-20260726-public-web-final-staging-closure.md
validation:
  - command: PR 232 exact-head required checks
    result: PASS
    evidence: Governance 30263071287, concurrency 30263071235, image build 30263071244, DB outage 30263071236, CI 30263071245 and Phase 7 30263071256
  - command: final exact-SHA Synology staging closure
    result: PASS
    evidence: build 30263299006, one-shot 30263298962, deployment 30263361980, live-smoke job 89968912201 and Issue comment 5090951110
  - command: cleanup PR 233 exact-head checks
    result: PASS
    evidence: six required workflows passed on 6cc64ac32a2bfb8722bf0ce0aa4c3ce210e5c260
blockers: []
next_action: Merge cleanup PR 233, close Issue 145 from comment 5090951110 and persist the closed-Issue checkpoint.
```

## Notes

The public website expansion programme is complete at `STAGING_PROVEN`. No staging evidence is promoted to `PRODUCTION_PROVEN`; Issue #91 remains the sole production execution tracker.
