# Game Catalog 1.3 Compatibility and Rollout Proposal

Status: architecture proposal only  
Programme: `GAME-CATALOG-PRODUCTION-COMPLETION`

## Compatibility matrix

| Producer | Consumer | Expected result |
|---|---|---|
| Canary `1.0.0` | current Platform | accepted under retained `1.0.0` contract |
| Canary `1.1.0` | current Platform | accepted inactive; null verified boundary remains non-activatable |
| Canary `1.2.0` | current Platform | accepted under exact runtime-threshold contract |
| proposed Canary `1.3.0` | current Platform | rejected fail closed as unsupported |
| proposed Canary `1.3.0` | future Platform `1.3.0` inactive consumer | accepted only after exact schema/hash, semantic validation and typed transactional import |
| future producer version | Platform without that version | rejected fail closed |

This architecture PR does not change the current matrix. It does not add `1.3.0` to `config/game-catalog.php`.

## Immutable retained versions

The complete `1.3.0` consumer must retain support for the exact existing contracts:

| Version | SHA-256 |
|---|---|
| `1.0.0` | `099a8373ff2b0017cc2b321991662dc4e4783b626391aa7a110a6db0559d146b` |
| `1.1.0` | `323ff6ae849759c9190f2a0c342855194ed74645816adc45051b6d914e67c7ac` |
| `1.2.0` | `a9fa1e3c6366a90d61005796511c344ced9c39594ed676276279a5917287c6de` |

No migration may rewrite existing immutable snapshot source documents or reinterpret their fields.

## Architecture artifact status

The extension schema proposal hash is:

```text
7e9699ecb04bbc777679e22cee9352ae49ff21220eff7294c042358f01d0571e
```

The fixture proposal hash is:

```text
b603b4ef1ccbe763d6f5f7565f40d6604027e73101006d359ccfc4aae06f10ca
```

These are architecture artifacts, not the final canonical schema/fixture hashes. The consumer implementation must produce the complete strict `1.3.0` schema by retaining all `1.2.0` definitions and adding the approved NPC/shop definitions. Its exact bytes and hash become authoritative only after validation and review.

## Required rollout order

```text
architecture proposal
  -> Platform complete inactive consumer
    -> Canary NPC runtime-authority proof
      -> Canary complete producer
        -> shared exact schema/fixture parity
          -> immutable candidate artifact
            -> staging import and review
              -> candidate activation
                -> rollback proof
                  -> separate public projection
                    -> separate exact-snapshot production gate
```

The Platform consumer implementation and Canary producer are an `atomic-required` cross-repository change. The consumer merges first.

## Platform consumer gate

Before Canary producer support may merge, Platform must prove:

- complete canonical `1.3.0` schema is valid Draft 2020-12;
- exact schema and fixture hashes are pinned;
- unsupported versions still fail closed;
- explicit entity dispatch handles only `item`, `creature` and `npc`;
- explicit relation dispatch handles only `creature_loot`, `npc_buy_offer` and `npc_sell_offer`;
- unknown entity/relation type is rejected rather than mapped to an existing table;
- source, target and currency endpoints resolve and have the correct type;
- canonical relation identity matches direction, endpoints and runtime path;
- duplicate canonical keys fail;
- counts are verified by typed family and total;
- import is transactional and inactive;
- failure preserves the prior active snapshot and projections;
- admin preview exposes typed counts and findings;
- no public route or automatic activation is added.

The consumer may import `1.3.0` for inactive review even when `verified_content_through_release` is null. It must not activate such a candidate.

## Canary authority gate

Before producer implementation, Canary must prove:

- deterministic read-only iteration of the final `Npcs` registry;
- collection runs within the existing safe dispatcher/Lua export boundary;
- registry key, runtime name, type name and display policy remain distinct;
- static `NpcType::info.shopItemVector` scope is explicit;
- player-specific `shopPlayers` overrides are excluded and reported as a limitation;
- nested `childShop` paths are stable and preserved;
- default storage key/value sentinel semantics are exact;
- nonzero buy/sell directions map to separate relations;
- currency is the exact `NpcType::info.currencyId` endpoint;
- canonical collisions and dangling item/currency endpoints fail closed.

## Canary producer gate

The producer PR must include:

- byte-identical complete schema and fixture from the compatible Platform consumer;
- identical SHA-256 values;
- exact-value unit tests for NPC and offers;
- duplicate, collision, nested-path and dangling endpoint tests;
- fixed-timestamp byte determinism;
- lowercase sidecar determinism;
- malformed metadata failure;
- previous-valid-output preservation;
- repeated full-datapack export;
- no database or network endpoint syscalls;
- telemetry-off/on loader stability when applicable.

The default producer profile must not switch to `1.3.0` unless deployment routing guarantees a compatible consumer.

## Staging gate

Staging is a separate Platform task. It requires one exact candidate record:

- Canary commit;
- Platform commit;
- complete schema hash;
- fixture hash used by contract tests;
- artifact digest and sidecar verification;
- datapack revision or explicit null/unknown state;
- profile key and protocol profile;
- generated timestamp;
- entity/relation family counts;
- validation summary.

Required lifecycle:

1. import inactive;
2. review semantic findings and diff;
3. verify current active snapshot remains unchanged;
4. activate candidate in staging only when activation boundaries are satisfied;
5. smoke administrator and public projections applicable to the task;
6. reactivate the previous snapshot through deterministic rollback;
7. record exact results.

A historical PR-#272/schema-`1.0.0` staging workflow is evidence that one bounded lifecycle worked; it is not a reusable current transport for `1.3.0`.

## Public gate

Schema support and inactive persistence do not authorize public NPC/shop pages.

Public projection requires a separate PR defining:

- active-profile visibility rules;
- allowlisted NPC and offer fields;
- display-name/localization policy;
- search, pagination and deterministic sorting;
- handling of registered-only, conditional, unknown and unverified data;
- exclusion of admin/test/private condition details where appropriate;
- authorization and audit for administrator-only diagnostics;
- desktop, tablet and mobile acceptance.

An offer must not become public merely because it exists in the runtime registry.

## Rollback compatibility

Existing `1.0.0`–`1.2.0` snapshots remain immutable rollback targets.

Activation or rollback to any version must use the same compatibility and completeness checks. A failed `1.3.0` candidate activation must leave:

- the previous active snapshot active;
- existing public projections unchanged;
- the candidate inactive or failed;
- an exact failure/audit record;
- no partial NPC/shop visibility.

No migration may require destructive conversion of retained old snapshots to make rollback possible.

## Production gate

Production activation is not part of schema `1.3.0` architecture, consumer or producer support.

A later production task requires explicit user approval naming one exact candidate and all of:

- artifact digest;
- Canary commit;
- Platform commit;
- complete schema hash;
- datapack/profile revision;
- import PASS;
- semantic validation PASS;
- staging activation PASS;
- staging public projection PASS when applicable;
- rollback PASS;
- previous active snapshot backup/target;
- monitoring and operator approval;
- no unresolved conflict.

Without those facts, production remains blocked.

## Fail-closed cases

The complete consumer and producer must reject:

- unsupported schema version;
- schema hash mismatch;
- artifact hash mismatch;
- count mismatch;
- duplicate entity or relation canonical key;
- unknown entity/relation type;
- dangling NPC, offered item or currency endpoint;
- wrong endpoint type;
- relation direction inconsistent with canonical key;
- canonical path inconsistent with runtime path;
- zero price emitted as an active direction;
- malformed storage condition;
- nondeterministic ordering;
- partial persistence or publication.

Unknown historical or availability evidence is not a validation error when represented as the allowed null/unknown state. Converting it to a stronger value is an error.

## Task decomposition

| Task | Repository | Result |
|---|---|---|
| `OTERYN-20260730-game-catalog-schema-1-3-consumer` | Platform | complete inactive consumer and final canonical schema/hash |
| `CAN-20260730-game-catalog-npc-runtime-authority` | Canary | safe final-registry authority and identity proof |
| `CAN-20260730-game-catalog-schema-1-3-producer` | Canary | complete producer and exact parity |
| `OTERYN-20260730-game-catalog-npc-shop-staging` | Platform | exact staging import/activation/rollback evidence |
| `OTERYN-20260730-game-catalog-npc-shop-public` | Platform | separately gated public projection |

Quest and spawn work must not be folded into any of these tasks.
