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
- [ ] The temporary dispatcher and inert image-build trigger are removed after evidence is reconciled, the task is archived and Issue #145 closes only from exact deployment evidence.
- [ ] No production, router, DSM, Internet-exposure, Canary/login-server repository or external-repository write occurs.

## Ownership

```yaml
owned_paths:
  - .github/workflows/one-shot-public-web-final-staging.yml
  - deploy/synology/.public-web-final-staging-trigger
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
  - PR 210 merge a59a815472ab089572b6680a1f5fb4d9adcc3b44
  - existing Deploy Synology Staging workflow
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-26T17:24:00Z
head: 9b876522142589daa36baaacb01a0af3c5ebe350
branch: ops/OTERYN-20260726-final-staging-observable-report
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
  - trusted main is a59a815472ab089572b6680a1f5fb4d9adcc3b44 after PR 210 squash merge with the guarded trigger marker
  - all six required PR 210 exact-head workflows passed before merge
  - the push-triggered one-shot and deploy runs are intentionally separated from pull-request checks
  - the available GitHub connector cannot enumerate push or workflow-dispatch runs by SHA
  - the report-enabled workflow posts only exact SHA, release tag, one-shot run ID, deployment run ID, job conclusions and artifact name to Issue 145
  - the report job runs with always semantics and does not expose identity email, secrets, private storage paths or image bytes
  - the revised inert deploy trigger forces exact Platform and Gateway images for the report-enabled merge SHA
  - no production, router, DSM, Internet-exposure or external-repository action occurred while adding observability
 derived:
  - an Issue 145 status comment is the narrowest durable observation channel supported by both GitHub Actions and this connector
  - a fresh exact-SHA deployment is preferable to inferring the unobservable PR 210 post-merge outcome
unknown:
  - outcome and run identifiers of the initial unobservable PR 210 post-merge one-shot
  - exact report-enabled deployment and smoke run identifiers until the observability change merges
  - whether staging currently contains exactly one eligible Wiki publisher Identity
conflicts: []
first_failure:
  marker: connector-observability-boundary
  evidence: fetch_commit_workflow_runs returned no push/workflow-dispatch runs for a59a815472ab089572b6680a1f5fb4d9adcc3b44 and the connector exposes no repository workflow-run listing action
rejected_hypotheses:
  - infer success from the merge or image build: deployment and live Chromium evidence remain separate
  - expose administrator identity to make the result observable: only sanitized run metadata is necessary
  - rely on manual UI inspection: a reviewed issue-comment report keeps the workflow autonomous and auditable
changed_paths:
  - .github/workflows/one-shot-public-web-final-staging.yml
  - deploy/synology/.public-web-final-staging-trigger
  - docs/agents/tasks/active/OTERYN-20260726-public-web-final-staging-closure.md
validation:
  - command: PR 210 exact-head required workflows
    result: PASS
    evidence: Agent Governance, CI, concurrency, DB outage, Phase 7 and Synology image build passed on 551e927ac7568552aa83dea107e03ef21116b558
  - command: PR 210 merge
    result: PASS
    evidence: squash merge a59a815472ab089572b6680a1f5fb4d9adcc3b44 contains the required public-web-final-staging marker
  - command: workflow observation through connector
    result: BLOCKED
    evidence: connector supports PR-associated runs only and returned no runs for the merge SHA
blockers: []
next_action: Open and validate a narrow report-enabled rerun pull request, then merge it with the guarded marker and observe the resulting Issue 145 comment.
```

## Notes

The temporary dispatcher is a reviewed staging-only mechanism and must be removed after exact evidence is retained. Production remains tracked exclusively by Issue #91.
