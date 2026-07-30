# Character Deletion, Restore and Finalization Contract

## Status

`DISCOVERY COMPLETE — DIRECT PLATFORM IMPLEMENTATION NOT APPROVED`

This contract records the current Canary behavior and the minimum operation boundary required before Oteryn Platform may implement character deletion scheduling, grace-period cancellation or restoration, and finalization.

The current decision is **NO-GO** for a Platform runtime endpoint, database principal or Canary mutation. The existing Canary mechanism is a legacy timestamp-plus-startup-cleanup lifecycle, not an operation-safe service that can satisfy Issue #317 recovery, concurrency, audit and side-effect requirements.

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

These effects are Canary-owned gameplay semantics. A Platform operation must not accept them accidentally as a consequence of a generic `DELETE` privilege.

## Ownership decision

### Platform-owned

A future implementation may own only the web-facing workflow and durable orchestration metadata:

- immutable public operation ID;
- authenticated Identity reference;
- server-resolved Canary account/player references;
- requested and effective timestamps;
- lifecycle state and monotonic version;
- idempotency key;
- bounded recovery/error classification;
- user notification state;
- privacy-safe audit metadata.

A Platform operation row is never character ownership proof.

### Canary-owned

Canary remains the semantic owner of:

- `players.deletion` meaning;
- login/list/load exclusion;
- online/session exclusion;
- physical finalization;
- guild, house, market, inventory, session and other gameplay side effects;
- retention or tombstone policy after finalization.

Because current finalization is an uncorrelated startup cleanup, Canary does not yet expose the operation boundary required by the Platform workflow.

## Required future operation semantics

No implementation is approved until an operation-specific Canary contract and executable implementation provide all semantics below.

### Schedule

A schedule request must atomically:

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

Scheduling makes the character unavailable to native login immediately under current Canary semantics. The product must state this consequence explicitly; “grace period” means a restoration window, not continued gameplay.

Required account states:

- `active`;
- `pending_deletion` with an effective date;
- `restored`;
- `finalization_pending`;
- `finalized`;
- `recovery_required`.

Required public behavior:

- `active`: normal privacy-aware profile/search behavior;
- `pending_deletion`: not public and not present in public search, sitemap, highscores, guild directory projections or new Bazaar listings;
- `restored`: normal visibility only after authoritative Canary state is active and Platform preferences are re-resolved;
- `finalization_pending`: not public;
- `finalized`: no live profile; historical public records follow a separate explicit retention policy rather than fabricated live data;
- `recovery_required`: fail closed and not public.

### Cancel or restore

Cancellation must atomically:

1. lock the same operation and Canary player row;
2. re-resolve current ownership and session/Bazaar conflicts;
3. require the authoritative deadline not to have passed;
4. require the character row still to exist and remain tied to the expected account;
5. restore active Canary state exactly once;
6. persist the Platform terminal result only after authoritative reread confirms active state.

A timeout or ambiguous database outcome becomes `recovery_required`; it never becomes fabricated success.

### Finalize

Finalization must be an explicit, idempotent Canary-owned operation, not an uncorrelated raw startup `DELETE`.

It must:

- claim one exact operation ID;
- lock and revalidate the player and conflict set;
- require the deadline to have passed;
- apply reviewed guild, house, market, mail, session and historical-retention policy;
- return a durable bounded result that reconciliation can reread;
- be retry-safe after failure before or after commit;
- leave sufficient tombstone/audit evidence to distinguish finalized, missing-before-finalization and ambiguous outcomes without retaining credentials or gameplay blobs.

## Conflict policy

Every schedule, restore and finalize attempt fails closed when any of these are true:

- ready immutable Identity-to-Canary binding is missing or not ready;
- player ownership changed or cannot be proven;
- any `cluster_sessions` row exists;
- Character Bazaar state is listing-pending, escrow-pending, active, cancellation-pending, settlement-pending or recovery-required;
- a rename or world/channel transfer operation owns the player;
- the player owns a guild and no approved leadership-transfer/disband policy has completed;
- the player owns a house and no approved release/transfer policy has completed;
- active market offers, house bids, mail delivery or other reviewed gameplay obligations require settlement;
- Canary revision/schema/effective grants differ from the approved evidence profile;
- dependency state is unavailable or contradictory.

`players_online` alone is not authoritative for cluster-wide online state.

## Concurrency and idempotency requirements

A future implementation must use:

- a unique client idempotency key scoped to Identity and intent;
- one immutable operation/public ID;
- optimistic Platform state versioning plus deterministic row locks;
- deterministic Canary account/player lock order;
- compare-and-set predicates containing expected ownership and deletion state;
- bounded retries only for recognized deadlock/serialization failures;
- reconciliation based on authoritative current rows and operation receipts, never timeout inference;
- mutual exclusion with Character Bazaar, rename and transfer operations.

Cross-database Platform/Canary work is a durable saga, not distributed ACID.

## Least-privilege decision

No deletion credential or grant template is approved or to be provisioned under this contract.

The existing principals are explicitly insufficient:

- `canary` is read-only;
- `canary_character_create` authorizes bounded inserts, not updates/deletes;
- `canary_character_transfer` authorizes only `UPDATE(players.account_id)`;
- Character Bazaar escrow authority does not authorize deletion, restore or finalization.

A future approved design should prefer a Canary-owned command/service or narrowly defined stored operation returning an operation receipt. If direct column privileges are still proposed, the contract must prove that they cannot bypass side-effect policy, startup cleanup, idempotency or recovery; current evidence does not prove this.

## Required Canary prerequisite

Before Platform implementation, a separately authorized Canary task must:

1. replace or fence the raw startup deletion path so it cannot race or bypass the contracted lifecycle;
2. define an explicit schedule/cancel/finalize interface with operation IDs and bounded outcomes;
3. define guild-leader, house-owner, market, session and history retention behavior;
4. make finalization deterministic and independently invokable rather than dependent on restart timing;
5. add exact schema/runtime/integration tests for authorization boundary, locking, retries, side effects and recovery;
6. document producer/consumer rollout and rollback compatibility;
7. expose only the minimum effective grants or authenticated internal interface required by Platform.

No Canary write is authorized by this Oteryn Platform task.

## Validation required before approval

A later implementation contract must require, on exact final revisions:

- Canary unit/integration tests for each lifecycle result and destructive side effect;
- real MariaDB concurrency tests for schedule/cancel/finalize races and finalizer recovery;
- effective-grant tests proving required and forbidden privileges;
- Platform authorization, object-injection, CSRF, rate-limit and stale-version tests;
- Character Bazaar/rename/transfer mutual-exclusion tests;
- failure injection before and after Canary commit with deterministic reconciliation;
- EN/PL desktop/tablet/mobile zero-retry browser acceptance;
- environment-gated deployment verification and rollback rehearsal.

## Decision

`CURRENT CHARACTER DELETION/RESTORE: NO-GO FOR DIRECT PLATFORM IMPLEMENTATION`

Issue #317 remains open. The next safe step is a separately authorized Canary lifecycle prerequisite. After that prerequisite is merged and pinned, Oteryn Platform must revalidate this contract and create a new implementation task. No runtime, credential, production or `PRODUCTION_PROVEN` claim follows from this discovery.
