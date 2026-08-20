---
task_id: OTERYN-20260820-atlas-mobile-preview-deploy
required_reads:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
  - docs/agents/CI_WORKFLOW_LIFECYCLE.md
search_first:
  - synology atlas preview
  - repair-synology-autostart
optional_reads: []
---

# OTERYN-20260820-atlas-mobile-preview-deploy

## Goal

Deploy exact merged `Oteryn/Oteryn-Atlas@e462396a931652a62f61ca4e32c2402dfad9504a` to the existing LAN-only Synology FullWorld preview at `192.168.1.2:8097`, preserve every verified full-world data root, revalidate HTTP Range and mobile UI bytes, then restore the temporary Platform execution workflow change.

## Acceptance criteria

- [ ] Exact Atlas merged revision is served by the existing preview container.
- [ ] Existing publication, semantic, pixel, overview, runtime-index and pixel-bucket roots remain unchanged.
- [ ] HTTP 200, exact revision header, HTTP 206 and exact Content-Range are verified after cutover.
- [ ] Deployed mobile HTML/CSS/module/proof bytes match the exact merged Atlas revision.
- [ ] Cutover is fail-closed with rollback to the prior preview revision.
- [ ] Temporary Platform workflow modification is restored after successful execution.
- [ ] Temporary branch/PR/task ownership is terminally cleaned up.

## Ownership

```yaml
owned_paths:
  - .github/workflows/repair-synology-autostart.yml
  - docs/agents/tasks/active/OTERYN-20260820-atlas-mobile-preview-deploy.md
modules:
  - synology-staging-runner
  - atlas-fullworld-preview
dependencies:
  - Oteryn/Oteryn-Atlas#23 merged as e462396a931652a62f61ca4e32c2402dfad9504a
  - existing oteryn-staging self-hosted runner
blockers:
  - none
cross_repository_tasks:
  - Oteryn/Oteryn-Atlas#22
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-20T15:00:00Z
head: b21322603c2d470fb19babbe88665b7377053fc1
branch: ops/atlas-22-mobile-preview-deploy
pr: 1184
status: validating
context_routes:
  - synology-staging
  - github-only-execution
owned_paths:
  - .github/workflows/repair-synology-autostart.yml
  - docs/agents/tasks/active/OTERYN-20260820-atlas-mobile-preview-deploy.md
proven:
  - Atlas mobile fix PR 23 is merged as e462396a931652a62f61ca4e32c2402dfad9504a after exact-head CI, Extraction Provenance, CodeQL and Mobile layout PASS
  - existing Synology preview is pinned to eee941de6d8b9b0d2217556a7a87f76ba67b602e and serves the verified full-world roots on 192.168.1.2:8097
  - Oteryn-Platform owns a repository-scoped oteryn-staging self-hosted runner with Docker access
  - workflow lifecycle budget is 53 and all 53 workflow files are registered on trusted main
derived:
  - the safest GitHub-only execution path is a temporary extension of the already-registered repair-synology-autostart workflow followed by byte-for-byte restoration
unknown: []
conflicts: []
first_failure:
  marker: workflow lifecycle budget
  evidence: PR 1184 CI run 32382783329 rejected a new 54th workflow as unregistered and over budget
rejected_hypotheses:
  - a new task-specific Platform workflow can be merged safely: workflow lifecycle validation rejected actual=54 budget=53
  - the Atlas repository can schedule the Platform Synology runner: Atlas one-shot run 32379921098 remained queued and was cancelled
changed_paths:
  - .github/workflows/repair-synology-autostart.yml
  - docs/agents/tasks/active/OTERYN-20260820-atlas-mobile-preview-deploy.md
validation:
  - command: Atlas PR 23 exact-head workflow suite
    result: PASS
    evidence: CI 32379326479, Extraction Provenance 32379326547, CodeQL 32379326474, Mobile layout 32379326469
  - command: Platform PR 1184 exact-head required checks
    result: NOT_RUN
    evidence: new final head created after hardening and task checkpoint persistence
blockers:
  - none
next_action: Require exact-head PR 1184 checks to pass, then squash-merge the temporary execution change so trusted main performs the Synology deployment.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: temporary trusted-main execution change must be removed after deployment and followed by a cleanup PR that restores the workflow
source_branch_evidence: pending
```

## Notes

Issue #1183 is the Platform execution record. No Platform product feature is being delivered; the retained operational deliverable is the Atlas preview container serving the new mobile-capable Atlas revision with the existing verified data products.