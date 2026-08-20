---
task_id: OTERYN-20260820-atlas-first-paint-preview-deploy
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

# OTERYN-20260820-atlas-first-paint-preview-deploy

## Goal

Deploy exact merged `Oteryn/Oteryn-Atlas@f99605a69981d9a1d2bca523aec3dff67a31e175` to the existing LAN-only Synology FullWorld preview at `192.168.1.2:8097`, preserving all verified data roots and then removing the temporary execution scaffolding.

## Acceptance criteria

- [ ] Exact Atlas revision `f99605a69981d9a1d2bca523aec3dff67a31e175` is served by `oteryn-atlas-fullworld-preview`.
- [ ] Publication, semantic, pixel, overview, runtime-index and pixel-bucket roots are unchanged.
- [ ] HTTP 200, exact revision header, HTTP 206 and exact Content-Range are verified after cutover.
- [ ] Deployed first-paint/runtime files match the exact Atlas revision byte-for-byte.
- [ ] Cutover is fail-closed with rollback to `e462396a931652a62f61ca4e32c2402dfad9504a`.
- [ ] Temporary Platform workflow extension and runner checkout are removed after success.
- [ ] Task branch/PR/task record are terminally cleaned up.

## Ownership

```yaml
owned_paths:
  - .github/workflows/repair-synology-autostart.yml
  - docs/agents/tasks/active/OTERYN-20260820-atlas-first-paint-preview-deploy.md
modules:
  - synology-staging-runner
  - atlas-fullworld-preview
dependencies:
  - Oteryn/Oteryn-Atlas#25 merged as f99605a69981d9a1d2bca523aec3dff67a31e175
  - existing oteryn-staging self-hosted runner
blockers:
  - none
cross_repository_tasks:
  - Oteryn/Oteryn-Atlas#24
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-20T16:48:00Z
head: 3e8e03242374530a20a13a012efb9caa7d1681b4
branch: ops/atlas-24-first-paint-preview-deploy
pr: none
status: implementing
context_routes:
  - synology-staging
  - github-only-execution
owned_paths:
  - .github/workflows/repair-synology-autostart.yml
  - docs/agents/tasks/active/OTERYN-20260820-atlas-first-paint-preview-deploy.md
proven:
  - Atlas Issue 24 is closed by PR 25 merged as f99605a69981d9a1d2bca523aec3dff67a31e175 after CI, Extraction Provenance, CodeQL, atlas-gate and real-Chrome WebGL proof passed on exact head 542c44e2a5d1080be011c4c86f8183424417a2d6
  - existing Synology preview currently serves Atlas e462396a931652a62f61ca4e32c2402dfad9504a at 192.168.1.2:8097
  - previous bounded Platform deployment lifecycle proved the existing repair-synology-autostart workflow can safely execute on runner oteryn-staging and was restored to blob f3959e6bea09d39920db0e5515770a1ec77114ca
derived:
  - reuse of the already-registered Synology workflow is the narrowest compliant execution path because the workflow budget is fixed and prior task-specific workflow creation exceeded it
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260820-atlas-first-paint-preview-deploy.md
validation:
  - command: Atlas PR 25 exact-head workflow suite
    result: PASS
    evidence: CI 32393691856, Extraction Provenance 32393691917, CodeQL 32393691930
blockers:
  - none
next_action: Add one push-only bounded Atlas deployment job to the existing registered Synology workflow, open the PR, pass exact-head checks and merge so trusted main performs the cutover.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: this branch is a bounded trusted-main deployment executor and must be removed after merge
source_branch_evidence: repository auto-delete is expected after merge and will be live-verified
```

## Notes

Issue #1188 is the Platform execution record. The retained deliverable is the existing Atlas preview container serving the repaired Atlas revision; Platform product behavior is not changed.
