---
task_id: OTERYN-20260805-single-native-protocol-version
coordination_id: OTS-20260804-native-protocol-selection
status: validating
agent: "native protocol contract correction owner"
project_lane: native-gameplay-protocol
track: cross-repository-contract
branch: docs/OTERYN-20260805-single-native-protocol-version
base_branch: main
created: 2026-08-05T08:45:00+02:00
updated: 2026-08-05T08:50:00+02:00
risk: medium
related_prs: [527]
depends_on:
  - completed OTERYN-20260804-native-protocol-contract
  - completed OTERYN-20260804-native-protocol-producer
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260805-single-native-protocol-version.md
  - docs/architecture/adr/0011-single-native-protocol-version.md
  - docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_SINGLE_VERSION_AMENDMENT.md
  - docs/agents/prompts/OTS_NATIVE_PROTOCOL_SINGLE_VERSION_COMPLETION_AGENT.md
modules_touched:
  - native gameplay protocol contract
  - cross-repository implementation sequencing
shared_path_lease:
  - docs/architecture/adr/**
  - docs/contracts/**
  - docs/agents/prompts/**
feature_scope:
  type: documentation
  user_facing: false
  backend_required: false
  frontend_required: false
  integration_required: false
  e2e_required: false
execution_mode: github-only
run_scope: single_task
continuation_policy: stop_at_task_boundary
task_completion_policy: finalize_archive_and_continue
user_communication: terminal_only
---

# Goal

Record the owner decision that the initial native Oteryn gameplay implementation has exactly one canonical version and must not create a runtime or user-facing profile catalogue. Preserve a clean future extension path through an explicit later contract revision, and provide one autonomous cross-repository completion prompt that corrects the disabled producer and finishes the remaining Otheryn, Rust and E2E work.

# Acceptance criteria

- [x] A superseding ADR defines one native protocol version and no current profile dimension.
- [x] A contract amendment identifies every superseded `profile` assumption and the replacement identity.
- [x] Canary compatibility profiles remain explicitly out of scope and unchanged.
- [x] The existing disabled Platform/Gateway producer is labelled transitional until its `profile` field is removed.
- [x] Future variants remain possible only through a new reviewed contract revision; no placeholder profile field is retained in v1.
- [x] A complete autonomous three-repository implementation prompt is stored.
- [ ] Exact-head CI, review, merge, archive and lease release pass.

# Delivered paths

- `docs/architecture/adr/0011-single-native-protocol-version.md`;
- `docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_SINGLE_VERSION_AMENDMENT.md`;
- `docs/agents/prompts/OTS_NATIVE_PROTOCOL_SINGLE_VERSION_COMPLETION_AGENT.md`.

# Claim boundary

This task changes documentation only. The merged Platform/Gateway producer still contains transitional profile-oriented runtime/storage and remains disabled. The saved implementation prompt owns the correction and the remaining Otheryn, Rust and integrated E2E programme.

# Context checkpoint

```yaml
phase: exact-head-validation
status: validating
pull_request: 527
next_action: Validate exact changed paths and required workflows, perform a fresh consistency review, merge PR #527 when green, then archive the task and release leases.
```
