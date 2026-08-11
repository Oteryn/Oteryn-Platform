---
task_id: OTERYN-20260811-dependabot-actions-major-cleanup
mode: implementation
branch: chore/dependabot-actions-major-cleanup
status: implementing
project_lane: oteryn-platform-core
---

# OTERYN-20260811 remaining Dependabot Actions major cleanup

## Goal

Finish the old #955-#958 dependency queue on current protected `main` without reverting the later #989/#997 work. Deliver one coherent current-main wave for `docker/build-push-action` 6→7, `actions/setup-node` 4→7, `actions/checkout` 5/6→7 on the bot-owned surfaces, and `docker/metadata-action` 5→6.

## Acceptance

- [x] #955-#958 are no longer open standalone bot PRs and their exact staged replacements were preserved in `deps/actions-node24-wave`.
- [x] GitHub generated a conflict-free current-main merge tree `0da0b993e3e509bb8318db66c4667c3913e2d831`; compare against main `681455739a054f344dc0e9478ff79821ac4a401d` contains exactly 19 workflow files and only action-version substitutions.
- [ ] Reconcile any additional old checkout occurrence added by later #989 inside those same owned workflow files.
- [ ] Remove temporary reconciliation tooling before final readiness.
- [ ] Pass exact-head workflow/runner compatibility validation and final review.
- [ ] Merge to protected main and archive this task.

## Ownership

```yaml
owned_paths:
  - .github/workflows/acceptance-validation.yml
  - .github/workflows/announcements-acceptance.yml
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/community-data-acceptance.yml
  - .github/workflows/content-scale-acceptance.yml
  - .github/workflows/deep-system-validation.yml
  - .github/workflows/downloads-acceptance.yml
  - .github/workflows/editorial-media-acceptance.yml
  - .github/workflows/error-state-acceptance.yml
  - .github/workflows/events-acceptance.yml
  - .github/workflows/native-auth-canary-cache-build.yml
  - .github/workflows/native-auth-ephemeral-cutover-rehearsal.yml
  - .github/workflows/native-protocol-contract-audits.yml
  - .github/workflows/native-protocol-contract.yml
  - .github/workflows/portal-acceptance-contract.yml
  - .github/workflows/portal-exhaustive-audit.yml
  - .github/workflows/support-legal-acceptance.yml
  - .github/workflows/support-moderation-acceptance.yml
  - .github/workflows/wiki-reconciliation-acceptance.yml
  - .github/workflows/dependabot-actions-major-reconcile.yml
  - docs/agents/tasks/active/OTERYN-20260811-dependabot-actions-major-cleanup.md
  - docs/agents/tasks/archive/OTERYN-20260811-dependabot-actions-major-cleanup.md
modules:
  - github-actions-ci
dependencies:
  - terminal Issue #691 / PR #989 Actions compatibility evidence
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-11T12:50:00Z
head: 681455739a054f344dc0e9478ff79821ac4a401d
branch: chore/dependabot-actions-major-cleanup
pr: none
status: implementing
context_routes:
  - ci-repair
  - testing
owned_paths:
  - the exact workflow/task paths declared above
proven:
  - Current protected main is 681455739a054f344dc0e9478ff79821ac4a401d.
  - Current main still contains checkout v5/v6, setup-node v4, build-push v6 and metadata v5 on the #955-#958 surfaces; the upgrades are not yet delivered to main.
  - GitHub-generated merge 0da0b993e3e509bb8318db66c4667c3913e2d831 cleanly combines staged #955-#958 changes with current main and changes exactly 19 workflow files.
  - PR #989 separately proved Node 24-era Actions compatibility and self-hosted runner 2.336.0; this task does not reopen Issue #691.
  - Active production-verification task owns only its own task document and does not own these workflow paths.
derived:
  - The remaining old bot upgrades can be delivered as one current-main workflow-only wave without reverting #989/#997.
unknown:
  - Whether #989 introduced additional old checkout markers inside the same owned workflow files beyond the staged #957 substitutions.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - #989 already delivered #955-#958; current main build workflow still shows checkout v5, metadata v5 and build-push v6.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260811-dependabot-actions-major-cleanup.md
validation:
  - command: current-main merge-tree compare
    result: PASS
    evidence: merge 0da0b993e3e509bb8318db66c4667c3913e2d831 is conflict-free and changes exactly 19 staged workflow paths.
blockers:
  - none
next_action: Reconcile old action markers inside the exact owned workflow set, remove temporary tooling, then validate the frozen current-main candidate.
```
