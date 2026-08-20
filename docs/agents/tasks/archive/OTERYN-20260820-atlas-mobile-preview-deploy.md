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

Deploy exact merged `Oteryn/Oteryn-Atlas@e462396a931652a62f61ca4e32c2402dfad9504a` to the existing LAN-only Synology FullWorld preview at `192.168.1.2:8097`, preserve every verified full-world data root, revalidate HTTP Range and mobile UI bytes, then remove all temporary Platform execution scaffolding.

## Acceptance criteria

- [x] Exact Atlas merged revision is served by the existing preview container.
- [x] Existing publication, semantic, pixel, overview, runtime-index and pixel-bucket roots remain unchanged.
- [x] HTTP 200, exact revision header, HTTP 206 and exact Content-Range are verified after cutover.
- [x] Deployed mobile HTML/CSS/module/proof bytes match the exact merged Atlas revision.
- [x] Cutover used fail-closed rollback protection.
- [x] Temporary Platform workflow modification was restored byte-for-byte after successful execution.
- [x] Task-owned runner checkout and merged source branches were cleaned up.
- [x] Atlas mobile UI had already passed exact-revision real-Chrome phone-layout proof before deployment.

## Ownership

```yaml
owned_paths: []
modules:
  - synology-staging-runner
  - atlas-fullworld-preview
dependencies:
  - Oteryn/Oteryn-Atlas#23 merged as e462396a931652a62f61ca4e32c2402dfad9504a
  - existing oteryn-staging self-hosted runner
blockers:
  - none
cross_repository_tasks:
  - Oteryn/Oteryn-Atlas#22 completed
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-20T15:52:00Z
head: fc98e78228f6016fe4aaaf382350552a287ff768
branch: main
pr: 1186
status: completed
context_routes:
  - synology-staging
  - github-only-execution
owned_paths: []
proven:
  - Atlas mobile fix PR 23 merged as e462396a931652a62f61ca4e32c2402dfad9504a after exact-head CI, Extraction Provenance, CodeQL and Mobile layout PASS
  - real Chrome Mobile layout run 32379326469 proved a full-width mobile map, working controls and inspector drawers, visible zoom controls and initialized mobile module on the exact Atlas candidate later merged as e462396a931652a62f61ca4e32c2402dfad9504a
  - Platform execution PR 1184 merged as 2e7f62d935b41cf388239dce0a7bcdd7ebfadaa8 after all required exact-head checks passed
  - first trusted-main deployment run 32386517537 failed before preview mutation because the persistent runner retained the exact task-owned atlas checkout
  - retry PR 1185 exact head 39f4e270c958a7c759a9646d2b6aa6f30ba78820 passed all required checks and merged as 6493df194266332ff83e3871afb86b97b0834268
  - trusted-main deployment run 32387502323 completed successfully on runner oteryn-synology-staging
  - deployment job 96485655609 verified the preview container running on 192.168.1.2:8097 at Atlas revision e462396a931652a62f61ca4e32c2402dfad9504a with restart policy unless-stopped
  - deployed fullworld HTML, CSS, mobile module and mobile proof bytes exactly matched Atlas e462396a931652a62f61ca4e32c2402dfad9504a
  - HTTP 200 and exact revision header passed; semantic Range returned HTTP 206, Content-Range bytes 100-199/72681462 and exactly 100 bytes
  - publication root sha256:9d0d2f3bb16a5a90f9b51a21366e4ed42963f5cb12366c404a20d9502ec4857f remained unchanged
  - semantic root sha256:27d7a83a7d9f498ea614b440ab4216cae5e6d11ea0527482410e40948cade5a9 remained unchanged
  - pixel root sha256:8b8228fcc4574903e547cb7d65b96f3d45e5a9e67045091c1bceb6e54d3690ad remained unchanged
  - overview root sha256:17683912d6758796d80a5b1647e2d0031f6849e51c40ae5264da6cfce3f9d6db remained unchanged
  - runtime-index root sha256:fa30ae5fc47f0ca8a6d482ed87b5db2cd74f32f7f523df16187ca719b8e04f08 remained unchanged
  - pixel-bucket root sha256:99cf23b01a0d652ff670a994a2b80cbef8d17036f514522d47f1aa98352d3116 remained unchanged
  - task-owned persistent-runner atlas checkout was removed by the successful deployment job
  - workflow-restoration PR 1186 exact head 36ca97be7c23df3fe37606e99a3a7eb4de5f1460 passed CI, CodeQL, Agent Governance, Edge, Game Auth, DB Outage and Phase 7 and merged as fc98e78228f6016fe4aaaf382350552a287ff768
  - repair-synology-autostart.yml on main is restored to original blob f3959e6bea09d39920db0e5515770a1ec77114ca
  - merged task branches for PRs 1185 and 1186 were verified absent after merge
derived:
  - the retained operational state is only the mobile-capable Atlas preview and its pre-existing verified data products; the temporary Platform deployment behavior has been removed
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: successful deployment and workflow-restoration lifecycle superseded all repaired intermediate failures
rejected_hypotheses:
  - first deployment failure changed Atlas runtime or roots: run 32386517537 failed before inspect, stage or cutover
  - live Chrome was required on the Synology runner to prove mobile bytes: deployed bytes match exact Atlas e462396a931652a62f61ca4e32c2402dfad9504a already proven by real Chrome run 32379326469
changed_paths:
  - .github/workflows/repair-synology-autostart.yml restored to pre-task state
  - docs/agents/tasks/active/OTERYN-20260820-atlas-mobile-preview-deploy.md moved to archive
validation:
  - command: Atlas PR 23 exact-head workflow suite
    result: PASS
    evidence: CI 32379326479, Extraction Provenance 32379326547, CodeQL 32379326474, Mobile layout 32379326469
  - command: Platform PR 1184 exact-head workflow suite
    result: PASS
    evidence: CI 32383644790, CodeQL 32383644723, Agent Governance 32383644773, Edge 32383644742, Game Auth 32383644762, DB Outage 32383645811, Phase 7 32383644774
  - command: first trusted-main Synology deploy run 32386517537
    result: FAIL
    evidence: job 96482452523 failed before mutation on persistent task-owned checkout residue
  - command: retry PR 1185 exact-head workflow suite
    result: PASS
    evidence: CI 32387163885, CodeQL 32387163640, Agent Governance 32387163684, Edge 32387163707, Game Auth 32387163783, DB Outage 32387163726, Phase 7 32387163750
  - command: trusted-main Synology deployment run 32387502323
    result: PASS
    evidence: job 96485655609 completed exact fetch, inspection/staging, cutover/verification and task-owned workspace cleanup
  - command: workflow-restoration PR 1186 exact-head workflow suite
    result: PASS
    evidence: CI 32388224709, CodeQL 32388224930, Agent Governance 32388224866, Edge 32388224759, Game Auth 32388224789, DB Outage 32388224820, Phase 7 32388224662
  - command: resulting main workflow identity
    result: PASS
    evidence: main fc98e78228f6016fe4aaaf382350552a287ff768 has repair-synology-autostart.yml blob f3959e6bea09d39920db0e5515770a1ec77114ca
blockers:
  - none
next_action: none
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: final archival PR is the ordinary same-repository lifecycle closeout for this completed task
source_branch_evidence: prior merged task branches were verified absent and repository auto-delete is active for the same squash-merge path
```

## Self-review

```yaml
self_review:
  result: PASS
  exact_head: fc98e78228f6016fe4aaaf382350552a287ff768
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
  evidence:
    - successful deployment retained all six verified Atlas roots
    - exact merged mobile UI bytes were served from Synology
    - temporary workflow behavior was removed after deployment
    - no raw legacy Atlas input was introduced
```

## E2E

`PASS`: Synology run `32387502323`, job `96485655609`, verified exact merged Atlas UI bytes, HTTP 200, exact revision header, HTTP 206 byte Range, exact Content-Range, all six roots, running preview container and retained restart policy. Real-Chrome mobile behavior is linked by exact byte identity to Atlas Mobile layout run `32379326469`.

## Terminal closeout

- Retained endpoint: `http://192.168.1.2:8097/web/fullworld.html`.
- Retained Atlas revision: `e462396a931652a62f61ca4e32c2402dfad9504a`.
- Live deployment evidence: Platform run `32387502323`, job `96485655609`, PASS.
- Temporary Platform workflow extension is absent from `main`; original workflow blob is restored.
- Task-owned persistent runner checkout was removed.
- Prior merged deployment/cleanup source branches were verified absent.
- Issue #1183 is the execution record and is ready for terminal closure after this archival move merges.
- Path ownership is released by this archival move.
