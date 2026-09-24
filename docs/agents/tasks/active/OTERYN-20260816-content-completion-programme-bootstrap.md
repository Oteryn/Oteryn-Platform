---
task_id: OTERYN-20260816-content-completion-programme-bootstrap
repository: blakinio/Oteryn-Platform
mode: documentation
task_kind: discovery
issue: 1115
programme: OTERYN_CONTENT_COMPLETION
project_lane: oteryn-platform-content
status: validating
implementation_authorized: false
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/PROMPTING_HANDOVER.md
  - docs/agents/PROMPTING_STANDARD.md
  - docs/agents/PROMPT_EVAL_STANDARD.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/PROJECT_LANES.json
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/agents/programs/GAME_CATALOG_PRODUCTION_COMPLETION_PROGRAM.md
search_first:
  - OTERYN_CONTENT_COMPLETION
  - GameCatalog Wiki PlayerCompanion
  - crystalserver content source inventory
optional_reads: []
---

# OTERYN-20260816-content-completion-programme-bootstrap

## Goal

Persist the owner-requested content-completion programme, provenance-aware CrystalServer source inventory, full re-audit prompt, parallel coordinator prompt and reusable execution-worker prompts without changing product runtime or claiming source/native authority.

## Acceptance criteria

- [x] Programme Issue #1115 exists and records owner intent, source hash, parallel roles and safety boundaries.
- [x] A durable programme document integrates with, rather than replaces, `OTERYN_PORTAL_COMPLETION` and `GAME_CATALOG_PRODUCTION_COMPLETION_PROGRAM`.
- [x] Owner-supplied archive identity and corrected bounded counts are recorded without committing the archive or bulk third-party content.
- [x] Source families are classified with provenance/authority caveats and `data-global` / `data-crystal` separation.
- [x] One discovery-auditor prompt explicitly performs a fresh complete player-visible audit and may create task-record-only draft PR scaffolds for independent READY slices.
- [x] One coordinator prompt allocates/monitors non-overlapping waves and verifies worker outcome rather than summaries.
- [x] Reusable worker prompts exist for source pipeline, Game Catalog core, Game Catalog extensions, Wiki/reference and Player Companion vertical slices.
- [x] Prompt changes include a documented manual scenario matrix and explicitly do not claim automated model trials.
- [x] Current live holds #1114 and #338 are recorded as generation-scoped and must be refreshed by future agents.
- [x] No server/game repository was accessed; only the owner-supplied ZIP and Platform repository were inspected.
- [x] No production/staging/protected-environment or owner-funded AI operation is authorized or performed.
- [ ] Exact final draft-PR head passes applicable documentation/governance CI.
- [ ] Whole-diff exact-head self-review is recorded before readiness/merge.
- [ ] Task lifecycle is archived only after this bootstrap PR merges and source branch closeout is verified.

## Ownership

```yaml
owned_paths:
  - docs/agents/programs/OTERYN_CONTENT_COMPLETION.md
  - docs/agents/prompts/OTERYN-CONTENT-COMPLETION-PROMPTS.md
  - docs/agents/reports/OTERYN-20260816-crystalserver-content-source-inventory.md
  - docs/agents/reports/OTERYN-20260816-crystalserver-content-source-inventory.json
  - docs/agents/evals/OTERYN-CONTENT-COMPLETION-MANUAL-EVAL.md
  - docs/agents/tasks/active/OTERYN-20260816-content-completion-programme-bootstrap.md
modules:
  - GameCatalog
  - Wiki
  - PlayerCompanion
  - agent programme coordination
dependencies:
  - Issue #1115
  - OTERYN_PORTAL_COMPLETION remains global selector
  - GAME_CATALOG_PRODUCTION_COMPLETION_PROGRAM remains specialized Game Catalog lifecycle
blockers:
  - none
cross_repository_tasks:
  - none; server/game repository access is not authorized
```

## Source evidence

```yaml
source_archive:
  file_name: crystalserver-main.zip
  sha256: 920a59e15175a5f53721f60b17f4bb37370bf0b61cd91abb4c909bf0d85e5f26
  archive_entries: 9486
  regular_files: 8819
  license_text_observed: GNU GPL v2
  default_datapack: data-global
  alternate_datapack: data-crystal
inspection_method: read-only local ZIP enumeration/XML parsing/bounded Lua text scanning
external_repository_access: none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-16T20:12:00+02:00
head: 9402e7f8de01d2ecc739b0f18a061743de85cd87
branch: docs/issue-1115-content-completion-programme
pr: 1116
status: validating
context_routes:
  - agent-governance
  - content
  - architecture
owned_paths:
  - docs/agents/programs/OTERYN_CONTENT_COMPLETION.md
  - docs/agents/prompts/OTERYN-CONTENT-COMPLETION-PROMPTS.md
  - docs/agents/reports/OTERYN-20260816-crystalserver-content-source-inventory.*
  - docs/agents/evals/OTERYN-CONTENT-COMPLETION-MANUAL-EVAL.md
  - docs/agents/tasks/active/OTERYN-20260816-content-completion-programme-bootstrap.md
proven:
  - Platform main at task start = 286efb1625d510c9d2cc344cb51a2438b31ebe48
  - open PRs observed at task start = #1114 and #338
  - owner-supplied source archive SHA-256 verified locally
  - source archive contains 9486 entries and 8819 regular files
  - config.lua.dist default datapack is data-global and data-crystal is alternate
  - root archive license text is GNU GPL v2
  - no external server/game repository was accessed
  - programme Issue #1115 created
  - bootstrap material commit = 9402e7f8de01d2ecc739b0f18a061743de85cd87
  - draft bootstrap PR #1116 created from the dedicated task branch
derived:
  - content completion requires a fresh audit because module AVAILABLE state does not prove real corpus/tool completeness
  - source families provide strong candidate material but do not establish native Oteryn authority
unknown:
  - exact deployed/staging content population for each target module
  - final publication-rights classification for all Tibia-derived prose/media fields
  - which implementation lanes remain READY after fresh auditor reconciliation
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - treat CrystalServer archive as automatic native source of truth
  - treat module AVAILABLE status as player-visible completeness
  - duplicate live PR #338 NPC/shop consumer ownership
changed_paths:
  - docs/agents/programs/OTERYN_CONTENT_COMPLETION.md
  - docs/agents/prompts/OTERYN-CONTENT-COMPLETION-PROMPTS.md
  - docs/agents/reports/OTERYN-20260816-crystalserver-content-source-inventory.json
  - docs/agents/reports/OTERYN-20260816-crystalserver-content-source-inventory.md
  - docs/agents/evals/OTERYN-CONTENT-COMPLETION-MANUAL-EVAL.md
  - docs/agents/tasks/active/OTERYN-20260816-content-completion-programme-bootstrap.md
validation:
  - command: GitHub exact-head docs/governance CI
    result: NOT_RUN
    evidence: draft PR #1116 created; first exact-head observation remains
blockers:
  - none
next_action: inspect the draft PR #1116 exact-head changed paths and applicable CI once; record self-review/validation without marking Ready or invoking owner-funded review
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is active and draft PR #1116 is not merged
source_branch_evidence: pending
```

## Notes

This task creates only documentation/governance/source-inventory metadata. Runtime/browser E2E is `NOT_APPLICABLE` because no executable user path is changed.