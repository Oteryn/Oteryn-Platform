# Oteryn-v2 Game Restore Reconciliation Contract

## Status

`ACCEPTED PLATFORM PRODUCER-SIDE CONTRACT — oteryn-v2-game-restore-reconciliation-v1 — DEFAULT-OFF`

This contract is Platform's side of Game OPERABILITY §3 rulings 9–11 and amendment A3, and of
Game contract `oteryn-game-character-authority-commands-v1` §5.4 (restore reconciliation, U-CC7).
It defines the restore notice wire, what Platform does with it, and when Platform acknowledges.
Task: PLATFORM-RESTORE-RECONCILE-P1. Governing issue: #1474.

## Principles

- Platform never calls Game. Game delivers the notice over the internal mTLS listener and repeats
  the identical bytes until Platform answers `ACKNOWLEDGED`.
- There is no cross-database transaction. Platform records the notice and reopens the affected
  operations in one Platform transaction; Game answers each reopened operation from its own
  restored store through the unchanged command routes (Game contract §4).
- Selection is by Game-issued `recovery_generation` only. Platform never compares a Platform time,
  `decided_at`, `issued_at_source` or the restore point T to decide what to reopen.
- History is kept. A superseded receipt is never deleted or rewritten except for `superseded_at`.
- One serialization point. Receipt publication and restore-notice recording share one Platform
  lock (see Serialization), so no receipt commits between a notice's selection and its commit.

## Wire

`POST /internal/v1/game-auth/game-restore/notice`, route name
`internal.character-authority.restore.notice`, `Content-Type: application/json`, at most 512 bytes,
canonical JSON with exactly these members:

| Member | Type | Rule |
| --- | --- | --- |
| `contract_version` | integer | `1` |
| `operation` | string | `PublishGameRestoreNoticeV1` |
| `source_authority` | string | must equal the configured `restore.source_authority` |
| `restore_event_id` | string | UUIDv7, one per Game restore |
| `restored_generation` | decimal string | H, the generation the restore rolled back to; > 0 |
| `new_recovery_generation` | decimal string | the generation Game decides under from now on; > H |
| `restore_point` | decimal string | T, Unix seconds; recorded only, never used for selection |
| `backup_label` | string | `[A-Za-z0-9._:-]{1,128}` |

The 200 answer is canonical JSON:

```json
{"contract_version":1,"restore_event_id":"<id>","status":"RECONCILING|ACKNOWLEDGED","reopened":N,"outstanding":M}
```

`status` is `ACKNOWLEDGED` exactly when `outstanding` is 0. Failures have an empty body:

| Status | Meaning |
| --- | --- |
| 400 | malformed notice, unknown or missing member, or `new_recovery_generation` ≤ `restored_generation` |
| 401 | no TLS 1.3 client certificate, an identity not in `restore.identities`, or another source authority |
| 409 | the same `restore_event_id` with different bytes, or a new event whose `new_recovery_generation` does not exceed every stored notice's (stale); both alarm |
| 413 | `Content-Length` above 512 |
| 429 | per-identity rate limit, with `Retry-After` |
| 503 | the surface is disabled or misconfigured, or the store is unavailable; Game repeats later |

Every response is `no-store`.

## Platform behaviour

On the first delivery of a notice with restored generation H, one transaction:

1. takes the restore lock exclusively (see Serialization) and, if an identical delivery of the
   same notice committed while it waited, answers that notice's status;
2. refuses it as stale unless its `new_recovery_generation` is above every stored notice's;
3. stores the notice bytes (`character_restore_notices`);
4. reopens every TERMINAL intent whose current receipt has `recovery_generation` ≥ H: the receipt
   gets `superseded_at`, the intent becomes PENDING with no current receipt, and a
   `character_restore_reopens` row links notice, intent and superseded receipt;
5. also waits for every intent an earlier notice reopened that still has no new receipt and whose
   superseded receipt has generation ≥ H (a second restore before the first was answered).

Game then lists and reads the reopened intents again under their unchanged `operation_id` and
intent bytes, and publishes exactly one new receipt per operation. The receipt route stores it
as the new current receipt and marks every open reopen row of that intent answered. The new
receipt may equal the superseded bytes (Game still held a decision made before the snapshot within
generation H); after it is stored, any receipt with other bytes gets 409 as usual.

A receipt whose `recovery_generation` lies strictly between a stored notice's
`restored_generation` and `new_recovery_generation` was decided after that restore's snapshot and
before that restore, so Game no longer holds it: the receipt route refuses it with 409 and the
alarm `character_authority.command_receipt_lost_to_restore`, and it answers no reopen row. A
receipt within the restored generation itself, or at or above the new generation, is accepted as
above. Game publishes the restored decision again at its current generation.

A repeated identical notice reopens nothing more; it answers the current status. Once every
reopen row of the notice is answered, Platform sets `acknowledged_at` once and answers
`ACKNOWLEDGED`. A notice that reopens nothing is acknowledged on its first delivery.

### Serialization

Receipt publication and restore-notice recording share one serialization point, the single row of
`character_restore_locks`. A receipt publication takes it shared, before it locks the intent, and
holds it until it commits; notice recording takes it exclusively before it checks for staleness
or selects anything. So a receipt either commits before the notice selects, and is reopened if
its generation is ≥ H, or waits until the notice has committed, and then answers the notice's
reopen row. No PENDING→TERMINAL transition crosses a notice's selection. Receipts do not wait for
each other, and notices are serialized with each other.

### Compensation

Platform settles or compensates only on the new receipt and never infers a Game outcome from its
own saga row. Consumers read live intent status, so a reopened operation reads as PENDING and then
as the new outcome. A superseded COMMITTED outcome answered by a REJECTED receipt raises the alarm
`character_authority.restore_outcome_reversed` (operation id, command, result code). Automated
compensation of such a reversal is deferred: no settling workflow is live while the command surface
is disabled, so operators handle each alarm until a consumer that settles value is enabled.

## Configuration

`config/character-authority.php` section `restore`:

| Key | Env | Default |
| --- | --- | --- |
| `enabled` | `CHARACTER_AUTHORITY_RESTORE_NOTICE_ENABLED` | `false` |
| `identities` | `CHARACTER_AUTHORITY_RESTORE_NOTICE_IDENTITIES` | none; a JSON list of client certificate subjects |
| `source_authority` | `CHARACTER_AUTHORITY_RESTORE_NOTICE_SOURCE_AUTHORITY` | none |
| `requests_per_minute` | `CHARACTER_AUTHORITY_RESTORE_NOTICE_REQUESTS_PER_MINUTE` | `60` (1–600) |

Unlike the command routes, the restore surface is not limited to an environment: Game opens
admission after a restore only on Platform's acknowledgement, so a stack whose Game can be
restored must be able to answer. The identity list is dedicated: sharing a subject with the
command consumer, the public-projection consumer or any other internal purpose disables every
surface involved. A reopened intent can only be answered while the command surface is enabled;
with commands never enabled there are no intents and every notice is acknowledged at once. Exposing the path on the deployed mTLS listener is a
deployment change made with enablement, not part of this contract version.

## Acknowledgement time (RTO acceptance item)

The acknowledgement is on Game's RTO path. Platform states:

- Platform adds no wait of its own. Reopening is one transaction bounded by the number of intents
  with generation ≥ H. Platform answers `ACKNOWLEDGED` on the first delivery after Game's last
  reopened receipt is stored, so with Platform up the acknowledgement time is Game's own
  republication time plus one delivery interval.
- With Platform unavailable, the acknowledgement waits for Platform's recovery. Platform's
  provisional RTO is 4 h (NAS-only staging, per the PostgreSQL plan). Game's restore RTO
  therefore includes up to Platform's RTO when both are down.

Owner ratification: the owner ratified this bound (no Platform wait; with Platform down, the
provisional 4 h Platform RTO) on 2026-10-15, to be reviewed after the first restore drill
(Oteryn/Oteryn-Game#1622, comment 6187301378, item 11).

## Platform restore (the other direction)

A Platform restore to an earlier point has no notice in this contract. These cases are covered:

- a lost receipt leaves the intent PENDING; Game lists it again and republishes its stored receipt,
  which Platform accepts as a first receipt;
- a lost intent makes Game's receipt answer 404 and alarm
  `character_authority.command_receipt_without_intent`; Game keeps its own decision;
- a reused `source_revision` or `operation_id` for a different intent is refused by Game with
  `CHAR_CMD_OPERATION_CONFLICT`, never committed as a different command.

Open item (follow-up PLATFORM-PITR-FENCE-1): a restore can lose both an intent and its
idempotency row while Game keeps the decision. Later requests can then move the restored
sequence past Game's retained `source_revision` range, and a retry of the lost request gets a
fresh revision and a fresh `operation_id`. Neither conflict above applies, so Game could execute
the mutation twice. Closing this needs a restore epoch or fence, or reconciliation of lost
operation identities, before Platform accepts new commands after the restore. That design
belongs to the follow-up. Until it lands, a Platform point-in-time restore with character
commands enabled is not covered by this contract.

## Deliveries (A4)

Amendment A4 (Game lists deliveries by generation, keyed by `delivery_operation_id`) is accepted
as the shape for the PROD-ENTITLEMENTS delivery reconciliation in
`OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md`. Platform has no delivery implementation yet, so
this contract version reconciles character commands only; the delivery side follows the same rules
(selection by generation, superseded history kept, one new outcome, acknowledgement after all are
answered) when deliveries are implemented.

## Related

- `docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md`
- `docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md`
- Oteryn/Oteryn-Game `docs/contracts/OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md` §5.4
