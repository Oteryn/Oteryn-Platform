---
task_id: OTERYN-20260726-public-web-final-staging-closure
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/architecture/adr/0006-admin-rbac-and-audit-policy.md
  - docs/architecture/adr/0015-wiki-launch-content-provisioning.md
search_first:
  - Issue 145 closed state and final PASS evidence
  - active task archival ownership
  - Issue 91 production boundary
optional_reads:
  - docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md
  - docs/architecture/WIKI_IMPLEMENTATION_PLAN.md
---

# OTERYN-20260726-public-web-final-staging-closure

## Goal

Complete exact-SHA Synology staging acceptance for Issue #145, preserve sanitized evidence, remove temporary execution infrastructure and return the completed programme to the separate production boundary.

## Acceptance criteria

- [x] Exact trusted-main images were built and deployed through the reviewed Synology workflow.
- [x] QR-first MFA and protected enrollment were verified.
- [x] The genuine sole MFA-confirmed Identity passed guarded administrator bootstrap.
- [x] Exact Wiki role bundles were granted without wildcard authority.
- [x] Reviewed bilingual Wiki launch content was installed or verified idempotently.
- [x] Live Chromium verified the localized homepage, Wiki, EN/PL launch articles, sitemap and robots.
- [x] Sanitized exact-SHA PASS evidence was persisted.
- [x] Temporary one-shot workflows and trigger were removed.
- [x] Programme coordination task was archived.
- [x] Issue #145 was closed as completed after cleanup merge.
- [x] This final task record is archived in a documentation-only PR.
- [x] No production, router, DSM, Internet-exposure, Canary/login-server repository or external-repository write occurred.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20260726-public-web-final-staging-closure.md
modules:
  - Deployment
  - PublicPortal
  - Identity
  - Wiki
  - AdminRbac
  - Testing
  - AgentGovernance
dependencies:
  - closed Issue 145
  - Issue 91 production boundary
  - PR 230 merge d7984a2def655a01b513cdbc823117f37b90d5d4
  - PR 232 merge 415aa3febd04c8d9c61082d4a7451352bf084013
  - PR 233 merge e3e94dae03e0468d71f911ad41e597bb5d802eb3
  - PR 234 merge 4131a34b8c5f1092a2d0b8fb1bb56785f217b194
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T14:34:00+02:00
head: 21d40c1423d8a55612ec91863da1eb29beb639ff
branch: docs/OTERYN-20260727-final-staging-archive
pr: 235
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
  - docs/agents/tasks/archive/OTERYN-20260726-public-web-final-staging-closure.md
proven:
  - PR 230 merged guarded Wiki RBAC, genuine-MFA first-administrator bootstrap and reviewed launch-content installation as d7984a2def655a01b513cdbc823117f37b90d5d4
  - PR 232 merged named-volume Chromium smoke as 415aa3febd04c8d9c61082d4a7451352bf084013
  - exact image build 30263299006, deployment 30263361980 and one-shot 30263298962 succeeded
  - live-smoke job 89968912201 verified the sole eligible publisher and all six public Chromium assertions
  - Issue comment 5090951110 records Final Synology staging closure result PASS
  - cleanup PR 233 merged as e3e94dae03e0468d71f911ad41e597bb5d802eb3 after six required workflows passed
  - cleanup removed three temporary one-shot workflows and the inert trigger and archived the programme coordination task
  - Issue #145 closed as completed at 2026-07-27T12:09:42Z after final closure comment 5091097147
  - PR 234 merged as 4131a34b8c5f1092a2d0b8fb1bb56785f217b194 and persisted the closed-Issue checkpoint
  - PR 235 contains only the final task-record archive and ACTIVE_WORK closure
  - superseded PR 213 and audit PR 231 are closed without merge
  - no Identity email, password, TOTP secret, recovery code, production, router, DSM, Internet-exposure or external-repository action occurred
derived:
  - the public website expansion programme is complete at STAGING_PROVEN
  - the completed final-staging task lifecycle is ready for archival merge
  - Issue 91 remains the sole production execution tracker
unknown: []
conflicts: []
first_failure:
  marker: runner-container-bind-mount
  evidence: one-shot 30262167013 job 89965363040 failed before Playwright start because runner-local /tmp was invisible to the host Docker daemon; PR 232 replaced it with a named volume and passed
rejected_hypotheses:
  - application or content caused the first smoke failure: deployment, bootstrap and Wiki installation passed before Docker rejected the bind mount
  - another administrator or fabricated MFA was required: the sole genuine MFA-confirmed account passed the audited gate
  - staging PASS proves production: Issue 91 remains the explicitly separate production boundary
changed_paths:
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260726-public-web-final-staging-closure.md
  - docs/agents/tasks/archive/OTERYN-20260726-public-web-final-staging-closure.md
validation:
  - command: final exact-SHA Synology staging closure
    result: PASS
    evidence: build 30263299006, one-shot 30263298962, deployment 30263361980, live-smoke 89968912201 and comment 5090951110
  - command: cleanup PR 233 exact-head checks
    result: PASS
    evidence: Governance 30264539575, concurrency 30264539600, DB outage 30264539577, image build 30264539654, CI 30264539651 and Phase 7 30264539580
  - command: Issue 145 closure
    result: PASS
    evidence: state closed/completed at 2026-07-27T12:09:42Z after comment 5091097147
  - command: documentation archival review
    result: PASS
    evidence: archived record replaces the active record and ACTIVE_WORK no longer lists this task
blockers: []
next_action: Preserve this archived record as completion evidence.
```

## Notes

The programme is complete at `STAGING_PROVEN`. No staging evidence is promoted to `PRODUCTION_PROVEN`; Issue #91 remains the sole production execution tracker.
