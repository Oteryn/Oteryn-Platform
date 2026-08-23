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

- [x] Pin the merged Game contract `96ea673839f1d93190a40c17ae8036ac82096ded` and SHA-256 `9bc87fba5b565e5c7d4d3f6ca7a9bd75d45d8110de64a2a50f8f74d9ba181cad`.
- [x] TDD RED precedes implementation of native envelope validation.
- [x] Verify file/artifact integrity, fixed contract/schema/authority identity and canonical payload digest.
- [x] Fail closed on unknown capability, duplicate identity, non-canonical order, dangling relation, invalid tombstone completeness and out-of-bound nested data.
- [x] Preserve unsupported/partial/unknown capability semantics without persistence or authoritative absence.
- [x] Validate an exact cross-repository fixture produced by the merged Game producer.
- [x] Add no route, public/admin UI, import persistence, activation, deployment or production mutation.
- [x] Focused tests, static/style checks and required exact-head CI pass.
- [x] Whole-diff review has zero unresolved material findings.

## Ownership

```yaml
owned_paths:
  - app/GameCatalog/Application/Import/Native/**
  - tests/Feature/GameCatalog/NativeCatalogEnvelopeValidatorTest.php
  - tests/Fixtures/GameCatalog/native-v1/**
  - docs/agents/tasks/archive/OTERYN-20260822-native-game-catalog-consumer-v1.md
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
updated_at: 2026-08-23T07:36:08Z
head: 095017ba031bace2794865275c717422d11d82bc
branch: feat/native-game-catalog-consumer-v1
pr: 1229
status: completed
context_routes:
  - game-catalog
owned_paths:
  - app/GameCatalog/Application/Import/Native/**
  - tests/Feature/GameCatalog/NativeCatalogEnvelopeValidatorTest.php
  - tests/Fixtures/GameCatalog/native-v1/**
  - docs/agents/tasks/archive/OTERYN-20260822-native-game-catalog-consumer-v1.md
proven:
  - Game producer contract is LOCKED at Game merge 96ea673839f1d93190a40c17ae8036ac82096ded
  - legacy PR #338 owns different existing GameCatalog paths and remains held
  - final PR generation was rebased/merged onto canonical Platform main d2fd6af3104e9e40e29263c19cf4f78beb1037d0 before exact-head validation
  - exact implementation head 70bec44bf936c34be226303ee112a1ddb131bef3 passed Agent Governance 32625852864, Game Catalog Contract 32625852883 and CI 32625852854
  - PR #1229 squash-merged as 095017ba031bace2794865275c717422d11d82bc at 2026-08-23T07:36:08Z
  - source branch feat/native-game-catalog-consumer-v1 is absent from canonical remote after merge
derived:
  - first mergeable Platform slice is native envelope validation only, not content persistence
unknown:
  - native capability-specific payload contracts remain absent for broad content families
conflicts: []
first_failure:
  marker: tdd-red-missing-native-consumer-classes
  evidence: Game Catalog Contract run 32597729513 / static-analysis job 97091220173
rejected_hypotheses:
  - reuse legacy schema 1.0.0 registry entry for native envelope; payload authority and shape differ
changed_paths:
  - app/GameCatalog/Application/Import/Native/NativeCatalogContract.php
  - app/GameCatalog/Application/Import/Native/NativeCatalogEnvelopeValidator.php
  - app/GameCatalog/Application/Import/Native/ValidatedNativeCatalogSnapshot.php
  - tests/Feature/GameCatalog/NativeCatalogEnvelopeValidatorTest.php
  - tests/Fixtures/GameCatalog/native-v1/unsupported-snapshot.json
  - tests/Fixtures/GameCatalog/native-v1/unsupported-snapshot.json.sha256
  - docs/agents/tasks/archive/OTERYN-20260822-native-game-catalog-consumer-v1.md
validation:
  - command: Game Catalog Contract run 32597729513
    result: FAIL
    evidence: expected TDD RED on missing NativeCatalogEnvelopeValidator and NativeCatalogContract
  - command: Docker PHP 8.5 syntax check
    result: PASS
    evidence: all native consumer production/test PHP files parse cleanly
  - command: focused PHPUnit NativeCatalogEnvelopeValidatorTest
    result: PASS
    evidence: 14 tests, 58 assertions
  - command: Game Catalog Contract run 32625852883
    result: PASS
    evidence: final-head Pint, PHPStan and contract validation all passed
  - command: Agent Governance run 32625852864
    result: PASS
    evidence: final-head task/governance checks passed
  - command: CI run 32625852854
    result: PASS
    evidence: final-head classify-changes, runtime-tests, required test and platform-gate all passed
  - command: Phase 7 Production-Like Validation run 32625852843
    result: PASS
    evidence: final-head production-like validation passed without activation
  - command: Portal Acceptance Contract run 32625852857
    result: PASS
    evidence: final-head portal contract validation passed
  - command: whole-diff review
    result: PASS
    evidence: zero unresolved material findings after depth-bound and synthetic-object fixture repairs
blockers:
  - none
next_action: continue programme #330 only with capability adapters backed by canonical Game authority; legacy PR #338 remains held
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository feature branch after terminal PR merge
source_branch_evidence: PR #1229 merged as 095017ba031bace2794865275c717422d11d82bc and canonical remote no longer lists feat/native-game-catalog-consumer-v1
```

## Terminal closeout

- PR #1229 merged: `095017ba031bace2794865275c717422d11d82bc`.
- Final PR head: `70bec44bf936c34be226303ee112a1ddb131bef3`.
- Producer contract remains locked to Game merge `96ea673839f1d93190a40c17ae8036ac82096ded`.
- Completion claim remains `partial_consumer`; no native content family, persistence, activation or production publication is claimed.
- Legacy Canary PR #338 remains independently held.
