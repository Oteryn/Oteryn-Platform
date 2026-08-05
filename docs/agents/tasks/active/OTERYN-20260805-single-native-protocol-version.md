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
updated: 2026-08-05T08:55:00+02:00
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
- [x] The existing disabled Platform/Game Gateway producer is labelled transitional until its `profile` field is removed.
- [x] Future variants remain possible only through a new reviewed contract revision; no placeholder profile field is retained in v1.
- [x] A complete autonomous three-repository implementation prompt is stored.
- [ ] Exact-head CI, review, merge, archive and lease release pass.

# Delivered paths

- `docs/architecture/adr/0011-single-native-protocol-version.md`;
- `docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_SINGLE_VERSION_AMENDMENT.md`;
- `docs/agents/prompts/OTS_NATIVE_PROTOCOL_SINGLE_VERSION_COMPLETION_AGENT.md`.

# Claim boundary

This task changes documentation only. The merged Platform/Game Gateway producer still contains transitional profile-oriented runtime/storage and remains disabled. The saved implementation prompt owns the correction and the remaining Otheryn, Rust and integrated E2E programme.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-05T06:55:00Z
head: d5046b049e3b439af86d72803e457c2f4a7b7d8f
branch: docs/OTERYN-20260805-single-native-protocol-version
pr: 527
status: validating
context_routes:
  - architecture
  - security
  - cross-repository
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260805-single-native-protocol-version.md
  - docs/architecture/adr/0011-single-native-protocol-version.md
  - docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_SINGLE_VERSION_AMENDMENT.md
  - docs/agents/prompts/OTS_NATIVE_PROTOCOL_SINGLE_VERSION_COMPLETION_AGENT.md
proven:
  - The current canonical contract and disabled producer contain a native profile dimension named oteryn.native.v1.
  - The product-owner decision requires one native protocol version and no current native profile catalogue.
  - ADR 0011, the normative amendment and the complete cross-repository implementation prompt are present on PR 527.
derived:
  - The existing disabled producer must be corrected before native activation.
  - Future variants can be introduced safely through a later contract and schema revision without a live v1 placeholder.
unknown:
  - Exact persisted candidate rows in every deployment environment; the implementation prompt requires direct verification before migration.
conflicts: []
first_failure:
  marker: governance-checkpoint-schema
  evidence: Agent Governance runs 30982620132 and 30982712536 rejected an incomplete checkpoint; this checkpoint supplies the required v1 fields.
rejected_hypotheses:
  - Keeping one configured profile is harmless: rejected because it preserves unused public, storage and downgrade complexity.
  - Renaming profile to variant now is sufficient: rejected because it retains the unused dimension.
changed_paths:
  - docs/agents/prompts/OTS_NATIVE_PROTOCOL_SINGLE_VERSION_COMPLETION_AGENT.md
  - docs/agents/tasks/active/OTERYN-20260805-single-native-protocol-version.md
  - docs/architecture/adr/0011-single-native-protocol-version.md
  - docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_SINGLE_VERSION_AMENDMENT.md
validation:
  - command: exact changed-path review
    result: PASS
    evidence: PR 527 changes exactly the four declared documentation paths.
  - command: independent contract and prompt consistency review
    result: PASS
    evidence: no native v1 profile field is permitted; Canary compatibility profiles and future contract evolution remain explicitly preserved.
blockers: []
next_action: Observe required workflows on the new exact head, mark PR 527 ready, merge when green, then archive the task and release leases.
```
