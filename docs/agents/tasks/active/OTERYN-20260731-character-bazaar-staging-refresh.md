---
task_id: OTERYN-20260731-character-bazaar-staging-refresh
policy_version: 2
project_lane: oteryn-platform-bazaar
task_kind: validation
decomposition_decision: single
classification: staging_enablement
required_reads:
  - AGENTS.md
  - docs/agents/AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/PROJECT_LANES.json
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/operations/MARKETPLACE_STAGING_ENABLEMENT.md
search_first:
  - current main, open PRs and active Marketplace/Synology ownership
  - PR #368, PR #370 and control run 30623491990
  - Build Synology Staging Images trigger contract
---

# OTERYN-20260731-character-bazaar-staging-refresh

## Goal

Deploy the current reviewed `main` release to the existing Synology staging environment through the permanent Character Bazaar staging control, preserving exact-image, least-privilege, reconciliation, scheduler and rollback gates.

## Acceptance criteria

- [ ] Exact Platform and Gateway images are published for the trusted-main merge SHA.
- [ ] Character Bazaar Staging Control completes `deploy-enable` successfully for that SHA.
- [ ] Sanitized evidence proves Marketplace enabled, exactly one scheduler, verified transfer privileges and reviewed unbound escrow.
- [ ] No production or Canary repository mutation occurs.
- [ ] Task is archived after exact evidence is recorded.

## Ownership

```yaml
owned_paths:
  - config/marketplace.php
  - docs/agents/tasks/active/OTERYN-20260731-character-bazaar-staging-refresh.md
  - docs/agents/tasks/archive/OTERYN-20260731-character-bazaar-staging-refresh.md
modules:
  - Character Bazaar staging operations
dependencies:
  - PR #368 staging control
  - control run 30623491990 baseline
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
phase: integrate
session_id: chat-20260731-character-bazaar-staging-refresh
session_role: operator
execution_mode: chat-github
execution_reason: existing reviewed workflow can be triggered and observed through narrow GitHub changes
updated_at: 2026-07-31T13:01:00Z
lease_expires_at: 2026-07-31T13:46:00Z
head: 3ae15819b623e49c37a7c66d3240577a4f03e191
branch: ops/OTERYN-20260731-character-bazaar-staging-refresh
pr: none
status: implementing
context_routes:
  - agent-governance
  - testing
owned_paths:
  - config/marketplace.php
  - docs/agents/tasks/active/OTERYN-20260731-character-bazaar-staging-refresh.md
  - docs/agents/tasks/archive/OTERYN-20260731-character-bazaar-staging-refresh.md
context_pressure: medium
context_growth: stable
context_score: 6
estimate_confidence: high
decomposition_decision: single
validation_level: focused
heavy_validation_runs: 0
session_rotation_count: 0
stale_takeover_count: 0
human_interruptions: 0
proven:
  - current main is 3ae15819b623e49c37a7c66d3240577a4f03e191
  - prior exact Character Bazaar staging deploy-enable run 30623491990 passed
  - build workflow publishes exact SHA images for config changes
  - control workflow deploys only when the trusted-main merge message contains the Character Bazaar staging marker
  - no open PR owns config/marketplace.php or the Character Bazaar control paths
derived:
  - one comment-only config refresh will trigger both exact image publication and the existing guarded control
unknown:
  - exact trusted-main merge SHA until PR merge
  - final control run and evidence artifact identifiers
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - rename staging as production
  - bypass the permanent Marketplace-aware rollback and reconciliation gate
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260731-character-bazaar-staging-refresh.md
validation:
  - command: repository and workflow contract inspection
    result: PASS
    evidence: current main workflows and PR/open-path search
blockers:
  - none
next_action: commit the comment-only Marketplace refresh marker and open the exact-head PR
```

## Notes

The user explicitly requested deployment to staging. Production remains outside this task.
