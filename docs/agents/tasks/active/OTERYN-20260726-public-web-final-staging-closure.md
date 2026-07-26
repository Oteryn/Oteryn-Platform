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

Deploy the exact final Issue #145 trusted-main SHA to the existing Synology staging environment, provide QR-first genuine MFA enrollment, install the reviewed Wiki launch-content package through one eligible publisher and retain sanitized live Chromium smoke evidence without production, router, DSM, Internet-exposure or external-repository writes.

## Acceptance criteria

- [x] Exact trusted-main Platform and Gateway images are built before every guarded deployment.
- [x] The repaired deployment recreates the private proxy after Platform and completes healthily.
- [x] Trusted-main SHAs containing QR-first MFA deployed healthily through runs `30223164413` and `30223546917`.
- [ ] The deployment job itself verifies the local inline SVG TOTP QR, deployed view/CSS markers and protected anonymous MFA route.
- [ ] Exactly one enabled MFA-confirmed Identity with all four exact Wiki permissions is selected internally without logging its email; zero or multiple candidates fail closed.
- [ ] `wiki:launch-content:install` installs or verifies content version `2026-07-26.1` without overwriting editorial changes.
- [ ] A real Chromium instance on the Synology host network receives HTTP 200 from the localized homepage, Wiki index, EN/PL launch articles, sitemap and robots surfaces and verifies expected public headings/content.
- [x] Sanitized reports contain only commit SHA, image tag, workflow/run identifiers, job conclusions and artifact names.
- [ ] Temporary one-shot workflows and inert triggers are removed after successful evidence is reconciled, tasks are archived and Issue #145 closes only from exact deployment evidence.
- [x] No production, router, DSM, Internet-exposure, Canary/login-server repository or external-repository write occurs.

## Ownership

```yaml
owned_paths:
  - .github/workflows/one-shot-public-web-final-staging.yml
  - .github/workflows/one-shot-mfa-qr-staging-deploy.yml
  - deploy/synology/.public-web-final-staging-trigger
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/health-check.sh
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
  - existing Deploy Synology Staging workflow
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T01:12:00+02:00
head: a4ea7c7e5b5725d09cf25798522ddf2004b83060
branch: fix/OTERYN-20260727-mfa-qr-container-route-probe
pr: null
status: implementing
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
  - final staging, QR deployment and cleanup paths listed in Ownership
proven:
  - PR 214 merged QR-first local SVG TOTP enrollment and all required checks passed
  - PR 215 and PR 219 exact images built and deployed successfully without rollback
  - PR 220 moved QR verification into the deployment health check so it runs in the same self-hosted job
  - inline deployment run 30224342781 reached the QR checks and logged MFA QR renderer verified
  - deployed QR view and CSS marker checks completed before the final failing command
  - the only remaining failure was curl exit 7 from the containerized runner to its own 127.0.0.1 host binding
  - Platform itself listens on 127.0.0.1:8000 inside its own container namespace
  - the corrected check now uses PHP get_headers inside the Platform container, disables redirect following and requires an HTTP 302 status line
  - the synthetic non-user TOTP URI, QR bytes and any Identity data are not logged
  - no production, router, DSM, Internet-exposure or external-repository action occurred
derived:
  - the QR renderer and deployed assets are already proven on staging
  - probing the protected route from the same Platform namespace removes the runner-loopback mismatch without weakening the expected HTTP 302 boundary
  - a successful new deployment report proves exact images, standard health checks, QR rendering, deployed assets and anonymous MFA protection in one job
  - final Wiki publication remains intentionally fail-closed until exactly one existing enabled account completes genuine MFA
unknown:
  - container-local route-probe merge SHA and resulting one-shot/deployment run identifiers
  - post-deployment confirmed-MFA count
conflicts: []
first_failure:
  marker: runner-loopback-route-probe
  evidence: deployment run 30224342781 job 89852009913 logged MFA QR renderer verified and then exited 7 when curl targeted runner-local 127.0.0.1 instead of the Platform container namespace
rejected_hypotheses:
  - the QR renderer is broken: the deployment log explicitly recorded MFA QR renderer verified
  - the deployed QR view or CSS is absent: those fail-fast checks preceded the curl command and the observed exit code was curl-specific 7
  - exact images or core deployment health are broken: all services, bindings and standard health checks passed before the QR route probe
  - MFA can be fabricated for closure: genuine confirmation by one existing Identity remains required
changed_paths:
  - deploy/synology/.public-web-final-staging-trigger
  - deploy/synology/scripts/health-check.sh
  - docs/agents/tasks/active/OTERYN-20260726-public-web-final-staging-closure.md
validation:
  - command: PR 220 exact-head required checks
    result: PASS
    evidence: Governance, CI, concurrency, DB outage, Phase 7 and Synology image/package build passed on e493169a933cac4d61412a34c45c88fb9a98ac31
  - command: inline QR staging deployment
    result: FAIL
    evidence: deployment run 30224342781 passed the renderer and failed only on runner-local curl exit 7
blockers: []
next_action: Validate the three-file container-local route probe, merge it with marker [mfa-qr-staging], require a sanitized PASS report, then have exactly one existing staging account complete genuine MFA before resuming the guarded final Wiki closure.
```

## Notes

Production remains tracked exclusively by Issue #91. Final cleanup and task archival must occur only after exact-SHA QR deployment evidence and the later full final-staging closure report are PASS.
