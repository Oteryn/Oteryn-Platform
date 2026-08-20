---
task_id: OTERYN-20260820-platform-stale-coordinate-terminal-reconciliation
status: validating
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: true
policy_version: 2
phase: validate
session_id: chatgpt-20260820-platform-coordinate-closeout
session_role: implementer-validator
execution_mode: github-actions-bounded-patch
execution_reason: reconcile only still-live mutable post-transfer coordinates from current protected main while preserving historical provenance
context_pressure: medium
context_growth: stable
context_score: 6
estimate_confidence: high
decomposition_decision: bounded
decomposition_reason: one current-main coordinate/provenance slice plus programme-state closeout
validation_level: full
heavy_validation_runs: 1
session_rotation_count: 0
stale_takeover_count: 0
human_interruptions: 0
issue: 1171
pr: 1181
branch: migration/issue-1171-terminal-reconciliation
base_sha: 8d9ee92336e7ba7a6a2cf1ed428241cb754f91cd
owned_paths:
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/agents/prompts/OTS_NATIVE_PROTOCOL_SINGLE_VERSION_COMPLETION_AGENT.md
  - docs/agents/prompts/OTS_NATIVE_SELECTION_E2E_IMPLEMENTATION.md
  - docs/agents/prompts/OTS_OTHERYN_NATIVE_PROTOCOL_IMPLEMENTATION.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/GAME_AUTH_ROLLOUT_PLAN.md
  - docs/architecture/GAME_CATALOG_ARCHITECTURE.md
  - docs/architecture/OTERYN_NATIVE_PROTOCOL_ROLLOUT.md
  - docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md
  - docs/contracts/GAME_CATALOG_1_3_NPC_SHOP_PROPOSAL.md
  - docs/contracts/GAME_CATALOG_IMPORT_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20260820-platform-stale-coordinate-terminal-reconciliation.md
---

# Platform stale-coordinate terminal reconciliation

## Objective

Close Issue #1171 from current protected `main` after the completed physical transfer and governance migration. Update only mutable current Platform/consumer/producer coordinates and mark obsolete executable prompts as historical; retain explicit source/history/audit provenance unchanged.

## Guardrails

- do not rewrite historical `source_repository`, `source_coordinate`, dated audit, evidence or archive provenance;
- do not change product/runtime/deployment/native-protocol semantics;
- do not re-open or repeat the completed physical transfer;
- current Platform authority becomes `Oteryn/Oteryn-Platform` and current Game producer routing becomes `Oteryn/Oteryn-Game` only where the durable topology owns that role;
- supersede stale PR #1172 after this clean successor passes exact-head validation;
- use trusted Platform `one_issue_one_owner_self_review` model with full exact-head self-review and required CI.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-20T08:20:00Z
head: 89edbf2271d2a44b3ce79af34bf558287ccdba4b
branch: migration/issue-1171-terminal-reconciliation
pr: 1181
status: validating
context_routes:
  - repository-migration
  - architecture
  - testing
owned_paths:
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/agents/prompts/OTS_NATIVE_PROTOCOL_SINGLE_VERSION_COMPLETION_AGENT.md
  - docs/agents/prompts/OTS_NATIVE_SELECTION_E2E_IMPLEMENTATION.md
  - docs/agents/prompts/OTS_OTHERYN_NATIVE_PROTOCOL_IMPLEMENTATION.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/GAME_AUTH_ROLLOUT_PLAN.md
  - docs/architecture/GAME_CATALOG_ARCHITECTURE.md
  - docs/architecture/OTERYN_NATIVE_PROTOCOL_ROLLOUT.md
  - docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md
  - docs/contracts/GAME_CATALOG_1_3_NPC_SHOP_PROPOSAL.md
  - docs/contracts/GAME_CATALOG_IMPORT_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20260820-platform-stale-coordinate-terminal-reconciliation.md
proven:
  - Platform physical transfer is terminally verified by PR 1164 merge a621a94d727be35ab73afe7d59f0e182cfd61356.
  - Safe post-transfer governance authority is terminally merged by PR 1178 as f24a682255073d8eaccb45b56c15cdf55754ee8e and archived by PR 1180 as 8d9ee92336e7ba7a6a2cf1ed428241cb754f91cd.
  - PR 1172 was authored against an older base and contains stale programme-state assumptions; its still-relevant coordinate edits are bounded and documentation-only.
  - Current canonical Platform repository is Oteryn/Oteryn-Platform at stable repository ID 1305155726.
  - Bounded reconciler run 32347203542 completed successfully; asserted substitutions, governance regression validation, native-protocol contract validation, native-protocol change-boundary validation and git diff check all passed before commit.
  - Reconciler implementation commit is 89edbf2271d2a44b3ce79af34bf558287ccdba4b and the temporary workflow was removed on 2bf4be10471b799966bb4004f422d161b4c853a5.
derived:
  - A clean current-main successor is safer than merging the stale PR 1172 branch.
  - Runtime E2E is NOT_APPLICABLE because the final scope is documentation/programme/contract coordinate routing only.
unknown: []
conflicts: []
first_failure:
  marker: stale-pr-base
  evidence: PR 1172 base predates provider transfer/governance closeout and its programme state no longer matches current main
rejected_hypotheses:
  - Historical provenance should be globally rewritten to the target coordinate.
  - The completed physical transfer needs to be repeated.
changed_paths:
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/agents/prompts/OTS_NATIVE_PROTOCOL_SINGLE_VERSION_COMPLETION_AGENT.md
  - docs/agents/prompts/OTS_NATIVE_SELECTION_E2E_IMPLEMENTATION.md
  - docs/agents/prompts/OTS_OTHERYN_NATIVE_PROTOCOL_IMPLEMENTATION.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/GAME_AUTH_ROLLOUT_PLAN.md
  - docs/architecture/GAME_CATALOG_ARCHITECTURE.md
  - docs/architecture/OTERYN_NATIVE_PROTOCOL_ROLLOUT.md
  - docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md
  - docs/contracts/GAME_CATALOG_1_3_NPC_SHOP_PROPOSAL.md
  - docs/contracts/GAME_CATALOG_IMPORT_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20260820-platform-stale-coordinate-terminal-reconciliation.md
validation:
  - command: bounded reconciler run 32347203542
    result: PASS
    evidence: exact asserted coordinate/provenance transforms and all embedded governance/native-protocol validators passed before commit
  - command: runtime E2E
    result: NOT_APPLICABLE
    evidence: no product/runtime/deployment behavior changes; documentation/programme/contract coordinate routing only
  - command: final exact-head Agent Governance and CI
    result: NOT_RUN
    evidence: final user-authored cleanup/checkpoint head is being validated after temporary workflow removal
blockers: []
next_action: Require fresh exact-head Agent Governance and CI on the cleaned PR, perform full-diff self-review and review hygiene, then mark Ready and squash-merge only if every required gate passes.
```
