---
task_id: OTERYN-20261011-decanary-final-1
governing_issue: 1474
required_reads:
  - docs/agents/programs/*_REMOVAL_PLAN.md
search_first:
  - docs/agents/LEGACY_TOKEN_BASELINE.json
optional_reads: []
---

# OTERYN-20261011 DECANARY-FINAL-1 remove provisioning and character-create stacks

## Goal

Governing GitHub Issue: #1474 (native-only migration) - canonical lifecycle authority for this task.

Owner mandate: remove the Platform game-account provisioning and character-create stacks (B1, partial D-9/D-10) and every consequence in config, CI, acceptance, docs and wording. Guard denylists and negative-test tokens stay; subsystems that still wait for a native replacement are listed, not renamed.

## Acceptance criteria

- [x] Registration creates no game-account binding; provisioning, character-create, verifier, command, connection, template and retry-flow code is deleted.
- [x] The game-catalog-contract cross-repository job, its integration test, trigger test and registry entry are deleted.
- [x] `docs/agents/LEGACY_TOKEN_BASELINE.json` lowered with the ratchet `--write`; report-only history groups kept at their previous values; no enforced group rises.
- [x] Touched PHPUnit, `python -m pytest tools/validation tests/ci deploy/synology/tests` and the governance validators pass.

## Ownership

```yaml
owned_paths:
  - app/**
  - config/database.php
  - routes/**
  - resources/views/**
  - lang/**
  - database/provisioning/**
  - scripts/acceptance/**
  - tests/**
  - services/game-gateway/internal/**
  - .github/workflows/**
  - .env.example
  - docs/**
modules:
  - Accounts
  - Identity
  - Characters
  - Documentation
dependencies:
  - Issue #1474
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-11T19:30:00Z
head: cf1ae574dd67e983ac6478aa7101c6f3c5afbd16
branch: claude/decanary-final-20261011
pr: 1549
status: completed
context_routes:
  - agent-governance
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20261011-decanary-final-1.md
proven:
  - Touched PHPUnit and the GameAuth suite pass; the single full-suite error was a witness-store write failure on a full local disk and does not reproduce in isolation.
derived: []
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - app/**
  - routes/**
  - tests/**
  - docs/**
  - docs/agents/LEGACY_TOKEN_BASELINE.json
  - docs/agents/tasks/archive/OTERYN-20261011-decanary-final-1.md
validation:
  - command: python tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: enforced groups fall; none rise
  - command: python -m pytest tools/validation tests/ci deploy/synology/tests
    result: PASS
    evidence: see PR 1549 CI
  - command: product-runtime-e2e
    result: NOT_APPLICABLE
    evidence: CI runs the full workspace.
blockers: []
next_action: Remaining hits wait on native replacements (public data, overview characters, Bazaar, prefs, DB schema, catalog wire fields); tracked in the removal plan.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: PR 1549 is the final authoring PR for this task and its source branch has no continuing ownership purpose after merge
source_branch_evidence: PR #1549 targets protected main; source branch deletes after merge
```
