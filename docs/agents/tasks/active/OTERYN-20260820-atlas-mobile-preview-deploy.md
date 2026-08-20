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

- [x] Exact Atlas merged revision is served by the existing preview container.
- [x] Existing publication, semantic, pixel, overview, runtime-index and pixel-bucket roots remain unchanged.
- [x] HTTP 200, exact revision header, HTTP 206 and exact Content-Range are verified after cutover.
- [x] Deployed mobile HTML/CSS/module/proof bytes match the exact merged Atlas revision.
- [x] Cutover is fail-closed with rollback to the prior preview revision.
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
updated_at: 2026-08-20T15:47:00Z
head: 07752f40ac06fd310286c61b9fda1597c38c224a
branch: ops/atlas-22-mobile-preview-terminal-cleanup
pr: 1186
status: ready
context_routes:
  - synology-staging
  - github-only-execution
owned_paths:
  - .github/workflows/repair-synology-autostart.yml
  - docs/agents/tasks/active/OTERYN-20260820-atlas-mobile-preview-deploy.md
proven:
  - Atlas mobile fix PR 23 is merged as e462396a931652a62f61ca4e32c2402dfad9504a after exact-head CI, Extraction Provenance, CodeQL and Mobile layout PASS
  - Platform execution PR 1184 merged as 2e7f62d935b41cf388239dce0a7bcdd7ebfadaa8 after all required exact-head checks passed
  - first trusted-main deployment run 32386517537 failed before any preview mutation because the persistent runner retained the task-owned atlas checkout
  - retry PR 1185 exact head 39f4e270c958a7c759a9646d2b6aa6f30ba78820 passed CI, CodeQL, Agent Governance, Edge, Game Auth, DB Outage and Phase 7 and merged as 6493df194266332ff83e3871afb86b97b0834268
  - trusted-main deployment run 32387502323 completed successfully on runner oteryn-synology-staging
  - deployment job 96485655609 verified container running on 192.168.1.2:8097 with revision e462396a931652a62f61ca4e32c2402dfad9504a and restart policy unless-stopped
  - deployed fullworld HTML, CSS, mobile module and mobile proof bytes exactly matched Atlas e462396a931652a62f61ca4e32c2402dfad9504a
  - HTTP 200 and revision header passed; semantic Range returned HTTP 206, Content-Range bytes 100-199/72681462 and exactly 100 bytes
  - publication, semantic, pixel, overview, runtime-index and pixel-bucket roots remained exactly unchanged
  - runner did not provide Chrome, so live deployed byte equality is linked to the exact Atlas revision independently proven by real Chrome Mobile layout run 32379326469
  - task-owned persistent-runner atlas checkout was removed in the always-run reporting step
  - cleanup PR 1186 restores repair-synology-autostart.yml to pre-task blob f3959e6bea09d39920db0e5515770a1ec77114ca
derived:
  - the mobile UI is now the retained live preview while all temporary deployment behavior can be removed from Platform
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: successful deployment run 32387502323 supersedes the repaired pre-cutover failure
rejected_hypotheses:
  - deployment failure changed verified Atlas data roots: first failed run stopped before mutation and successful retry verified all roots unchanged
  - a new task-specific Platform workflow can be retained safely: workflow lifecycle budget is fixed at 53, so the existing registered workflow was reused temporarily
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
  - command: retry PR 1185 exact-head workflow suite
    result: PASS
    evidence: CI 32387163885, CodeQL 32387163640, Agent Governance 32387163684, Edge 32387163707, Game Auth 32387163783, DB Outage 32387163726, Phase 7 32387163750
  - command: trusted-main Synology deploy run 32387502323
    result: PASS
    evidence: job 96485655609 completed fetch, inspect/stage, cutover/verification and reporting successfully
  - command: cleanup PR 1186 exact-head checks
    result: NOT_RUN
    evidence: cleanup checkpoint update creates the final check generation
blockers:
  - none
next_action: Require PR 1186 exact-head checks to pass and squash-merge the workflow restoration, then archive this task and close Issue 1183.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: cleanup PR 1186 is the bounded final workflow-restoration PR for this task
source_branch_evidence: repository auto-delete removed both prior merged task branches and will apply to the same-repository cleanup PR after merge
```

## Notes

Issue #1183 is the Platform execution record. The retained operational deliverable is the Atlas preview container serving the new mobile-capable Atlas revision with the existing verified data products.
