# OTERYN-20260812 decoded Worldmap runtime capture

Task: `OTERYN-20260811-tibia-client-analysis`
Branch: `ops/oteryn-tibia-client-analysis-20260811`
PR: `#1006`

This file is the durable checkpoint for the live decoded-map runtime capture obtained on 2026-08-12. It contains only bounded structural evidence; no credentials, session secrets, account data, screenshots, Tibia binaries, or protected character data are stored here.

## PROVEN — live decoded map-field coordinate correlation

A post-authentication GDB capture against the already-running official client attached at the decoded Worldmap path without relogging. The capture produced **44 one-to-one `FIELD ↔ COORD` records** from the live session.

The records included map data from at least floors `z=7` and `z=6` in the same bounded sequence, proving that the captured field structures are tied to concrete world coordinates and are not merely UI/minimap pixels.

The client remained authenticated afterwards with:

- `ACTIVE_LOCAL_SOCKS_COUNT > 0`;
- `ACTIVE_DIRECT_TCP_COUNT=0`;
- `LIVE_SESSION_RETAINED=true`;
- canonical `oteryn-staging` container inventory unchanged.

## PROVEN — ordered field contents

Static tracing of the same decoded-map routine `0x19a8a80` showed:

- field content count is read from the field structure at generated-object offset `+0x38`;
- the repeated content pointer storage begins at `field+0x40`;
- the loop preserves content order;
- every content element is passed through the protobuf default-instance path associated with address `0x314b480` and then into the map-content builder `0xceca50`.

A live breakpoint inside that ordered contents loop captured deterministic records of the form:

`(x, y, z, idx, payload_ptr, value_at_payload_plus_0x30, kind_at_payload_plus_0x28)`

Thus the acceptance path `(x,y,z) -> ordered contents` is now runtime-proven.

One captured live field had five ordered elements with `idx=0..4` and `payload+0x30` values:

```text
4407, 313, 6379, 19394, 6217
```

These values are reported only as numeric runtime values here; no semantic label is assigned without evidence.

## PROVEN — AppearanceInstance path and factory

The binary contains the generated/runtime type names:

- `tibia::appearances::TAppearanceInstance`
- `tibia::appearances::TWorldMapAppearanceInstance`
- `tibia::appearances::TObjectAppearanceInstance`
- `tibia::appearances::TExpireObjectAppearanceInstance`

The default-instance candidate `0x314b480` is used on the same per-content path before the builder.

The live builder call chain was resolved as:

`0x19a8a80 -> 0xceca50 -> virtual target 0x762d30`

A live capture resolved the same factory target `0x762d30` for 64 sampled content records.

Inside `0x762d30`:

```text
0x762d55: mov 0x30(%rdx), %eax
0x762d6b: mov %ax, -0xaa(%rbp)
```

Therefore the value at `AppearanceInstance payload + 0x30` is read immediately by the concrete appearance factory path and narrowed to 16 bits before subsequent type-specific creation/lookup logic.

## PROVEN — semantic neighborhood around object appearance data

Embedded runtime/QML type metadata adjacent to the appearance classes contains the fields:

```text
objectID
objectCount
upgradeTier
liquidType
```

and explicitly names `TObjectAppearanceInstanceInfoQmlType` plus `TObjectAppearanceInstanceTypeID` / `ObjectAppearanceInstanceTypeID` in related appearance UI/runtime contexts.

This establishes that the runtime appearance subsystem exposes object/type IDs in the same class family.

## INFERENCE — `payload+0x30` is the concrete object/appearance type ID

Confidence: high, but not promoted to FACT yet.

Evidence supporting the inference:

1. `payload+0x30` is taken directly from the decoded `AppearanceInstance` content path.
2. It is narrowed to a 16-bit value immediately before the resolved appearance factory logic.
3. Live values are in the expected numeric range for Tibia object/type identifiers.
4. The surrounding runtime metadata for the same class family exposes `objectID`, `TObjectAppearanceInstanceTypeID`, and `ObjectAppearanceInstanceTypeID`.

What is still missing for a strict FACT label is one direct offset-to-field-name proof (for example generated protobuf accessor/descriptor metadata proving that generated offset `+0x30` is specifically `objectID`/type ID).

## Current acceptance status

The primary runtime objective is now **substantially proven** as:

`(x,y,z) -> ordered field contents -> concrete appearance-factory numeric IDs`

The only unresolved semantic detail is the exact official field name for the numeric value at `AppearanceInstance+0x30`.

Do not repeat login or world entry merely to preserve this evidence. The live session is no longer required for the already captured coordinate/order/factory proof.

## Exact next action

`next_action`: resolve the generated `AppearanceInstance+0x30` offset to its exact field name using static generated-code/descriptor metadata; do not require another authenticated runtime capture unless static evidence proves insufficient.

## Relevant workflow/commit evidence from this continuation

Successful bounded analyses/captures were produced while iterating `.github/workflows/tibia-client-analysis-dispatch.yml` on PR #1006, including:

- ordered map-field content capture;
- appearance-instance field/value-flow trace;
- live appearance factory target capture resolving `0x762d30`;
- disassembly of `0x762d30` proving the `+0x30` load and 16-bit narrowing;
- descriptor/runtime-string correlation around `objectID`, `objectCount`, `upgradeTier`, `liquidType` and the `TWorldMapAppearanceInstance` family.

Final durable checkpoint commit creating this report should be treated as authoritative repository state for continuation.
