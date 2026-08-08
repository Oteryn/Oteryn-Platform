# Character Deletion, Restore and Finalization Contract

## Status

`LEGACY CANARY COMPATIBILITY DISCOVERY — NOT NATIVE OTERYN-V2 LIFECYCLE AUTHORITY`

This contract preserves evidence about the current Canary deletion mechanism and the minimum operation boundary that would be required **only if Oteryn deliberately retains a separately authorized Legacy Canary Compatibility implementation** for deletion scheduling, grace-period cancellation/restoration or finalization.

It is not the target native character-lifecycle contract. Native lifecycle authority is governed by ADR 0030, ADR 0031 and `docs/architecture/character-lifecycle/NATIVE_CHARACTER_LIFECYCLE_AUTHORITY.md`: Oteryn Platform owns authenticated UX/orchestration/business state while Oteryn-v2 Character Authority owns canonical `CharacterId`, current ownership, lifecycle eligibility, mutation execution and authoritative results/receipts.

The current Canary decision remains **NO-GO** for a direct Platform runtime endpoint, database principal or Canary mutation. The existing Canary mechanism is a legacy timestamp-plus-startup-cleanup lifecycle, not an operation-safe service.

## Evidence pin

Read-only evidence was inspected at:

- `blakinio/canary@3c90d3ada717cd2ed0c5344f1dac210a205355f6`;
- `schema.sql@4e3d7e4b87ac51d1f664328b5399a78226153dc8`;
- `src/account/account_repository_db.cpp@4b4089848c35a7dde7bd04b474d643c47ca7034e`;
- `src/io/functions/iologindata_load_player.cpp@19169deecdf67b47691e9e4521f3519c1bfbdc40`;
- `src/io/iologindata.cpp@6fb9bff080a4d8f9038f74006c21729466279c0b`;
- `src/server/network/protocol/protocollogin.cpp@8efc7b986dbaf127860a6183a756f65d1f742c32`;
- `src/server/network/protocol/protocolgame.cpp@bfa9457e27a65eaf1f45407b80bb3fdf6a9c0a24`;
- `data/scripts/globalevents/server_initialization.lua@0542f922a824e76cec4ac1214d51d13e543fcaf3`;
- `tests/integration/account/account_repository_db_it.cpp@78b68c24fcee66b6c8db8cab60968c56fb2f2e52`.

A materially different deployed revision or schema requires complete revalidation. Repository evidence is not production verification.

## Proven current Canary behavior

### State representation

`players.deletion` is a required `bigint` with default `0`.

- `0` means active/listable/loadable.
- any non-zero value makes the character unavailable immediately;
- current code treats the non-zero value as a Unix timestamp for later physical deletion.

The account repository omits every non-zero row from the account character map and therefore from login character lists and issued character-bound session tokens. `preLoadPlayer` independently rejects every non-zero value before loading the player into the game world.

The earlier game-world ownership gate does not itself include `deletion`, but this is not a proven login bypass because `preLoadPlayer` rejects the character before placement.

### Finalization timing

Current finalization is not a continuously scheduled or request-addressable operation. During server startup, a Lua global event submits:

```sql
DELETE FROM players
WHERE deletion != 0
  AND deletion < current_time
```

The query is submitted asynchronously. Therefore:

- the effective deletion deadline does not cause immediate finalization;
- an expired row may remain until a later server startup;
- finalization is coupled to infrastructure lifecycle rather than a durable operation record;
- the Platform cannot correlate a request, retry or timeout with one exact Canary finalization result;
- a cancellation/finalization race cannot be reconciled from a Platform operation identifier because Canary stores none.

### Physical-delete side effects

A physical `players` delete is broad and destructive.

Foreign-key cascades remove player-keyed rows including account VIP references, daily rewards, forge history, guild invitations/membership, market offers/history, online/session rows, deaths, inventory, depot/inbox/reward items, stash, storage, spells, prey/taskhunt/bosstiary, namelocks and other gameplay/audit rows.

Additional material effects include:

- deleting a guild owner row cascades deletion of the `guilds` row, which in turn cascades guild ranks, memberships and invitations;
- the `ondelete_players` trigger sets every owned house to owner `0`;
- historical name-bearing rows such as guild-war kills and death participant text are not normalized into an immutable deletion history by this lifecycle;
- physical deletion removes the source row required for later ownership, state and idempotency checks.

These effects are Canary-owned compatibility semantics. They must not be copied into native Oteryn-v2 lifecycle rules by analogy.

## Native target authority

For the native target:

### Platform-owned

Platform may own the web-facing workflow and durable orchestration metadata, including:

- immutable Platform operation/public ID;
- authenticated Identity and canonical `AccountId` reference;
- canonical game-owned `CharacterId` reference;
- requested business intent and Platform policy revision;
- idempotency/correlation identity;
- bounded saga/recovery/error classification;
- user notification state;
- privacy-safe audit metadata.

A Platform operation row is never ownership proof or proof of game-domain mutation success.

### Oteryn-v2 Character Authority-owned

Oteryn-v2 remains authoritative for:

- current `AccountId <-> CharacterId` ownership;
- character lifecycle state and eligibility;
- schedule-delete, cancel/restore and finalization mutations;
- gameplay-owned conflict/side-effect policy;
- authoritative mutation result/receipt and reconciliation state.

Native work uses versioned game-owned commands/results or equivalent explicit service contracts. Exact wire shape, transport, storage and game-internal locking remain external Oteryn-v2 authority and are not invented here.

## Legacy Canary Compatibility-only operation semantics

The remainder of this section applies **only** if a future owner decision explicitly retains a Canary compatibility implementation. It is not a prerequisite for native Issue #317.

No compatibility implementation is approved until an operation-specific Canary contract and executable implementation provide all required semantics below.

### Schedule

A Canary compatibility schedule request would need to atomically:

1. resolve the authenticated Identity's ready immutable Canary account binding server-side;
2. lock the exact account and player rows in deterministic order;
3. require current `players.account_id` to match the resolved account;
4. require current deletion state to be active;
5. reject any `cluster_sessions` row for the player, regardless of status or expiry;
6. reject a Character Bazaar listing, escrow, settlement, cancellation or recovery state involving the player;
7. reject concurrent rename, transfer or another deletion operation;
8. apply explicit guild-owner, house-owner, market-offer and other destructive-side-effect policy;
9. record one immutable operation identifier before making the character unavailable;
10. return a bounded idempotent result.

The browser may provide a public character/operation reference and confirmation text, but never an account ID, authoritative player ID, deadline or lifecycle state.

### Grace period and visibility

Scheduling makes the character unavailable to Canary login immediately under current Canary semantics. The product must state this consequence explicitly; “grace period” means a restoration window, not continued gameplay.

Compatibility account states would need to include:

- `active`;
- `pending_deletion` with an effective date;
- `restored`;
- `finalization_pending`;
- `finalized`;
- `recovery_required`.

Compatibility public behavior would need to distinguish these states explicitly and must not fabricate live data after deletion/finalization.

### Cancel or restore

A Canary compatibility cancellation would need to atomically:

1. lock the same operation and Canary player row;
2. re-resolve current ownership and session/Bazaar conflicts;
3. require the authoritative deadline not to have passed;
4. require the character row still to exist and remain tied to the expected account;
5. restore active Canary state exactly once;
6. persist the Platform terminal result only after authoritative reread confirms active state.

A timeout or ambiguous database outcome becomes `recovery_required`; it never becomes fabricated success.

### Finalize

A Canary compatibility finalization would need to be an explicit, idempotent Canary-owned operation, not an uncorrelated raw startup `DELETE`.

It would need to:

- claim one exact operation ID;
- lock and revalidate the player and conflict set;
- require the deadline to have passed;
- apply reviewed guild, house, market, mail, session and historical-retention policy;
- return a durable bounded result that reconciliation can reread;
- be retry-safe after failure before or after commit;
- leave sufficient tombstone/audit evidence to distinguish finalized, missing-before-finalization and ambiguous outcomes without retaining credentials or gameplay blobs.

## Compatibility conflict policy

Any future Canary compatibility schedule, restore or finalize attempt must fail closed when ownership, session, Bazaar, concurrent lifecycle operation, destructive gameplay obligation, schema/revision, grant or dependency state is unavailable or contradictory.

`players_online` alone is not authoritative for cluster-wide online state.

## Compatibility concurrency and idempotency requirements

A retained Canary compatibility implementation would require:

- a unique client idempotency key scoped to Identity and intent;
- one immutable operation/public ID;
- optimistic Platform state versioning plus deterministic row locks;
- deterministic Canary account/player lock order;
- compare-and-set predicates containing expected ownership and deletion state;
- bounded retries only for recognized deadlock/serialization failures;
- reconciliation based on authoritative current rows and operation receipts, never timeout inference;
- mutual exclusion with Character Bazaar, rename and transfer operations.

Cross-database Platform/Canary work is a durable compatibility saga, not distributed ACID.

## Compatibility least-privilege decision

No deletion credential or grant template is approved or to be provisioned under this contract.

The existing principals are explicitly insufficient:

- `canary` is read-only;
- `canary_character_create` authorizes bounded inserts, not updates/deletes;
- `canary_character_transfer` authorizes only `UPDATE(players.account_id)`;
- Character Bazaar escrow authority does not authorize deletion, restore or finalization.

If direct column privileges are ever proposed for a retained compatibility path, that compatibility contract must prove they cannot bypass side-effect policy, startup cleanup, idempotency or recovery. Current evidence does not prove this.

## Optional Legacy Canary Compatibility prerequisite

Issue #344 is no longer a prerequisite for the **native** deletion/restore target.

If the repository owner later explicitly decides to retain deletion/restore on the Canary compatibility stack, a separately authorized Canary task would still need to replace/fence the raw startup deletion path and expose a bounded operation-safe interface. Such work would be compatibility-only, would require its own external-repository authorization, and would need rollout/rollback/removal criteria.

No Canary write is authorized by this Oteryn Platform task.

## Validation required for any future compatibility implementation

A retained Canary compatibility implementation would require exact-revision Canary behavior/concurrency/side-effect evidence, effective-grant tests, Platform authorization/security tests, mutual-exclusion tests, ambiguous-outcome recovery tests, user-facing acceptance and environment-gated rollout/rollback verification.

Those requirements do not define or block the native Oteryn-v2 implementation path.

## Decision

`NATIVE CHARACTER DELETION/RESTORE: ROUTE TO OTERYN-V2 CHARACTER AUTHORITY`

`LEGACY CANARY CHARACTER DELETION/RESTORE: NO-GO FOR DIRECT PLATFORM IMPLEMENTATION`

Issue #317 remains the product gap, but its native target is no longer blocked by Issue #344 or by a new Canary lifecycle producer. Native implementation must follow ADR 0030/0031 and the focused lifecycle routing guide using canonical `AccountId`/`CharacterId` and versioned game-owned command/result semantics.

Issue #344 and this document remain historical/current Canary compatibility evidence only. They may be used again only if a separate owner decision explicitly retains the compatibility feature.

No runtime, database migration, credential, production, Canary/Oteryn-v2 repository write or `PRODUCTION_PROVEN` claim follows from this reconciliation.
