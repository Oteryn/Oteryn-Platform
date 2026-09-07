# Oteryn Game Catalog Completion Agent

```yaml
prompt_contract:
  version: 2.0
  objective: Advance Issue #301 and current Game Catalog consumers as far as proven Platform authority permits without duplicating held PR #338 or inventing producer truth.
  baseline_version: 1.0
  eval_suite: docs/agents/evals/oteryn-platform-parallel-wave-prompts-v1.json
  rollback_version: 1.0
  required_invariants:
    - reuse_held_consumer
    - retrieved_evidence_not_producer_truth
    - partial_consumer_not_complete_feature
owner_alias: OTERYN-GAME-CATALOG-COMPLETION
```

## Outcome

Resolve Issue #301, the current production-completion programme, content coordination and PR #338 from live state. Deliver the largest truthful mergeable Platform-side Game Catalog slice supported by accepted producer contracts and provenance.

PR #338 is the existing held schema 1.3 NPC/shop consumer. Do not duplicate its paths or merge it without exact producer compatibility with its pinned contract. Reuse or take it over only when live ownership permits.

## Scope and domain constraints

Writes are limited to `Oteryn/Oteryn-Platform`. Do not access another server/game repository without separate exact authority. No production/staging activation, live import, invented server facts, third-party Wiki authority, credentials or live user effects are authorized.

Read `GAME_CATALOG_ARCHITECTURE.md`, `GAME_CATALOG_IMPORT_CONTRACT.md`, the current catalogue programme, schema registry/tests and only the source needed by the selected slice.

A source archive is reference evidence, not native gameplay authority. Preserve `UNKNOWN` when producer facts are absent. Implement only entity/relation facts whose producer contract, exact revision and provenance are established from permitted evidence.

## Acceptance delta

- Current schema, producer/consumer ownership and rollout order are proven.
- PR #338 is intentionally reused, held or terminal and never duplicated.
- Each selected family has exact source revision, provenance, completeness and availability evidence.
- Imports remain deterministic, bounded, transactional, inactive by default and rollback-safe.
- Unknown or unsupported producer facts fail closed.
- Consumer-only work says `partial_consumer`; it does not claim a complete feature.
- A delivered public/admin family includes its real query/route, EN/PL states, integration and applicable browser E2E.
- Related content ownership remains non-overlapping.

Select the first independently mergeable Platform slice whose producer contract is available. Reuse existing schema/import boundaries. Verify generated/imported artifacts and reachable consumers from the resulting environment.

## Stop delta

Stop for missing producer authority/evidence, the held #338 dependency, ownership overlap, production activation or external-repository access. Record the exact missing contract or decision and one next action; do not weaken a schema or test to bypass the hold.
