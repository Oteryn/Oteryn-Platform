---
task_id: OTERYN-20260822-native-game-catalog-consumer-v1
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/contracts/OTERYN_V2_GAME_CATALOG_CONTENT_CONTRACT.md
  - docs/architecture/adr/0034-native-game-catalog-content-ownership.md
  - docs/agents/prompts/OTERYN-GAME-CATALOG-COMPLETION-AGENT.md
search_first:
  - game catalog native import validator
optional_reads: []
---

# OTERYN-20260822-native-game-catalog-consumer-v1

## Goal

Implement the first bounded Platform-side inactive consumer for the locked native Game Catalog envelope without touching legacy-Canary PR #338 paths or claiming unsupported native content families as implemented.

## Acceptance criteria

- [ ] Pin the merged Game contract `96ea673839f1d93190a40c17ae8036ac82096ded` and SHA-256 `9bc87fba5b565e5c7d4d3f6ca7a9bd75d45d8110de64a2a50f8f74d9ba181cad`.
- [ ] TDD RED precedes implementation of native envelope validation.
- [ ] Verify file/artifact integrity, fixed contract/schema/authority identity and canonical payload digest.
- [ ] Fail closed on unknown capability, duplicate identity, non-canonical order, dangling relation, invalid tombstone completeness and out-of-bound nested data.
- [ ] Preserve unsupported/partial/unknown capability semantics without persistence or authoritative absence.
- [ ] Validate an exact cross-repository fixture produced by the merged Game producer.
- [ ] Add no route, public/admin UI, import persistence, activation, deployment or production mutation.
- [ ] Focused tests, static/style checks and required exact-head CI pass.
- [ ] Whole-diff review has zero unresolved material findings.

## Ownership

```yaml
owned_paths:
  - app/GameCatalog/Application/Import/Native/**
  - tests/Feature/GameCatalog/NativeCatalogEnvelopeValidatorTest.php
  - tests/Fixtures/GameCatalog/native-v1/**
  - docs/agents/tasks/active/OTERYN-20260822-native-game-catalog-consumer-v1.md
modules:
  - GameCatalog
dependencies:
  - Oteryn/Oteryn-Game@0240f9586bff579aca58cdf5686b96886a76cc23
  - OTERYN-GAME-PLATFORM-CATALOG-V1 locked producer contract
blockers:
  - none
cross_repository_tasks:
  - OTERYN-GAME-PLATFORM-CATALOG-V1
```
## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-22T20:35:00Z
head: 20f8aac95ae1b890ec6ebe8a705dda7dfb6674d4
branch: feat/native-game-catalog-consumer-v1
pr: none
status: implementing
context_routes:
  - game-catalog
owned_paths:
  - app/GameCatalog/Application/Import/Native/**
  - tests/Feature/GameCatalog/NativeCatalogEnvelopeValidatorTest.php
  - tests/Fixtures/GameCatalog/native-v1/**
  - docs/agents/tasks/active/OTERYN-20260822-native-game-catalog-consumer-v1.md
proven:
  - Game producer contract is LOCKED at Game merge 96ea673839f1d93190a40c17ae8036ac82096ded
  - legacy PR #338 owns different existing GameCatalog paths and remains held
  - canonical Platform main is 20f8aac95ae1b890ec6ebe8a705dda7dfb6674d4
derived:
  - first mergeable Platform slice is native envelope validation only, not content persistence
unknown:
  - native capability-specific payload contracts remain absent for broad content families
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - reuse legacy schema 1.0.0 registry entry for native envelope; payload authority and shape differ
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260822-native-game-catalog-consumer-v1.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: implementation not started
blockers:
  - none
next_action: add native envelope behavior test and observe RED before production code
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is active
source_branch_evidence: pending
```

