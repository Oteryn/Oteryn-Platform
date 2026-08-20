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
updated_at: 2026-08-20T15:37:00Z
head: f12430d6e4ee5fbbf28aa48b223b4c5fecb9ffe0
branch: ops/atlas-22-mobile-preview-cleanup
pr: 1185
status: validating
context_routes:
  - synology-staging
  - github-only-execution
owned_paths:
  - .github/workflows/repair-synology-autostart.yml
  - docs/agents/tasks/active/OTERYN-20260820-atlas-mobile-preview-deploy.md
proven:
  - Atlas mobile fix PR 23 is merged as e462396a931652a62f61ca4e32c2402dfad9504a after exact-head CI, Extraction Provenance, CodeQL and Mobile layout PASS
  - Platform execution PR 1184 merged as 2e7f62d935b41cf388239dce0a7bcdd7ebfadaa8 after all required exact-head checks passed
  - first trusted-main deployment run 32386517537 reached the oteryn-staging runner; autostart repair passed and Atlas deployment failed before any preview mutation
  - failed deployment job 96482452523 stopped at Fetch exact merged Atlas revision because the persistent self-hosted workspace still contained the task-owned atlas checkout with an existing origin remote
  - existing Synology preview therefore remains on the pre-cutover verified revision because inspect, stage and cutover steps were skipped
  - retry workflow removes only the exact task-owned atlas workspace path before reinitializing it and removes that path again in the always-run reporting step
  - retry PR 1185 is open on branch ops/atlas-22-mobile-preview-cleanup
derived:
  - the failure is runner-workspace residue rather than Atlas code, data-root, Docker or preview-runtime failure
unknown: []
conflicts: []
first_failure:
  marker: persistent runner checkout residue
  evidence: deployment run 32386517537 job 96482452523 error remote origin already exists before any preview mutation
rejected_hypotheses:
  - deployment failed during Synology cutover: cutover and staging steps were skipped in run 32386517537
  - deployment failure changed verified Atlas data roots: no stage or cutover step ran
  - a new task-specific Platform workflow can be merged safely: workflow lifecycle validation rejected actual=54 budget=53
  - the Atlas repository can schedule the Platform Synology runner: Atlas one-shot run 32379921098 remained queued and was cancelled
changed_paths:
  - .github/workflows/repair-synology-autostart.yml
  - docs/agents/tasks/active/OTERYN-20260820-atlas-mobile-preview-deploy.md
validation:
  - command: Atlas PR 23 exact-head workflow suite
    result: PASS
    evidence: CI 32379326479, Extraction Provenance 32379326547, CodeQL 32379326474, Mobile layout 32379326469
  - command: Platform PR 1184 exact-head workflow suite
    result: PASS
    evidence: CI 32383644790, CodeQL 32383644723, Agent Governance 32383644773, Edge 32383644742, Game Auth 32383644762, DB Outage 32383645811, Phase 7 32383644774
  - command: trusted-main Synology deploy run 32386517537
    result: FAIL
    evidence: job 96482452523 failed before mutation because task-owned atlas directory retained an existing origin remote
  - command: retry PR 1185 exact-head Platform checks
    result: NOT_RUN
    evidence: new checkpoint commit requires a fresh exact-head check generation
blockers:
  - none
next_action: Require retry PR 1185 exact-head checks to pass, squash-merge it, then verify the resulting trusted-main Synology deployment run to terminal success.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: this task-owned retry branch carries only the persistent-runner repair and checkpoint needed to complete the same bounded deployment lifecycle
source_branch_evidence: pending
```

## Notes

Issue #1183 is the Platform execution record. No Platform product feature is being delivered; the retained operational deliverable is the Atlas preview container serving the new mobile-capable Atlas revision with the existing verified data products.
