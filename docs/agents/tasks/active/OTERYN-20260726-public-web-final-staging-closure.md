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

Deploy the exact final Issue #145 trusted-main SHA to the existing Synology staging environment through a temporary reviewed one-shot dispatcher, install the exact reviewed Wiki launch-content package through one existing eligible publisher and retain sanitized live Chromium smoke evidence without production, router, DSM, Internet-exposure or external-repository writes.

## Acceptance criteria

- [x] The reviewed merge SHA builds and publishes exact `sha-<full-sha>` Platform and Game Gateway images before deployment dispatch.
- [x] The existing `Deploy Synology Staging` workflow deploys that exact tag with the previously proven immutable compatible Canary digest and LAN-only game configuration.
- [x] Deployment runs only on trusted `main`, environment `synology-staging` and runner label `oteryn-staging`.
- [x] The deployed Platform and Gateway containers report the exact expected image tag with zero ambiguous service selection.
- [x] Exactly one enabled MFA-confirmed Identity with all four exact Wiki permissions is selected internally without logging its email; zero or multiple eligible publishers fail closed.
- [x] `wiki:launch-content:install` installs or verifies content version `2026-07-26.1` without overwriting editorial changes.
- [x] A real Chromium instance on the Synology host network receives HTTP 200 from the localized homepage, Wiki index, EN/PL launch articles, sitemap and robots surfaces and verifies expected public headings/content.
- [x] Sanitized evidence records only commit SHA, image tags, workflow/run identifiers, route/status assertions and timestamps.
- [ ] The temporary dispatcher, inert image-build trigger and read-only evidence audit are removed after evidence is reconciled, the task is archived and Issue #145 closes only from exact deployment evidence.
- [x] No production, router, DSM, Internet-exposure, Canary/login-server repository or external-repository write occurs.

## Ownership

```yaml
owned_paths:
  - .github/workflows/one-shot-public-web-final-staging.yml
  - .github/workflows/inspect-public-web-final-staging-report.yml
  - deploy/synology/.public-web-final-staging-trigger
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260725-public-web-programme-closure.md
  - docs/agents/tasks/active/OTERYN-20260726-public-web-final-staging-closure.md
  - docs/agents/tasks/archive/OTERYN-20260726-public-web-final-staging-closure.md
  - docs/agents/tasks/archive/OTERYN-20260725-public-web-programme-closure.md
modules:
  - Deployment
  - PublicPortal
  - Wiki
  - Testing
  - AgentGovernance
dependencies:
  - Issue 145
  - PR 208 merge f8002191f0e5270dc4191227fd01d5e709ee5ab6
  - PR 209 merge a262996eda36fc9430fe1883ea637ffd2f6ff698
  - PR 210 merge a59a815472ab089572b6680a1f5fb4d9adcc3b44
  - PR 211 merge 62dedb894ffc3af55ba10a0717a6a892b87f1370
  - existing Deploy Synology Staging workflow
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-26T17:38:00Z
head: e3b7f7969537c89e982a221d44e9c3d2351ba00a
branch: chore/OTERYN-20260726-public-web-final-staging-cleanup
pr: none
status: validating
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
  - final staging cleanup and archive paths listed in Ownership
proven:
  - trusted main is 62dedb894ffc3af55ba10a0717a6a892b87f1370 after PR 211 squash merge with the report-enabled guarded marker
  - all six required PR 211 exact-head workflows passed before merge
  - Issue 145 contains a sanitized final-staging report for exact SHA 62dedb894ffc3af55ba10a0717a6a892b87f1370 with result PASS
  - the PASS report is emitted only after exact image-tag verification, successful existing-workflow deployment, reviewed Wiki content installation or verification and all six bounded live Chromium assertions
  - deterministic release tag is sha-62dedb894ffc3af55ba10a0717a6a892b87f1370
  - deterministic sanitized artifact name is public-web-final-staging-62dedb894ffc3af55ba10a0717a6a892b87f1370
  - the temporary audit workflow is read-only, searches only the sanitized Issue 145 marker and prints no identity email, secret, private path or image byte
  - no production, router, DSM, Internet-exposure or external-repository action occurred
 derived:
  - Issue 145 completion criteria are satisfied subject only to durable exact-ID reconciliation and temporary-workflow cleanup
  - the audit job output can preserve exact one-shot and deployment run IDs before its own workflow is removed
unknown:
  - exact one-shot and deployment run IDs until the temporary audit job output is read
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - infer success from image builds alone: the Issue 145 PASS report proves deployment and live Chromium smoke separately
  - retain temporary workflow files after closure: durable cleanup must remove one-shot, report trigger and read-only audit before final review
  - expose administrator identity to preserve evidence: exact workflow/run metadata is sufficient
changed_paths:
  - .github/workflows/inspect-public-web-final-staging-report.yml
  - docs/agents/tasks/active/OTERYN-20260726-public-web-final-staging-closure.md
validation:
  - command: PR 211 exact-head required workflows
    result: PASS
    evidence: Agent Governance, CI, concurrency, DB outage, Phase 7 and Synology image build passed on 92ab3059b351803e4dcee483d43eade0dddeec4b
  - command: report-enabled staging merge
    result: PASS
    evidence: PR 211 squash-merged as 62dedb894ffc3af55ba10a0717a6a892b87f1370 with the guarded marker
  - command: sanitized Issue 145 final-staging report
    result: PASS
    evidence: report exists for exact SHA 62dedb894ffc3af55ba10a0717a6a892b87f1370 and states Final Synology staging closure result PASS
blockers: []
next_action: Open the read-only audit and cleanup pull request, capture exact sanitized run IDs from its job log, then remove all temporary workflows/triggers and archive both completed tasks before final review.
```

## Notes

Production remains tracked exclusively by Issue #91. The final cleanup PR must contain no deployment trigger or read-only audit workflow at final review.