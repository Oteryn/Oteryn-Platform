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
- [x] Trusted-main SHA `348f483938fc8358132128fc79d229e38b98045b`, containing QR-first MFA, deployed healthily through run `30223164413`.
- [ ] The deployed Platform locally renders the TOTP provisioning URI as an inline SVG QR code while the anonymous MFA route remains protected.
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
  - existing Deploy Synology Staging workflow
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T00:35:00+02:00
head: 5f7d9a235aec9dc9725fc8d4832273d018f11f47
branch: fix/OTERYN-20260727-mfa-qr-staging-verifier
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
  - PR 212 repaired the stale private-proxy upstream and its deployment completed successfully
  - the earlier final closure failed closed because staging had zero MFA-confirmed Identities and zero eligible Wiki publishers
  - PR 214 merged QR-first local SVG TOTP enrollment as 671ac9fed05f51cc3989ff0aed2d37c99bc6d933
  - PR 215 merged the guarded QR deployment workflow as 348f483938fc8358132128fc79d229e38b98045b after all exact-head checks passed
  - exact image build run 30223101821 completed successfully
  - guarded Synology deployment run 30223164413 completed successfully without rollback
  - exact deployed Platform and Gateway image verification passed in one-shot run 30223101826
  - QR verification then failed before route/view checks because the verifier called nonexistent renderDataUri instead of the public dataUri method
  - App Identity Mfa MfaQrCode exposes dataUri(string) and rejects non-TOTP provisioning URIs
  - the corrected workflow now calls dataUri with the same synthetic non-user TOTP URI
  - no production, router, DSM, Internet-exposure or external-repository action occurred
derived:
  - the staging runtime is healthy and the observed failure is isolated to the verification harness API name
  - rerunning exact-SHA deployment with the corrected verifier is required before asking an operator to enroll genuine MFA
  - final Wiki publication remains intentionally fail-closed until exactly one existing enabled account completes genuine MFA
unknown:
  - corrected verifier merge SHA and resulting one-shot/deployment run identifiers
  - post-deployment confirmed-MFA count
conflicts: []
first_failure:
  marker: qr-verifier-api-name
  evidence: one-shot run 30223101826 verify job 89849342662 failed with undefined method App Identity Mfa MfaQrCode renderDataUri after exact images and deployment passed
rejected_hypotheses:
  - the exact images failed to build: image run 30223101821 passed
  - Synology deployment failed: deployment run 30223164413 passed without rollback
  - QR implementation is absent: the exact deployed image contains PR 214 and the verifier resolved the class before failing on its incorrect method name
  - MFA can be fabricated for closure: genuine confirmation by one existing Identity remains required
  - a synthetic publisher can install Wiki content: publisher selection remains exact-permission and MFA guarded
changed_paths:
  - .github/workflows/one-shot-mfa-qr-staging-deploy.yml
  - deploy/synology/.public-web-final-staging-trigger
  - docs/agents/tasks/active/OTERYN-20260726-public-web-final-staging-closure.md
validation:
  - command: PR 215 exact-head required checks
    result: PASS
    evidence: Governance, CI, concurrency, DB outage, Phase 7 and Synology image/package build passed on 7956b9b835ef3eb7167917b34dd0788951320c52
  - command: first QR staging exact-image deployment
    result: PASS
    evidence: image run 30223101821 and deployment run 30223164413 completed successfully
  - command: first QR staging bounded verifier
    result: FAIL
    evidence: job 89849342662 called nonexistent renderDataUri after exact-image verification passed
blockers: []
next_action: Validate the three-file verifier correction, merge it with marker [mfa-qr-staging], require a sanitized PASS report, then have exactly one existing staging account complete genuine MFA before resuming the guarded final Wiki closure.
```

## Notes

Production remains tracked exclusively by Issue #91. Final cleanup and task archival must occur only after exact-SHA QR deployment evidence and the later full final-staging closure report are PASS.
