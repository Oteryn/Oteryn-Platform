---
task_id: OTERYN-20260726-mfa-qr-staging-deploy
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/tasks/active/OTERYN-20260726-public-web-final-staging-closure.md
  - .github/workflows/one-shot-public-web-final-staging.yml
  - .github/workflows/build-synology-staging-images.yml
search_first:
  - active tasks and open pull requests touching Synology staging or MFA QR deployment
optional_reads:
  - deploy/synology/README.md
---

# OTERYN-20260726-mfa-qr-staging-deploy

## Goal

Deploy the merged QR-first MFA implementation to the existing Synology staging environment through the reviewed final-staging one-shot without production or external-repository writes.

## Acceptance criteria

- [ ] The exact trusted-main merge containing PR #214 is included in the deployment SHA.
- [ ] Exact Platform and Gateway images are built and published.
- [ ] The guarded Synology staging deployment completes healthily.
- [ ] The staging MFA enrollment page exposes the local QR-first flow.
- [ ] The final-staging workflow remains fail-closed for zero or multiple eligible Wiki publishers.
- [ ] No production, router, DSM, Internet-exposure or external-repository write occurs.

## Ownership

```yaml
owned_paths:
  - .github/workflows/one-shot-public-web-final-staging.yml
  - deploy/synology/.public-web-final-staging-trigger
  - deploy/synology/.mfa-qr-staging-deploy-pr
  - docs/agents/tasks/active/OTERYN-20260726-mfa-qr-staging-deploy.md
  - docs/agents/tasks/archive/OTERYN-20260726-mfa-qr-staging-deploy.md
modules:
  - Deployment
  - Identity
  - Security
  - Testing
dependencies:
  - PR 214 merge 671ac9fed05f51cc3989ff0aed2d37c99bc6d933
  - existing final-staging one-shot and Synology deployment workflow
blockers: []
cross_repository_tasks: []
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-27T00:15:00+02:00
head: f3bc393c049e384a9713278ff75fd1893e43f2e2
branch: chore/OTERYN-20260726-mfa-qr-staging-deploy
pr: null
status: implementing
context_routes:
  - agent-governance
  - deployment
  - identity
  - security
  - testing
owned_paths:
  - paths listed in Ownership
proven:
  - PR 214 merged QR-first local SVG TOTP enrollment as 671ac9fed05f51cc3989ff0aed2d37c99bc6d933
  - all required PR 214 checks passed on exact head aa49338225a5a3cb5917681e9ddd385f1f081327
  - the existing reviewed one-shot deploys only trusted main exact-SHA images to Synology staging
  - the deployment does not modify production, router, DSM or external repositories
derived:
  - a new trusted-main merge is required so image tags include the QR implementation and the one-shot push filter is satisfied
unknown:
  - final deployment merge SHA and run identifiers
  - post-deployment confirmed-MFA count
conflicts: []
first_failure:
  marker: qr-not-yet-on-staging
  evidence: PR 214 is merged to main but its exact image has not yet been deployed to Synology staging
rejected_hypotheses:
  - manual secret transcription is acceptable as the primary path: QR-first enrollment has now been merged
  - MFA can be fabricated for closure: the workflow must require genuine user confirmation
changed_paths:
  - .github/workflows/prepare-mfa-qr-staging-retry.yml
  - deploy/synology/.mfa-qr-staging-deploy-pr
  - docs/agents/tasks/active/OTERYN-20260726-mfa-qr-staging-deploy.md
validation: []
blockers: []
next_action: Open the bounded deployment PR, let the one-shot preparation self-remove, validate the exact head, then merge with the guarded staging marker.
```
