# ADR 0019: Preserve Canary runtime loot thresholds as contextual values

Status: Accepted
Date: 2026-07-29

## Context

Game Catalog schemas `1.0.0` and `1.1.0` model loot chance as a bounded rational numerator and denominator. Canary runtime data does not always contain that kind of value. It stores a configured threshold, applies runtime rate and context modifiers, and compares a loot roll against the adjusted threshold. The repository-default datapack contains reviewed runtime thresholds above the selected roll maximum.

Clamping those thresholds would lose source evidence. Increasing one denominator would change the meaning of ordinary records. Rendering threshold divided by roll maximum as a percentage could produce a false value above 100% and would still omit runtime modifiers.

## Decision

1. Preserve schemas `1.0.0` and `1.1.0` byte-for-byte.
2. Add schema `1.2.0` with loot chance model `canary_dynamic_threshold_v1`.
3. Replace the rational chance fields in schema 1.2 loot data with:
   - `chance_model`;
   - `chance_threshold`;
   - `roll_maximum`.
4. Define `chance_threshold` as the exact configured Canary runtime threshold and `roll_maximum` as the selected profile's declared base roll maximum.
5. Do not claim that threshold divided by roll maximum is one context-free effective probability. Runtime rate, schedule, dynamic and other contextual modifiers remain part of the versioned model semantics.
6. Persist legacy rational and new threshold models in distinct nullable column sets selected by `chance_model`.
7. Display threshold values as contextual configuration, never as a percentage.
8. Block migration rollback while threshold-model rows exist because the prior table cannot represent them.
9. Roll out Platform consumer support before Canary emits schema 1.2 snapshots.

## Consequences

- Default-datapack thresholds can be imported without dropping, clamping or inventing values.
- Existing schema 1.0/1.1 snapshots retain their rational representation and rollback behavior.
- Public and administrator readers can distinguish configured thresholds from probabilities.
- Older consumers reject schema 1.2 fail closed.
- A separate Canary producer task must pin byte-identical schema and fixture files and update exporter validation.
- Exact per-kill probability remains intentionally outside the snapshot unless a later model captures every relevant runtime context.

## Rejected alternatives

- Clamp threshold to roll maximum: loses configured runtime evidence.
- Increase a universal denominator: changes the meaning of ordinary thresholds and still omits runtime modifiers.
- Allow numerator above denominator in schema 1.1: silently changes pinned probability semantics.
- Publish a computed percentage: no single static percentage is proven for all runtime contexts.

## Related records

- `docs/architecture/adr/0016-versioned-game-catalog-snapshots.md`
- `docs/architecture/adr/0018-game-catalog-unknown-verified-boundary.md`
- `docs/contracts/GAME_CATALOG_IMPORT_CONTRACT.md`
- Platform task `OTERYN-20260729-game-catalog-runtime-threshold`
- Canary tasks `CAN-20260729-game-catalog-loot-integrity` and `CAN-20260729-game-catalog-loot-threshold-schema`
- Coordination ID `OTS-20260728-game-catalog-v1`
