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

- [x] The first reviewed merge SHA built and published exact `sha-<full-sha>` Platform and Game Gateway images before deployment dispatch.
- [ ] The existing `Deploy Synology Staging` workflow deploys the repaired exact tag healthily with the previously proven immutable compatible Canary digest and LAN-only game configuration.
- [x] The failed attempt ran only from trusted `main`, environment `synology-staging` and runner label `oteryn-staging`.
- [x] The running Platform and Gateway containers from the failed attempt used the exact expected image tag with zero ambiguous service selection.
- [ ] Exactly one enabled MFA-confirmed Identity with all four exact Wiki permissions is selected internally without logging its email; zero or multiple eligible publishers fail closed.
- [ ] `wiki:launch-content:install` installs or verifies content version `2026-07-26.1` without overwriting editorial changes.
- [ ] A real Chromium instance on the Synology host network receives HTTP 200 from the localized homepage, Wiki index, EN/PL launch articles, sitemap and robots surfaces and verifies expected public headings/content.
- [x] The failed attempt's sanitized report records only commit SHA, image tag, workflow/run identifiers, job conclusions and artifact name.
- [ ] The temporary dispatcher and inert image-build trigger are removed after successful evidence is reconciled, the task is archived and Issue #145 closes only from exact deployment evidence.
- [x] No production, router, DSM, Internet-exposure, Canary/login-server repository or external-repository write occurs.

## Ownership

```yaml
owned_paths:
  - .github/workflows/one-shot-public-web-final-staging.yml
  - deploy/synology/.public-web-final-staging-trigger
  - deploy/synology/scripts/deploy.sh
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
updated_at: 2026-07-26T18:04:00Z
head: 2c9e4aa9313e465df8897f649dee10d6e4e4d514
branch: chore/OTERYN-20260726-public-web-final-staging-cleanup
pr: 212
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
  - final staging repair and later cleanup paths listed in Ownership
proven:
  - trusted main for the first observable attempt was 62dedb894ffc3af55ba10a0717a6a892b87f1370
  - one-shot run 30212540753 dispatched deployment run 30212567112 and deployment job 89821183195
  - exact Platform and Gateway images for sha-62dedb894ffc3af55ba10a0717a6a892b87f1370 were pulled and started
  - MariaDB, Redis and Canary were healthy; migrations, Passport keys, OAuth client configuration and all three database privilege checks passed
  - the deployment failed only after Gateway /ready remained HTTP 503 for the full readiness timeout; live Chromium smoke was skipped
  - Gateway /health returned 200 and Canary session dependency https://canary-session-internal:8444/health returned 200
  - Platform itself was healthy and listened on 0.0.0.0:8000, while the Gateway dependency request to https://platform-internal:8443/health returned 502
  - deploy.sh recreated Platform but used docker compose up -d internal-proxy gateway, which did not recreate the unchanged Nginx container
  - Nginx therefore retained the previous Platform container IP after Platform recreation
  - PR 212 changes deploy.sh to force-recreate only internal-proxy after Platform verification and starts Gateway after the refreshed proxy
  - PR 212 extends the guarded one-shot path filter to deploy.sh so its exact merge SHA receives new images and an observable deployment/live-smoke attempt
  - the temporary diagnostic workflow has been removed from the PR; exactly three durable files remain changed
  - no production, router, DSM, Internet-exposure or external-repository action occurred
derived:
  - recreating the private proxy after Platform is the narrow repair for Docker Compose container-IP churn without restarting database, Redis or Canary
  - a successful repaired run must produce a new exact-SHA PASS report before any closure or archive action
unknown:
  - exact repaired merge SHA and resulting one-shot/deployment run identifiers until PR 212 merges
  - whether staging contains exactly one eligible Wiki publisher Identity; this remains intentionally fail-closed
conflicts: []
first_failure:
  marker: stale-private-proxy-upstream
  evidence: deployment run 30212567112 ended with Health probe failed Gateway /ready; live dependency inspection proved Platform proxy 502 and Canary session 200
rejected_hypotheses:
  - the exact images were missing or invalid: both exact images pulled and the containers ran
  - Platform failed to bind: its Dockerfile and live health prove 0.0.0.0:8000 and Platform health passed
  - Canary session caused readiness failure: its private TLS health returned 200 with protocol_version 1
  - increasing the readiness timeout would fix the issue: the stale Nginx upstream persisted for the complete 4.5-minute loop
  - recreate the whole stack: only the private proxy holds stale upstream resolution and requires recreation
changed_paths:
  - .github/workflows/one-shot-public-web-final-staging.yml
  - deploy/synology/scripts/deploy.sh
  - docs/agents/tasks/active/OTERYN-20260726-public-web-final-staging-closure.md
validation:
  - command: first observable one-shot report
    result: FAIL
    evidence: run 30212540753 reported dispatch failure, deployment run 30212567112 and skipped live Chromium smoke
  - command: deployment job forensic inspection
    result: PASS
    evidence: job 89821183195 proves exact images and dependencies passed until Gateway readiness
  - command: live private dependency inspection
    result: PASS
    evidence: Platform proxy returned 502 while Canary session returned 200 from inside the exact Gateway container
  - command: bounded repair diff
    result: PASS
    evidence: deploy.sh replaces one combined compose up with force-recreate internal-proxy followed by up gateway
blockers: []
next_action: Run exact-head PR 212 validation, then merge with the guarded marker and require a new exact-SHA PASS report before cleanup.
```

## Notes

Production remains tracked exclusively by Issue #91. Final cleanup and task archival must occur only after the repaired exact-SHA report is PASS.