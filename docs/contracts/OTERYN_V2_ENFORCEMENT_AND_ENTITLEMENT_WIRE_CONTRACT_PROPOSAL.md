# Oteryn-v2 enforcement command and Profile-B entitlement wire (proposal)

## Status

`PROPOSED — oteryn-v2-game-enforcement-wire-v1 AND oteryn-v2-profile-b-authority-wire-v1 — AUTHORIZES NO IMPLEMENTATION — OWNER ACCEPTANCE PENDING — DEFAULT-OFF`

- `IMPLEMENTATION_AUTHORITY: NONE` (Platform and Game). `PRODUCTION_AUTHORITY: NONE`.
- Requested by the programme coordinator on 2026-11-13. The owner must accept this text before Game or Platform writes code against it.
- It proposes the transport and IDL that two accepted architecture contracts defer:
  - Part A: `OTERYN_V2_GAME_ENFORCEMENT_COMMAND_CONTRACT.md`, *Explicitly deferred* ("exact transport, endpoint, schema/IDL bytes").
  - Part B: `OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md`, *Profile B finite authority lease* and *Deferred details* ("event/query/command transport", "numeric per-product Profile-B lease/refresh/skew values").
- It is subordinate to both. Where this draft and an accepted contract disagree, the accepted contract wins and this draft is wrong.
- Every number, name and enum value below is a **proposal** with its reason. Nothing here activates a profile, a product, a flag, a certificate or a route.

This proposal authorizes no payment, entitlement issuance, sanction, Game runtime or schema change, certificate or secret operation, deployment or production mutation.

## 1. Shared conventions

Both parts reuse the conventions that the Platform's current `/internal/v1` routes already follow (`OTERYN_V2_SANCTION_EXPORT_CONTRACT.md` (N009), `OTERYN_V2_LOYALTY_SNAPSHOT_CONTRACT.md` stream, `OTERYN_V2_ACCOUNT_SECURITY_REVOCATION_STREAM_CONTRACT_PROPOSAL.md`, `OTERYN_V2_PREMIUM_TIME_SNAPSHOT_CONTRACT.md`, `OTERYN_V2_COIN_TRANSFER_AND_MARKET_CONTRACT.md`). This proposal invents no new mechanism.

### 1.1 Direction

Platform never calls Game (plan §5.3.2). Every route is Game→Platform:

- Game pulls commands and authority over `/internal/v1`.
- Game posts its authoritative results back over `/internal/v1`.

There is no inbound endpoint on Game, no shared database and no SQL projection writer.

### 1.2 Transport and peer identity

- `POST` only, HTTP/1.1, on the existing private TLS 1.3 listener (`OTERYN_INTERNAL_TLS_*`). The public listener supplies no verified identity, so it cannot reach these routes.
- The peer identity is the verified client certificate's single DNS SAN, exact lowercase, compared in constant time. The CN, `SSL_CLIENT_S_DN` and every request header are ignored as identity.
- Each route group (§6.6, §8.5) has **one dedicated identity** in its own `*_IDENTITY` variable. That variable takes part in the shared overlap check: every `*_IDENTITY`, `*_SAN` and `*_IDENTITIES` key in the process forms one `IdentityMap`. An identity that equals any other internal identity disables both groups that share it (`503`).

### 1.3 Guard order

For every route, in this order:

1. `503`, empty, when the group's flag is not literally `true`, or its configuration is missing, malformed or overlapping. Checked before the body is read.
2. `401`, empty, when there is no verified TLS 1.3 peer, or its SAN is not the group's identity.
3. `429`, empty, with `Retry-After: 60`, when the group's per-process peer budget is exhausted. Authenticated requests count, including malformed bodies.
4. Transport checks, then the body:
   - exactly one `Content-Type: application/json`;
   - no `Transfer-Encoding`;
   - `Content-Encoding` absent or `identity`;
   - no query string.
5. The route handler.

### 1.4 Bodies

- **Requests.** A request is one JSON object within the route's byte bound, JSON depth at most 4. Member order and insignificant whitespace are accepted. Any unknown, duplicate, missing or wrongly typed member answers `400`. So do a fraction or exponent, an oversized or malformed body, or an invalid cursor. This matches N009.
  - Exception: the result body (§4) is **canonical**. It must be compact, in the listed member order, with no escapes, because Platform stores and compares its exact bytes.
- **200 bodies.** Every 200 body is canonical compact UTF-8 JSON:
  - members in exactly the listed order;
  - no whitespace, BOM, trailing newline or backslash escape;
  - integers as JSON integers, never floats.
- **Failures.** Every non-200 body is empty.
- **Caching.** Every response, of any status, carries `Cache-Control: no-store, private`.
- **Timeout.** A handler that exceeds a 3-second read timeout answers `503` (N009).

### 1.5 Types

| Type | Form |
|---|---|
| `uuid7` | canonical lowercase RFC 9562 UUIDv7, 36 characters |
| `int64+` | JSON integer from 1 through 9223372036854775807 |
| `int64?` | `int64+`, or `null` where the member table says so |
| `ms` | `int64+` Unix milliseconds (N009 time form) |
| `rfc3339` | `YYYY-MM-DDTHH:MM:SSZ`, UTC, whole seconds (Premium snapshot time form) |
| `int53` | JSON integer from 0 through 9007199254740991 (Premium snapshot revision form) |
| `hex64` | lowercase SHA-256 hex, 64 characters |
| `cursor` | `v1:<positive canonical signed-64-bit decimal>`, 4 to 22 bytes, or `null` (N009 form) |

The coin wire's decimal-string integers are not used here. No member of either part is an amount. Revisions and times follow N009, the revocation stream and the Premium snapshot, which use JSON integers.

### 1.6 Streams

The stream pattern is N009's:

- An append-only journal is written in the same transaction as the source change, under a single locked counter row, so that a larger position never becomes visible before a smaller one commits. A rollback removes both.
- Items are returned in ascending publication position. `next_cursor` is the position of the last returned item. It is `null` only on an empty page.
- `limit` is from 1 through 500. The page has a byte budget of 262144 bytes (256 KiB). The producer reserves the envelope bytes, measures each canonical item and stops before the budget would be exceeded, so a page can hold fewer items than `limit` and still carry a cursor.
- The consumer persists a cursor only after it has durably applied every item of the page. It keeps the previous cursor on an empty page and polls again. An empty page means "caught up", never "nothing exists".
- History is not pruned. Cursors do not expire and carry no identity, reason or time.

## Part A — Game enforcement command wire (`oteryn-v2-game-enforcement-wire-v1`)

## 2. Scope

### 2.1 What this wire carries

Only in-game penalties that **do not end the session**. Owner decision 1 (2026-11-11) on the revocation stream proposal sends every session-ending cause through `POST /internal/v1/identity/security-revocations/stream/read`. Those causes are bans, suspensions, staff kick, credential and status changes. This wire never carries a session-ending cause, and the revocation stream never carries a mute or a jail.

| Profile | Meaning | Ends the session? | Path |
|---|---|---|---|
| `MUTE` | the character cannot send player chat for a bounded time; Game defines the exact channels | no | **this wire** |
| `JAIL` | the character is confined to a Game-defined jail area for a bounded time | no | **this wire** |
| account ban / suspension | Platform refuses tickets and admissions; live sessions end | yes | revocation stream cause 3 |
| staff kick | live sessions end | yes | revocation stream cause 4 |
| credential / status change | live sessions end | yes | revocation stream causes 1, 2 |
| deny new admission (account) | Platform refuses tickets and admissions | n/a | existing Platform admission gate (`require_game_access`), not this wire |

The architecture's families "deny new game admission" and "disconnect or restrict an active session" are therefore **not** v1 profiles of this wire. Character ban, namelock, IP, device, trade, market and movement restrictions other than jail are not implied (architecture: *Command families*). Each needs its own accepted policy and a later wire version.

### 2.2 Relation to the N009 sanction export

N009 exports a player-visible record of kinds `1 mute` and `2 jail`. Game "does not enforce decisions from this feed", and N009 has no producer for those kinds today. This wire is the enforcement-bearing path for the same decisions. Proposal E2:

- the command's `sanction_stream_id` is the N009 `sanction_stream_id` of the same typed sanction;
- the command's `decision_revision` is the N009 `decision_revision` of the decision it enacts;
- N009 also allocates revisions for display-only changes (reason metadata, appeal review). The enforcement stream therefore sees gaps in `decision_revision`, which is allowed: ordering is "higher wins", never "next expected".

Game keeps using N009 for display only. Only a command on this wire changes game state, and only Game's result proves the effect.

### 2.3 Intents in v1

| Intent | Meaning | `decision_revision` |
|---|---|---|
| `APPLY` | create the restriction of a new stream | the stream's first decision |
| `REPLACE` | atomically change the stream's times or reason under a newer decision; the profile is immutable per stream, as in N009 | strictly greater than `prior_decision_revision` |
| `REVOKE` | end the stream's restriction now, by a newer authorized decision (for example an accepted appeal outcome) | strictly greater than `prior_decision_revision` |
| `EXPIRE` | ask Game to reconcile the authoritative end of a time-bounded stream. Platform emits it once its own clock has passed `expires_at_ms`. Game alone decides, on Game time, whether the end has been reached (architecture: expiry is not inferred from a Platform clock) | equal to the decision whose end it reconciles; not a new decision |

`RECONCILE` as a command intent is deferred. In v1, reconciliation is by operation identity through route A4 (§5) and Game's result republication.

## 3. Routes

All paths start with `/internal/v1/support/enforcement-commands/`, beside N009's `/internal/v1/support/sanctions/`.

| # | Path | Operation | Request bound | 200 bound | Group |
|---|---|---|---|---|---|
| A1 | `stream/read` | read commands in publication order | 256 | 262144 | reader |
| A2 | `commands/read` | read one command by `operation_id` | 256 | 1024 | reader |
| A3 | `results` | publish one authoritative Game result | 1024 | 128 | results |
| A4 | `reconcile/read` | read Platform's view of every operation | 256 | 262144 | reader |

### 3.1 A1 `stream/read`

Request, at most 256 bytes:

```json
{"schema":"oteryn.enforcement_command_stream_request.v1","after":null,"limit":200}
```

`after` is `null` (from the start of history) or a `cursor` returned by this route. `limit` is an integer from 1 through 500. A cursor beyond committed history answers `400`.

200 body: `schema` (`oteryn.enforcement_command_stream_page.v1`), `items`, `next_cursor`, in that order. Each item is one **command** (§3.2). It is byte-identical to the A2 answer for its `operation_id`.

Ordering:

- items are in ascending publication position;
- `operation_id` is unique across the stream;
- within one `sanction_stream_id`, items appear in non-decreasing `decision_revision`;
- a page can hold several commands of one stream.

### 3.2 Command

The command members are listed in wire order. Every member is present; `null` only where the table allows it.

| # | Member | Type | Rule |
|---|---|---|---|
| 1 | `operation_id` | `uuid7` | one semantic attempt; Platform-minted, never reused, also not after a Platform restore |
| 2 | `sanction_stream_id` | `uuid7` | stable lineage (§2.2) |
| 3 | `command_version` | integer | `1` |
| 4 | `intent` | string | `APPLY` \| `REPLACE` \| `REVOKE` \| `EXPIRE` |
| 5 | `profile` | string | `MUTE` \| `JAIL`; immutable per stream |
| 6 | `decision_revision` | `int64+` | §2.3 |
| 7 | `prior_decision_revision` | `int64?` | `null` for `APPLY` and `EXPIRE`; for `REPLACE` and `REVOKE`, the revision Platform holds as current, which is the precondition |
| 8 | `policy_revision` | `int64+` | Platform moderation policy revision the decision was taken under |
| 9 | `account_id` | `uuid7` | canonical Platform `AccountId` |
| 10 | `character_id` | `uuid7` | game-owned `CharacterId`; both v1 profiles are character-scoped, as N009 kinds 1 and 2 |
| 11 | `effective_at_ms` | `ms` or `null` | required for `APPLY` and `REPLACE`; `null` for `REVOKE` (immediate) and `EXPIRE` |
| 12 | `expires_at_ms` | `ms` or `null` | required and greater than `effective_at_ms` for `APPLY` and `REPLACE`, because both profiles are time-bounded (N009 kinds 1 and 2 require an end); `null` otherwise |
| 13 | `reason_code` | integer or `null` | 1..65535, N009 public reason category; `null` only for `EXPIRE` |
| 14 | `decided_at_ms` | `ms` | Platform decision time; recorded and displayed, never used for ordering |
| 15 | `audit_ref` | `uuid7` | opaque handle of the Platform audit record; carries no staff identity, case body or evidence |
| 16 | `correlation_id` | `uuid7` | non-secret correlation for logs and traces |

The command carries no reporter identity, moderator notes, free-form text, evidence handle, IP, device or credential data. The architecture's `actor_context_reference`, `source_case_reference` and `evidence_reference_set` stay on the Platform side and are reachable through `audit_ref` (owner decision E5). `precondition_evidence` is `prior_decision_revision`.

`command_digest` = lowercase SHA-256 of the exact command bytes as A1 and A2 serve them.

Worst case: 638 bytes per command. A full 200-item consumer page is at most 127904 bytes. A 500-item page would exceed the budget, so the §1.6 byte budget applies.

### 3.3 A2 `commands/read`

Request, at most 256 bytes: `{"schema":"oteryn.enforcement_command_read_request.v1","operation_id":"<uuid7>"}`.

| Status | Meaning |
|---|---|
| `200` | the command bytes, verbatim |
| `404` | no command with that `operation_id` |
| `400`, `401`, `429`, `503` | §1.3, §1.4 |

A2 exists so that Game can fetch one command it lacks after reading A4, for example after a Game restore, without rescanning history.

## 4. A3 `results`: Game's authoritative result

### 4.1 Body

The body is canonical (§1.4), at most 1024 bytes. Members in wire order:

| # | Member | Type | Rule |
|---|---|---|---|
| 1 | `schema` | string | `oteryn.enforcement_result.v1` |
| 2 | `operation_id` | `uuid7` | the command's |
| 3 | `sanction_stream_id` | `uuid7` | equal to the command's |
| 4 | `command_digest` | `hex64` | SHA-256 of the command bytes Game decided on |
| 5 | `receipt_revision` | integer | 1..65535; increases per `operation_id` |
| 6 | `state` | string | `ACCEPTED_PENDING` \| `APPLIED` \| `REPLACED` \| `REVOKED` \| `EXPIRED` \| `REJECTED` |
| 7 | `result_code` | string | §4.2 |
| 8 | `accepted_decision_revision` | integer | 0..9223372036854775807; Game's durable high-water `decision_revision` for the stream after this decision; `0` only when Game holds no accepted decision for the stream |
| 9 | `stream_state` | string | `NONE` \| `PENDING` \| `ACTIVE` \| `REVOKED` \| `EXPIRED`; the stream's authoritative state after this decision |
| 10 | `effective_until_ms` | `ms` or `null` | the authoritative end when `stream_state` is `PENDING` or `ACTIVE`; `null` otherwise |
| 11 | `runtime_effect` | string | `ONLINE_APPLIED` (a live session of the character reached the profile's terminal runtime outcome) \| `ON_NEXT_ADMISSION` (no live session; Game applies the restriction when the character is next admitted) \| `NONE` (rejected, or no runtime action needed) |
| 12 | `capability_revision` | string | `[a-z0-9._:-]{1,64}`; Game's enforcement capability and profile revision |
| 13 | `recovery_generation` | `int64+` | Game recovery fence asserted by the deciding transaction (restore contract) |
| 14 | `decided_at_ms` | `ms` | Game transaction time; recorded, never used for ordering |

Worst case: 649 bytes.

### 4.2 States and result codes

| `state` | Terminal | Allowed `result_code` | Intents |
|---|---|---|---|
| `ACCEPTED_PENDING` | no | `PENDING_RUNTIME` (durably accepted, runtime outcome not yet reached), `PENDING_EXPIRY` (EXPIRE accepted, Game time has not reached the end) | all |
| `APPLIED` | yes | `OK` | `APPLY` |
| `REPLACED` | yes | `OK` | `REPLACE` |
| `REVOKED` | yes | `OK` | `REVOKE` |
| `EXPIRED` | yes | `OK` | `EXPIRE` |
| `REJECTED` | yes | one rejection code below | all |

Rejection codes, closed, mapped from the architecture's *Typed rejection*:

| Code | Meaning |
|---|---|
| `UNSUPPORTED` | command version, profile or scope not supported; the only code for a profile Game has not enabled |
| `TARGET_NOT_APPLICABLE` | target not found or not applicable; one code for both, so existence is not disclosed |
| `TARGET_MISMATCH` | `character_id` is not owned by `account_id` in Game |
| `STALE_REVISION` | `decision_revision` (or, for `REPLACE` and `REVOKE`, `prior_decision_revision`) is not the stream's current; `accepted_decision_revision` and `stream_state` carry the authoritative current state |
| `LIFECYCLE_CONFLICT` | character lifecycle or topology conflict, for example deletion or transfer in progress |
| `OPERATION_CONFLICT` | the `operation_id` is already bound to a different command digest in Game |
| `CAPABILITY_CONFLICT` | `policy_revision` or the capability is incompatible |
| `INTERNAL_NON_RETRYABLE` | internal failure for which no effect can later commit |

A retryable failure, a dependency outage, or any failure after possible acceptance is **not** a result. Game publishes nothing, and Platform keeps the operation `PENDING`.

### 4.3 Platform handling

Platform handles a result in one transaction that locks the operation row.

| Status | Body | When |
|---|---|---|
| `200` | `{"schema":"oteryn.enforcement_result_ack.v1","result":"accepted"}` | the first result for (`operation_id`, `receipt_revision`), or a byte-identical republication of a stored one |
| `400` | empty | not canonical; wrong member set; `state` / `result_code` / intent combination not allowed; `sanction_stream_id` not the command's; `effective_until_ms` nullability wrong |
| `404` | empty | no command with that `operation_id`. Game keeps its result, stops republishing and alarms |
| `409` | empty | different bytes for a stored (`operation_id`, `receipt_revision`); a `command_digest` that is not the stored command's; a `receipt_revision` above a stored terminal result |
| `401`, `429`, `503` | empty | §1.3; on `503` Game republishes later |

Rules:

- Platform's operation state is the state of the highest stored `receipt_revision`. A late non-terminal result below a stored terminal one is stored as history and changes nothing.
- A terminal result is final for the operation. Exactly one terminal result exists per `operation_id`.
- Platform never infers a result from time, from absence, or from N009.

## 5. A4 `reconcile/read`

Request: `{"schema":"oteryn.enforcement_reconcile_request.v1","after":null,"limit":200}`, with the §3.1 rules. The cursor positions are the A1 publication positions.

200 body: `schema` (`oteryn.enforcement_reconcile_page.v1`), `items`, `next_cursor`. It covers **every** command in publication order, in any state. Item members in order:

| Member | Type | Rule |
|---|---|---|
| `operation_id` | `uuid7` | |
| `sanction_stream_id` | `uuid7` | |
| `state` | string | `PENDING` (no stored result) or the `state` of the highest stored result |
| `receipt_revision` | integer or `null` | of the highest stored result; `null` iff `PENDING` |
| `result_digest` | `hex64` or `null` | SHA-256 of that stored result's bytes; `null` iff `PENDING` |

Worst case: 250 bytes per item.

Game compares each item with its own receipt store. Drift raises an operator alarm and has a bounded metric. It never causes an automatic sanction or deletion. The drift cases are:

- `PENDING` while Game holds a result: Game's outbox republishes it;
- a digest that differs from Game's stored result: alarm on both sides; neither rewrites the other;
- a command Game does not know: Game fetches it through A2 and decides it normally. Revision fencing makes a late command harmless.

## 6. Semantics across the boundary

### 6.1 Platform side (outbox)

- A moderator decision with a profile enabled in `PORTAL_GAME_ENFORCEMENT_PROFILES` is written in one transaction with these effects:
  - the Platform sanction record and its audit row;
  - the N009 journal row (when public);
  - the command journal row, which is the outbox (`crates/db` outbox).

  The command is immutable once written.
- A decision for a profile that is not enabled is a Platform-local record only. The UI labels it "not enforced in game". Enabling the profile later does not backfill old decisions (owner decision E8).
- Platform authorization, confirmed MFA and the exact permission are checked at decision time. The command is created server-side. Browser input never supplies the stream, revision or operation identities.

### 6.2 Game side

- Game applies each command at most once per `operation_id`. It stores the command digest with its result in the deciding transaction. An exact re-read republishes the stored result. The same `operation_id` with another digest is `OPERATION_CONFLICT`.
- Game keeps a durable per-stream high-water `decision_revision`. Any `APPLY`, `REPLACE`, `REVOKE` or `EXPIRE` below it, or with a stale precondition, is `STALE_REVISION`. An older command never weakens a newer decision. A restore never lowers the high-water mark without the restore procedure (§6.4).
- `REPLACE` is atomic in Game, so no gap appears between the old and the new restriction.
- `EXPIRE` is idempotent. It names one stream and cannot touch another.
- Game writes each result into its own outbox in the deciding transaction and publishes it to A3 at least once.

### 6.3 Timeouts and ambiguity

- A timeout or lost response on A3 is neither success nor failure. Game republishes the identical bytes until it gets `200`, `404` or `409`.
- A Platform operation without a terminal result stays `PENDING`. This is the architecture's Platform-local `AMBIGUOUS`. Platform never mints a replacement operation for it.
- Platform raises the operator alarm `support.enforcement_result_overdue` when an operation has no result 15 minutes after publication (proposal E7). The alarm changes no state.
- `UNKNOWN_TO_GAME` is never published in v1. Game never proves non-acceptance on this wire, so Platform treats a missing result as pending, not as safe to replace.

### 6.4 Restore

- **Game restore.** It uses the existing `POST /internal/v1/game-auth/game-restore/notice`, amended, as the Store wire does in §7, to reopen enforcement results whose `recovery_generation` ≥ the restored generation, and to refuse later results below the notice's new generation with `409`. This amendment is a precondition for enabling the wire (owner decision E6). Until it lands, the results group must stay disabled on a stack whose restore surface is enabled.
- **Platform restore.** A Platform restore can lose commands that Game already decided. Game's republication then gets `404` and alarms. Operator reconciliation against Game's results is required. This is the same limit as PLATFORM-PITR-FENCE-1.

### 6.5 Appeals and presentation

- An appeal submission produces no command.
- An accepted appeal outcome produces a `REVOKE` or `REPLACE` with a new `operation_id` and a higher `decision_revision`.
- Platform shows "enforced in game" only from a stored `APPLIED`, `REPLACED`, `REVOKED` or `EXPIRED` result. It shows "pending in game" otherwise. It never derives the game state from N009 or from its own clock.

### 6.6 Configuration

| Variable | Default | Effect |
|---|---|---|
| `PORTAL_GAME_ENFORCEMENT_ENABLED` | off | routes A1 to A4; anything but literal `true` answers `503` |
| `PORTAL_GAME_ENFORCEMENT_PROFILES` | `[]` | JSON list over `MUTE`, `JAIL`; a profile not listed produces no command (§6.1); an unknown value disables the surface (`503`) |
| `PORTAL_GAME_ENFORCEMENT_READER_IDENTITY` | none | DNS SAN of the reader group (A1, A2, A4); dedicated; takes part in the overlap check |
| `PORTAL_GAME_ENFORCEMENT_RESULTS_IDENTITY` | none | DNS SAN of the results group (A3); dedicated; must differ from the reader identity |
| `PORTAL_GAME_ENFORCEMENT_REQUESTS_PER_MINUTE` | `1200` (1..6000) | per-process peer budget of each group |

The two identities follow the one-identity-per-purpose rule. A reader certificate cannot forge results, and a results certificate cannot read moderation commands.

### 6.7 Failure matrix

| Situation | Platform answer | Platform state | Game action |
|---|---|---|---|
| flag off, identity missing, malformed or overlapping | `503` | unchanged | retry later; nothing applied |
| wrong or unverified peer, TLS < 1.3, header-only identity | `401` | unchanged | configuration alarm |
| malformed, oversized, unknown member, bad cursor | `400` | unchanged | defect; never retried unchanged |
| budget exhausted | `429` + `Retry-After: 60` | unchanged | retry after delay |
| database down or read timeout | `503` | unchanged | retry; current restrictions unchanged |
| duplicate command delivery (rescan, replay) | `200` | — | idempotent by `operation_id`; republish stored result |
| command older than Game's high-water mark | — | — | `REJECTED` / `STALE_REVISION` with current state |
| profile not supported in Game | — | — | `REJECTED` / `UNSUPPORTED`; never broadened to another profile |
| Game crash after deciding, before publishing | — | `PENDING` | outbox publishes after restart |
| A3 timeout or lost response | — | stored or not | republish identical bytes |
| A3 result with different bytes for a stored key | `409` | unchanged | alarm both sides |
| A3 for an unknown operation (Platform restore) | `404` | unchanged | stop republishing; alarm; operator reconciles |
| no result after 15 min | — | `PENDING` + alarm | — |
| Game restore | per §6.4 | reopened | republish new results |

### 6.8 Owner decisions needed (Part A)

| # | Decision | Options | Recommendation |
|---|---|---|---|
| E1 | v1 profiles | (a) `MUTE`, `JAIL` only; (b) add character ban or namelock | **(a).** Session-ending causes go through the revocation stream (owner decision 1). Each other profile needs its own policy. |
| E2 | lineage shared with N009 | (a) same `sanction_stream_id` and `decision_revision`; (b) independent ids | **(a).** Game can join display and enforcement without a mapping table; gaps are harmless. |
| E3 | identities | (a) separate reader and results identities; (b) one identity for all four routes | **(a)**, per the one-identity-per-purpose convention and least privilege. |
| E4 | capability negotiation | (a) static Platform flag list plus Game's `UNSUPPORTED` rejection; (b) a Game-posted capability route | **(a)** for v1: no new route, fails closed both ways. |
| E5 | evidence across the boundary | (a) `audit_ref` only; (b) opaque evidence handles | **(a).** Game does not need evidence to execute a mute or jail. |
| E6 | Game restore | amend the restore notice before enabling | **Yes**, as a precondition, mirroring Store wire §7. |
| E7 | overdue-result alarm | 15 min | **15 min.** Mute and jail are short; a quarter hour without a result is an incident, not noise. |
| E8 | decisions while a profile is off | (a) Platform-local only, no backfill; (b) backfill on enable | **(a).** A backfill would enact stale decisions without a fresh moderator decision. |

## Part B — Profile-B entitlement authority wire (`oteryn-v2-profile-b-authority-wire-v1`)

## 7. Scope

- **What it covers.** Part B gives every Profile-B product/version **other than** `oteryn.premium_time` v1 one generic authority read, B1. It also gives every Profile-B product, Premium included, one change feed, B2.
- **Premium time.** `oteryn.premium_time` v1 keeps its existing route and wire (`OTERYN_V2_PREMIUM_TIME_SNAPSHOT_CONTRACT.md`) unchanged (owner decision P3).
- **Other profiles.** Profile C item delivery is proposed in the Store wire (`OTERYN_V2_STORE_WIRE_CONTRACT.md`). Profile D goes through `OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md`. Profile E is the Platform Wallet. Part B adds no delivery route for C, D or E.
- **Products.** Part B approves no product. A product/version may use B1 only after its own product contract names it, declares Profile B, declares the lease policy of §9, and has at most one entitlement per (`account_id`, `product_id`, `product_version`), as Premium v1 does (§2.1 of that contract).

## 8. Routes

All paths start with `/internal/v1/products-entitlements/profile-b/`.

| # | Path | Operation | Request bound | 200 bound | Group |
|---|---|---|---|---|---|
| B1 | `authority/read` | read the current authority of one account for every enabled product | 256 | 8192 | authority |
| B2 | `changes/read` | read lifecycle change hints in publication order | 256 | 262144 | changes |

### 8.1 B1 request

This request mirrors the Premium snapshot request:

```json
{"schema":"oteryn.entitlement_authority_request.v1","account_id":"<uuid7>","nonce":"<32 lower hex>"}
```

- `account_id` is a canonical UUIDv7.
- `nonce` is 128 fresh bits from Game per request, as 32 lowercase hexadecimal characters. Platform echoes it and does not store it.

### 8.2 B1 response

200 body, canonical, at most 8192 bytes. Members in order:

| # | Member | Type | Rule |
|---|---|---|---|
| 1 | `schema` | string | `oteryn.entitlement_authority.v1` |
| 2 | `producer_revision` | string | `PLATFORM_BUILD_REVISION`, 40 lowercase hex |
| 3 | `producer_profile` | string | `oteryn.entitlement.profile_b.v1` |
| 4 | `nonce` | string | the request's, byte for byte |
| 5 | `account_id` | `uuid7` | the request's |
| 6 | `authority_revision` | `int53` | at least 1; strictly increasing per account across every B1 answer (§10.2) |
| 7 | `authority_issued_at` | `rfc3339` | Platform time of issue |
| 8 | `entitlements` | array, 0..16 | exactly one item per enabled product/version in `PRODUCTS_ENTITLEMENTS_PROFILE_B_PRODUCTS`, in that list's order |

Each `entitlements` item has these members, in order:

| Member | Type | Rule |
|---|---|---|
| `product_id` | string | Store wire `product_id` form, at most 64 bytes |
| `product_version` | `int53` | at least 1 |
| `entitlement_id` | `uuid7` or `null` | `null` iff `NONE` |
| `entitlement_state` | string | `ACTIVE` \| `NOT_YET_EFFECTIVE` \| `EXPIRED` \| `REVOKED` \| `NONE` |
| `lifecycle_revision` | `int53` | at least 1, or `0` iff `NONE` |
| `effective_from` | `rfc3339` or `null` | `null` iff `NONE` |
| `effective_until` | `rfc3339` or `null` | `null` iff `NONE` or the commercial interval is open-ended. An open end never makes authority open-ended. |
| `authority_valid_until` | `rfc3339` | finite cutoff, §9.2 |
| `refresh_after` | `rfc3339` | §9.2 |
| `lease_policy` | string | `oteryn.profile_b.lease.v1` (§9.1); Game rejects any other value |

- **Explicit `NONE`.** `NONE` is negative evidence. A product with no entitlement is still listed, so absence is never confused with an outage.
- **State at issue.** Platform classifies each item at `authority_issued_at` with the Premium snapshot's §5.3 order: `NONE`, `REVOKED`, `EXPIRED`, `NOT_YET_EFFECTIVE`, `ACTIVE`.
- **State is not a grant.** `entitlement_state` is commercial truth at issue time. It is not authorization by itself (§9.3).
- **Size.** Worst case: 460 bytes per item, and 7732 bytes for 16 items.

### 8.3 B1 outcomes

| Condition | Status |
|---|---|
| authority issued | `200` |
| malformed request | `400` |
| wrong peer | `401` |
| no Platform identity has this `account_id`; decided before any revision is allocated | `404` |
| budget exhausted | `429` + `Retry-After: 60` |
| off, misconfigured, database unavailable, invalid durable state | `503` |

Game treats every non-200 status, and every 200 that fails §10.3, as **no fresh authority**. Existing accepted evidence then runs out on its own absolute times.

### 8.4 B2 change hints

Request: `{"schema":"oteryn.entitlement_change_request.v1","after":null,"limit":200}`, with the §1.6 stream rules.

200 body: `schema` (`oteryn.entitlement_change_page.v1`), `items`, `next_cursor`. Item members, in order:

| Member | Type | Rule |
|---|---|---|
| `account_id` | `uuid7` | |
| `product_id` | string | any Profile-B product, `oteryn.premium_time` included |
| `product_version` | `int53` | |
| `lifecycle_revision` | `int53` | the entitlement's new lifecycle revision |

Worst case: 206 bytes per item, and 103596 bytes for a 500-item page.

- **Publication.** One item is written in the transaction of every grant, extension, revocation or other lifecycle change. Expiry is not a stored transition (Premium §2.4), so it publishes nothing. Its absolute time already fences it.
- **A hint, never authority.** An item can only **reduce** benefit. On an item whose `lifecycle_revision` is higher than Game's accepted one for that (account, product, version), Game stops the benefit at once and refreshes through B1, or through the Premium snapshot for `oteryn.premium_time`. A hint never starts, extends or restores benefit.
- **Why a feed.** Without the feed, an unseen revocation lasts at most until `refresh_after` (40 minutes, §9). With the feed polled every 10 seconds, it lasts about 10 seconds plus one B1 round trip while Platform is reachable.

### 8.5 Configuration

| Variable | Default | Effect |
|---|---|---|
| `PRODUCTS_ENTITLEMENTS_PROFILE_B_AUTHORITY_ENABLED` | off | B1; anything but literal `true` answers `503` |
| `PRODUCTS_ENTITLEMENTS_PROFILE_B_AUTHORITY_MTLS_CLIENT_IDENTITY` | none | DNS SAN of the authority group; dedicated; overlap-checked |
| `PRODUCTS_ENTITLEMENTS_PROFILE_B_AUTHORITY_REQUESTS_PER_MINUTE` | `1200` (1..6000) | B1 budget |
| `PRODUCTS_ENTITLEMENTS_PROFILE_B_PRODUCTS` | `[]` | JSON list of `"<product_id>@<product_version>"`, at most 16 entries, no duplicates, never `oteryn.premium_time@1`; anything else disables B1 (`503`) |
| `PRODUCTS_ENTITLEMENTS_PROFILE_B_CHANGES_ENABLED` | off | B2 |
| `PRODUCTS_ENTITLEMENTS_PROFILE_B_CHANGES_MTLS_CLIENT_IDENTITY` | none | DNS SAN of the changes group; dedicated; overlap-checked |
| `PRODUCTS_ENTITLEMENTS_PROFILE_B_CHANGES_REQUESTS_PER_MINUTE` | `1200` (1..6000) | B2 budget |
| `PLATFORM_BUILD_REVISION` | none | required by B1; missing or malformed answers `503` |

Lease numbers are **not** configuration (§9.1).

## 9. Profile-B lease values (PROPOSED)

### 9.1 Policy `oteryn.profile_b.lease.v1`

These values are proposed as **fixed contract values**, not deployment configuration. The generic contract says that widening a lease is an authorization-policy change. Premium v1 already reads it that way (§3 of that contract). A product needing other values needs a new `lease_policy` id in an amended contract.

| Policy (generic contract name) | Proposed value | Reason |
|---|---|---|
| `max_authority_lease` (L) | **3600 s** | Same as accepted Premium v1. It bounds the life of any accepted active evidence to one hour, even across a Platform outage or a lost change feed. One policy for every Profile-B product keeps Game's evaluator single. |
| `refresh_before`, on the wire `refresh_after` | **issued + floor(2 × (authority_valid_until − issued) / 3)**: 2400 s for a full lease | Same rule as Premium v1. It always precedes a cutoff that the commercial end has clipped, as the generic contract requires. |
| Game renewal cadence | **one B1 read on admission and reconnect; then, while the account is online and holds the benefit, one read at issued + L/2 (1800 s) with up to 120 s random jitter; on failure, retry with backoff from 15 s doubling to 120 s, within `Retry-After`** | The first renewal starts 600 s before `refresh_after`, which leaves at least four retries before benefit stops. Jitter spreads load after a mass login. |
| `max_clock_skew` | **5 s** | Same as Premium v1. Both sides run NTP-disciplined servers. Game fails closed when its trusted-time uncertainty exceeds 5 s. |
| `stale_within_bound` | **DENY** | Zero stale grace, as Premium v1. From `refresh_after` without newer accepted evidence, the evidence is `STALE_WITHIN_BOUND` and the benefit is denied. |
| maximum offline benefit since the last successful B1 read | **2400 s (40 min)**; the evidence becomes `EXPIRED` at 3600 s | This follows from DENY: benefit ends at `refresh_after`. The 3600 s cutoff only bounds how long the evidence classifies as stale rather than expired. |
| `authority_expired` | **deny** until a fresh B1 answer is accepted **and** trusted time is within 5 s again | Required by the generic contract. |
| authority-time recovery anchor | **none in v1** | A fresh answer alone never restores authority while time is unsafe (generic contract). Game waits for its trusted-time source to return within 5 s. |
| change-feed poll (B2) | **every 10 s** while healthy; backoff as above on failure | It bounds unseen revocation to about 10 s while Platform is reachable, at 6 requests per minute against a budget of 1200. |

Capacity check. At one renewal per online account per 1800 s, the default B1 budget of 1200 per minute serves about 36000 concurrently entitled online accounts, before logins. A larger population raises `PRODUCTS_ENTITLEMENTS_PROFILE_B_AUTHORITY_REQUESTS_PER_MINUTE` (maximum 6000). That raise widens no authorization.

### 9.2 Cutoff values on the wire

With `issued = authority_issued_at` and `L = 3600 s`:

```text
ACTIVE, NOT_YET_EFFECTIVE:
    authority_valid_until = min(issued + L, effective_until)     (issued + L when effective_until is null)
EXPIRED, REVOKED, NONE:
    authority_valid_until = issued + L
refresh_after = issued + floor(2 × (authority_valid_until − issued) / 3)
```

So `issued ≤ refresh_after < authority_valid_until ≤ issued + 3600 s`. For every usable active item, `authority_valid_until ≤ effective_until`. Platform may issue an `ACTIVE` item whose `effective_from` lies in the future, up to one lease ahead. Game classifies that item `NOT_YET_EFFECTIVE` until the start is proven reached.

### 9.3 Cutoff evaluation in Game

Let `t` be trusted server time and `u` its current uncertainty bound. `earliest = t − u` and `latest = t + u`. Benefit for one item is permitted only when **all** of the following hold:

1. `u ≤ 5 s`;
2. the item is from the newest accepted B1 answer (§10.3), and its `entitlement_state` is `ACTIVE` or `NOT_YET_EFFECTIVE`;
3. `earliest ≥ effective_from`. This is the not-before check: uncertainty delays the start;
4. `latest < min(effective_until, refresh_after)`, with `effective_until` treated as +∞ when it is `null`. This is the not-after check: uncertainty shortens the usable lease;
5. no B2 hint with a higher `lifecycle_revision` has arrived since that answer.

Classification follows the generic contract's precedence, mapped onto these values:

| Classification | Condition | Benefit |
|---|---|---|
| `REVOKED` | accepted item state `REVOKED` | deny |
| `EXPIRED` | accepted item state `EXPIRED`, or `latest ≥ effective_until`, or `latest ≥ authority_valid_until` | deny |
| `NOT_YET_EFFECTIVE` | active evidence and `earliest < effective_from` | deny |
| `STALE_WITHIN_BOUND` | active evidence, `refresh_after ≤ latest < authority_valid_until`, or a newer B2 hint is pending | deny (DENY policy) |
| `AUTHORITY_UNAVAILABLE` | no accepted evidence for the product, and no fresh answer | deny |
| `CURRENT_AUTHORITY` | all five conditions above hold | permit |

Running sessions: at or before the conservative cutoff, the benefit moves to the product-defined non-entitled behavior. The session itself is not disconnected; forced-disconnect policy stays deferred, as in the generic contract.

## 10. Ordering and fencing (Part B)

### 10.1 `lifecycle_revision`

`lifecycle_revision` increases by exactly 1 on each accepted lifecycle change of one entitlement, and on nothing else (Premium §6.1). Within one lifecycle revision, `entitlement_id`, the stored state, `effective_from` and `effective_until` are immutable (Premium §6.2).

### 10.2 `authority_revision`

Each account has one Profile-B authority row for B1. It is separate from the Premium authority row, so the two sequences do not interact. Every B1 answer, and every lifecycle change of any product listed in `PRODUCTS_ENTITLEMENTS_PROFILE_B_PRODUCTS`, first locks that row with `SELECT … FOR UPDATE`. Each B1 answer increments `authority_revision` in the transaction that reads the entitlements. The revision is therefore strictly increasing per account across all products. A higher revision never reflects an older lifecycle state.

### 10.3 Consumer fence (Game)

Game accepts a B1 `200` only when all of these hold:

- the `nonce`, `account_id`, `schema`, `producer_profile` and `lease_policy` are exact;
- the body passes every rule in §8.2 and §9.2.

Beyond those checks, Game keeps durable fences:

- a durable per-account high-water `authority_revision`. It rejects any answer at or below that revision;
- per (account, product, version), a durable high-water `lifecycle_revision`. It rejects a lower one. For an equal lifecycle revision, it rejects a contradicting `entitlement_id`, interval or state, as conflicting authority evidence;
- restart, reconnect, outage, cache replay or snapshot restore never moves `effective_from` earlier, never extends `authority_valid_until`, and never lowers a fence.

Platform database rollback has the effect of Premium §6.5: the fence rejects re-issued low revisions and fails closed until new revisions exceed the high-water mark.

### 10.4 Failure matrix (Part B)

| Situation | Platform answer | Game classification | Benefit |
|---|---|---|---|
| flag off, misconfigured, overlapping identity | `503` | evidence ages on its own times | until `refresh_after`, then deny |
| Platform down for 30 min | `503` or timeout | `CURRENT_AUTHORITY` until `refresh_after`, then `STALE_WITHIN_BOUND` | at most 40 min after the last read |
| Platform down for more than 60 min | — | `EXPIRED` | deny until a fresh answer and safe time |
| revocation, feed healthy | B2 item | `STALE_WITHIN_BOUND` from the hint, then `REVOKED` after the re-read | stops within about 10 s |
| revocation, feed down | — | until `refresh_after` | stops at most 40 min after the last read |
| commercial end before the lease | — | `EXPIRED` at `effective_until` | commercial end wins |
| pre-issued future start | — | `NOT_YET_EFFECTIVE` | deny until `earliest ≥ effective_from` |
| clock uncertainty > 5 s | — | unsafe time | deny, even with a fresh answer |
| replay of an older answer | — | rejected by the fence | unchanged |
| `404` unknown account | `404` | no fresh authority | existing evidence ages out |
| wrong peer / malformed | `401` / `400` | no fresh authority | existing evidence ages out |

### 10.5 Owner decisions needed (Part B)

| # | Decision | Options | Recommendation |
|---|---|---|---|
| P1 | lease values | the §9.1 table | **Accept as proposed.** They equal the accepted Premium v1 values, so Game needs one evaluator. |
| P2 | stale policy for all v1 Profile-B products | (a) DENY; (b) ALLOW within bound for low-risk products such as cosmetics, up to 20 extra minutes | **(a) DENY.** One rule. An outage of more than 40 min removing benefit is the accepted Premium behavior. |
| P3 | Premium time on B1 | (a) Premium keeps its own route; (b) migrate Premium to B1 under a new product version | **(a)** for v1. The Premium wire is accepted on merge and may already be in Game. |
| P4 | B2 covers Premium | (a) yes, as hints; (b) B2 for B1 products only | **(a).** It closes Premium's 40-minute revocation window with no change to the Premium wire. |
| P5 | authority-time anchor for recovery | (a) none; (b) use `authority_issued_at` as a bounded anchor | **(a).** Simpler, and the generic contract forbids a weaker anchor. |
| P6 | B2 poll cadence | 10 s | **10 s.** |
| P7 | multiplicity | (a) at most one entitlement per account and product/version on B1 v1; (b) several | **(a)**, as Premium. A product needing several needs B1 v2. |
| P8 | identities | (a) separate B1 and B2 identities; (b) one | **(a)**, as the loyalty snapshot and loyalty stream are split. |

## 11. Fixtures

These are synthetic, byte-exact, compact canonical vectors with no terminal newline:

- `docs/contracts/fixtures/game-enforcement-wire-v1/`:
  - `stream-request.json`, `stream-page.json` and `stream-empty.json`;
  - `command-apply.json` and `command-revoke.json`, which are byte-identical to the two page items;
  - `read-request.json`;
  - `result-applied.json` and `result-revoked.json`. Their `command_digest` is the SHA-256 of the matching command fixture;
  - `result-ack.json`;
  - `reconcile-request.json` and `reconcile-page.json`. The page shows the first operation with its result digest, the SHA-256 of `result-applied.json`, and the second still `PENDING`.
- `docs/contracts/fixtures/profile-b-authority-v1/`:
  - `authority-request.json`;
  - `authority.json`, with an `ACTIVE` item on a full lease and a `NONE` item, issued at `2023-11-14T22:13:20Z`;
  - `changes-request.json`, `changes-page.json` and `changes-empty.json`.

The product ids `oteryn.example_active` and `oteryn.example_none` are placeholders, not products. These fixtures change with this proposal until it is accepted. After acceptance, a fixture change is a contract change.

## 12. Validation required before any activation

Platform and Game prove their own sides, then a joint run on the shared fixtures. Beyond the architecture contracts' own lists:

- **Byte equality.** Every route's 200 bytes equal the fixtures. Every `400`, `401`, `404`, `409`, `429` and `503` case is covered, including header-spoofed identity and identity overlap.
- **Part A, results.**
  - An exact A3 republication answers `200` and writes nothing.
  - Changed bytes answer `409`. A result above a stored terminal answers `409`.
  - A late non-terminal result changes nothing.
- **Part A, ordering and scope.**
  - A stale `APPLY`, `REVOKE` or `EXPIRE` is rejected with the current state, and a newer decision is never weakened.
  - `EXPIRE` before Game time reaches the end answers `PENDING_EXPIRY`, then `EXPIRED`.
  - No session-ending cause is ever written to this wire, and no mute or jail to the revocation stream.
- **Part A, flags and restore.**
  - With a profile not enabled, a decision writes no command.
  - Game restore reopens and refuses per §6.4.
- **Part B, evidence.**
  - Every §10.4 row holds.
  - The §9.2 arithmetic holds at boundaries: `issued == effective_until` is `EXPIRED`; a clipped lease has `refresh_after < authority_valid_until`.
  - A B2 hint only ever reduces benefit.
  - `authority_revision` strictly increases under concurrent B1 reads and lifecycle changes.
- **PostgreSQL tests** run with `PORTAL_TEST_DATABASE_URL`. A skipped test is not evidence.

## 13. Ambiguities found in the accepted architecture

These are recorded for the owner. This draft does not resolve them by changing accepted text.

1. **Enforcement families vs owner decision 1.** The enforcement contract still lists "deny new game admission" and "disconnect … an active session" as families. Owner decision 1 sends every session-ending cause to the revocation stream. This draft follows the decision and excludes those families. The enforcement contract may want a one-line note.
2. **`expire` origin and revision.** The enforcement contract lists `expire` as a command intent, and also says expiry is evaluated on game time and "not inferred solely from a Platform clock". It also says a new decision needs a strictly newer revision, but `expire` is not a decision. This draft makes `EXPIRE` a Platform reconcile request that carries the existing revision, and lets Game decide.
3. **`UNKNOWN_TO_GAME` in a pull model.** With no Platform→Game query, Game never proves non-acceptance. This draft never publishes `UNKNOWN_TO_GAME`, and keeps a missing result pending.
4. **N009 mute and jail.** N009 defines kinds 1 and 2 as display-only and has no producer for them. The relationship to the enforcement command is unstated. Proposal E2 links them.
5. **Lease values: configuration or contract.** The generic entitlement contract says numeric lease values "remain product/runtime configuration". Premium v1 fixes them as contract values that are "not deployment configuration". This draft follows Premium: runtime configuration can never widen a lease.
6. **`refresh_before` vs `refresh_after`.** The generic contract names a `refresh_before` value. Premium v1 puts `refresh_after` on the wire, and starts `STALE_WITHIN_BOUND` there. This draft follows Premium.
7. **Open-ended commercial interval.** The generic contract allows an open-ended `effective_until`. Premium v1 never emits one for an active state. This draft allows `null` on B1 and clips the lease to `issued + L`.

## References

- `docs/contracts/OTERYN_V2_GAME_ENFORCEMENT_COMMAND_CONTRACT.md`
- `docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md`
- `docs/contracts/OTERYN_V2_ACCOUNT_SECURITY_REVOCATION_STREAM_CONTRACT_PROPOSAL.md`
- `docs/contracts/OTERYN_V2_SANCTION_EXPORT_CONTRACT.md`
- `docs/contracts/OTERYN_V2_PREMIUM_TIME_SNAPSHOT_CONTRACT.md`
- `docs/contracts/OTERYN_V2_LOYALTY_SNAPSHOT_CONTRACT.md`
- `docs/contracts/OTERYN_V2_STORE_WIRE_CONTRACT.md`, `docs/contracts/OTERYN_V2_STORE_PURCHASE_DELIVERY_CONTRACT.md`
- `docs/contracts/OTERYN_V2_COIN_TRANSFER_AND_MARKET_CONTRACT.md`
- `docs/contracts/OTERYN_V2_GAME_RESTORE_RECONCILIATION_CONTRACT.md`
