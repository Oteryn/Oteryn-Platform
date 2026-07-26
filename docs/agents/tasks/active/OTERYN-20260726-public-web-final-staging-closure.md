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

- [ ] The reviewed merge SHA builds and publishes exact `sha-<full-sha>` Platform and Game Gateway images before deployment dispatch.
- [ ] The existing `Deploy Synology Staging` workflow deploys that exact tag with the previously proven immutable compatible Canary digest and LAN-only game configuration.
- [ ] Deployment runs only on trusted `main`, environment `synology-staging` and runner label `oteryn-staging`.
- [ ] The deployed Platform and Gateway containers report the exact expected image tag with zero ambiguous service selection.
- [ ] Exactly one enabled MFA-confirmed Identity with all four exact Wiki permissions is selected internally without logging its email; zero or multiple eligible publishers fail closed.
- [ ] `wiki:launch-content:install` installs or verifies content version `2026-07-26.1` without overwriting editorial changes.
- [ ] A real Chromium instance on the Synology host network receives HTTP 200 from the localized homepage, Wiki index, EN/PL launch articles, sitemap and robots surfaces and verifies expected public headings/content.
- [ ] Sanitized evidence records only commit SHA, image tags, workflow/run identifiers, route/status assertions and timestamps.
- [ ] The temporary dispatcher is removed after evidence is reconciled, the task is archived and Issue #145 closes only from exact deployment evidence.
- [ ] No production, router, DSM, Internet-exposure, Canary/login-server repository or external-repository write occurs.

## Ownership

```yaml
owned_paths:
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/one-shot-public-web-final-staging.yml
  - docs/agents/ACTIVE_WORK.md
  - docs/agents/tasks/active/OTERYN-20260725-public-web-programme-closure.md
  - docs/agents/tasks/active/OTERYN-20260726-public-web-final-staging-closure.md
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
  - existing Deploy Synology Staging workflow
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-26T17:02:00Z
head: a262996eda36fc9430fe1883ea637ffd2f6ff698
branch: ops/OTERYN-20260726-public-web-final-staging-closure
pr: none
status: implementing
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
  - final staging closure paths listed in Ownership
proven:
  - trusted main is a262996eda36fc9430fe1883ea637ffd2f6ff698 after PR 209 archival merge
  - all Issue 145 implementation requirements are merged and exact-head validated
  - current build workflow publishes full-SHA Platform and Gateway tags on qualifying pushes to main
  - existing deployment workflow is workflow-dispatch-only, checks out trusted main, targets environment synology-staging and runner oteryn-staging
  - previous successful LAN staging deployment used immutable Canary image ghcr.io/blakinio/canary@sha256:784e5dbdcc64e311c48c51cd94aa206e2efa1e5eefb2f4ef40170d5aac55031f and private game bind 192.168.1.2
  - Wiki launch-content installation is idempotent and requires an enabled MFA-confirmed Identity with exact access, category, article and publish permissions
  - no production, router, DSM, Internet-exposure or external-repository action occurred while creating this task
 derived:
  - a temporary path-triggered one-shot workflow can safely wait for exact merge-SHA images and invoke the existing reviewed deployment workflow
  - a self-hosted follow-up job can verify exact container image tags, select one eligible publisher internally and run Chromium through a pinned Playwright container on the host network
unknown:
  - whether staging currently contains exactly one eligible Wiki publisher Identity
  - exact deployment and smoke run identifiers until the reviewed trigger merges
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - deploy the pre-archive PR 208 image as the final trusted main: the exact programme closure SHA must include durable archival reconciliation
  - log or hard-code a publisher email: identity data is unnecessary and zero/multiple eligible publishers must fail closed
  - treat CI image builds as live staging evidence: deployment and browser smoke remain separate required proof
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260726-public-web-final-staging-closure.md
validation:
  - command: repository, workflow, Issue 145 and prior deployment evidence reconciliation
    result: PASS
    evidence: current main, PRs 127/130/137/138/141, current build/deploy workflows and Wiki installer inspected through GitHub
blockers: []
next_action: Add the guarded one-shot deployment and live-browser-smoke workflow, wire exact-SHA image builds, then open a draft pull request.
```

## Notes

The temporary dispatcher is a reviewed staging-only mechanism and must be removed after exact evidence is retained. Production remains tracked exclusively by Issue #91.
