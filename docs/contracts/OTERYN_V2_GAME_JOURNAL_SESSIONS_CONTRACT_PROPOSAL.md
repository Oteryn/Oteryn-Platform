Status: ACCEPTED revision 3 by the game owner (coordinator), 2026-11-07. Revision 3 proposed by Platform; the game owner accepted it through the coordinator with the clarifications in the acceptance record (revision 3), which this text applies. Acceptance covers the wire and semantics only; open questions stay open for the Game producer lanes, and producer delivery, consumer qualification and production activation remain separate gates.

# G-9 private game journal and hunting sessions — contract proposal

## Acceptance record (revision 3)

Coordinator verdict on 2026-11-07: **ACCEPT**, with three clarifications:

- **(i) Transfer scope:** character transfer hides and purges only character-scoped data. BANK entries, account achievements and premium stay with the account.
- **(ii) Owner-only sums:** "no aggregates" means nothing across accounts and nothing for staff. Totals computed from one account's own BANK history may be shown to that account's owner, as the owner's own data. Nothing else: no cross-account sums, no G-10 values in admin analytics, no analytical export. G-10 is not a source for any admin analytics family; aggregate economy statistics come from a separate Game producer (ADMINAN lane).
- **(iii) Windows:** every other rev 2 seven-day window is unchanged.

Open questions are resolved in the Game producer lanes and do not block this acceptance. Acceptance does not start Game producers.

## Revision 3 changes

Game-owner decisions relayed by the coordinator on 2026-11-07, common to G-9, G-10 and G-11. Every other revision 2 decision is unchanged; open questions in §12 stay open.

- **1a, owner data export:** added a Platform-only obligation (§8.1): on the owner's request, one JSON export per account per rolling 24 hours, containing only that account's own G-9/G-10/G-11 data; owner-only, authenticated, rate-limited and audited. No Game wire change. Admin and analytical exports remain forbidden.
- **2a, unified deletion:** account or character deletion, opt-out and character transfer hide affected data immediately and purge it within 24 hours (§8.2). Only content-free tombstones (id, epoch, digest) remain, for 120 days, so replay cannot restore deleted data. The rev 2 120-day minimal fence now has this tombstone form, one rule for expiry and deletion. Backups expire on their own schedule; a restore re-applies tombstones before serving or ingest. On transfer the seller's pre-cutoff records of that character are now purged instead of staying private to the seller (§8).
- **3a:** applies to G-10 bank history only; no G-9 change. G-9 records are not a source for admin analytics or any other aggregate or analytical export, except the separately accepted T81 publication described in §8.
- **4a, legal hold:** only the project owner may place one; none exists by default; each is audited with who, when, scope, reason and expiry; it suspends purge only for its scope until expiry or release (§8.3).
- Questions 2, 4 and 6 in §12 are affected by these decisions and remain open until the coordinator accepts revision 3.

## Revision 2 (review of 2026-11-05)

- Re-pinned all Oteryn-Game references to main `2253b997`; marked unverified game paths explicitly.
- Clarified reuse as aligned with the proposed Store wire, without implying acceptance.
- Listed the NEW journal emitter/outbox, durable logout event and replay pull consumer as game-side deliverables; Store outbox remains Store-only.
- Defined entry-to-durable-logout sessions and hunting-area/15-minute kill-idle segments; updated closure rules and acceptance criteria.
- Replaced ownership_revision with character_revision from character command receipts (§4.3 of the character authority contract), including fixtures and privacy requirements.
- Distinguished journal session_id, GameSessionId and CharacterLease ownership, purpose and lifetimes; prohibited reuse.

## 1. Purpose, owners and authority

Proposed contract identifier: `oteryn-v2-game-journal-sessions-v1`; wire `contract_version` is integer `1`.
This is a documentation proposal, not evidence of acceptance, implementation or activation.
It addresses G-9 in `docs/architecture/OTERYN_PORTAL_RUST_PLAN.md`, gate table and §10:

| Consumer | Use |
| --- | --- |
| T72 `playerdata` core | Private ingest, deduplication, watermarks, retention, ownership boundary and flags |
| T73 hunting analyzer | Session/segment totals, comparisons, party loot accounting; pricing remains T74 |
| T75 drops and records | Rare drops, personal records and private character chronicle |
| T81 hunting-place map | Delayed, privacy-controlled aggregates derived by Platform |

Game owns measured gameplay facts, hunting-session boundaries, durable capture and replay.
Platform owns AccountId issuance, authorization, private projections, presentation, valuation and aggregation.
The game owner/coordinator accepts or revises producer obligations; Platform accepts consumer obligations.
Neither side may unilaterally declare G-9 complete. G-10 inventories/trades and G-11 progress/deaths/logins are excluded.
No game mutation, payment, loot grant, schema change, workflow change or deployment is authorized here.

## 2. Evidence and conventions reused

Platform references read for this proposal:

- `docs/contracts/OTERYN_V2_STORE_WIRE_CONTRACT.md`, especially §§2–5 and 9: closed canonical JSON, decimal-string counters, internal mTLS peer guard, exact-byte digests and idempotent delivery. Its status is **PROPOSED**, not accepted.
- `docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md`: accepted consumer architecture; game facts vs disposable projections, independent privacy ordering, gaps and rebuilds. It defers producer transport and does not authorize private journal data in public projections.
- `docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md`: authoritative ownership, stable operations, pending/ambiguous results and reconciliation. Its semantic contract defers cryptographic transport selection.
- `docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md`: durable idempotency and failure/authority separation; entitlement status never substitutes for game facts or ownership.
- `legacy/app/CharacterAuthority/CharacterCommandWire.php`: existing Platform pull/read/receipt wire and `TransferCharacterOwnership` spelling.

Game references are pinned to Oteryn-Game main [2253b997](https://github.com/Oteryn/Oteryn-Game/tree/2253b997):

| Game path at that commit | Fact relied on |
| --- | --- |
| **unverified path**: `crates/gameplay/native-admission-source/src/character_commands.rs` | `ListPendingCharacterCommandsV1` / `ReadCharacterCommandIntentV1` / `PublishCharacterCommandReceiptV1` pattern; purpose-bound mTLS; revision + operation cursor; bounded rescans. This consumer declares `CreateCharacter`; Platform's transfer spelling does not prove Game transfer support here. |
| **unverified path**: `crates/gameplay/native-admission-source/src/store_wire.rs` | Closed member sets/order, decimal strings, `binding_digest`, `COMMITTED` / `REJECTED` receipts |
| **unverified path**: `crates/gameplay/durability/src/durability/store/outbox.rs` | `list_unacknowledged_store_receipts`, `mark_store_receipt_acknowledged`; persisted receipt retry until acknowledgement, Store-only; this is not a journal/ingest outbox |
| **unverified path**: `crates/gameplay/durability/src/durability/character_command.rs` | Durable classification, canonical intent/receipt binding and authoritative commit boundary |
| **unverified path**: `apps/game-server/migrations/0532_bosstiary_kill_receipts.sql` | Character + occurrence keyed immutable boss-kill receipts; these are evidence, not an existing G-9 summary feed |

All journal operation names, routes, fields and limits below are **proposed additions**, not claims that Game has them.
No broker, shared database, new signature scheme or synchronous website-to-runtime dependency is introduced.

### 2.1 Game-side deliverables

The game has no journal/ingest outbox or journal emitter. The existing `store/outbox.rs` is Store-only; it supplies no G-9 delivery capability. All of the following are **NEW game-side work**, with implementation paths unverified:

- Add the durable logout event promised by the game owner, establishing the session end at the authoritative durability boundary.
- Add game-entry/session and hunting-area/kill-idle segment capture, journal identities, metrics and character_revision binding from character command receipts (§4.3 of the character authority contract).
- Add a journal emitter and durable journal/ingest outbox with atomic fact capture, immutable canonical bytes, retries, acknowledgements, retention and restore fencing.
- Add a new pull consumer for `ReplayGameJournalRecords`: pending/read polling, authorization, durable decisions, retained-byte requeueing and receipt delivery. The existing Character Commands consumer does not implement this operation.

## 3. Direction, transport and authentication

Logical flows:

1. **Game → Platform ingest**: durable segment and rare-drop records, retried from a NEW game-side journal outbox (§2.1).
2. **Platform → Game command**: only a request to replay retained journal records. Platform stores an intent; Game pulls it, decides it and publishes a receipt. Platform never opens an HTTP connection to Game. No start/stop hunt or gameplay mutation command is proposed.

All physical requests are Game → Platform `POST` over the existing internal TLS 1.3 mTLS listener, with authenticated Platform server identity and purpose-bound Game client identity. The proposed journal guard is aligned with the proposed Store wire: `SSL_CLIENT_VERIFY=SUCCESS`, `SSL_PROTOCOL=TLSv1.3`, constant-time comparison of `SSL_CLIENT_S_DN` (printable ASCII, 1–128 bytes). Ignore client-supplied equivalents. Journal identity must not overlap Store, Character Commands, public projection, admission or other internal-purpose identities. Bind one configured `source_authority` to the allowed producer account-stream partition; certificate authentication alone does not authorize arbitrary accounts.

Reuse purpose descriptors and approved certificate provisioning/rotation; never put credentials in records. Aligned with the proposed Store wire, mTLS authenticates the exchange; SHA-256 digests bind canonical bytes and are **not signatures**. No detached signature, HMAC key, bearer token or signing field is added. A later requirement for offline-verifiable signing needs an accepted contract revision, not ad hoc signing.

For every body: UTF-8 object, `Content-Type: application/json`, no BOM/trailing newline, no whitespace/escapes/backslash, exact member order below, no missing/extra/duplicate members, depth ≤8. Wire bytes equal compact re-encoding in listed order. `Transfer-Encoding` absent; `Content-Encoding` absent or `identity`; `Content-Length`, if present, decimal without leading zero, at most route bound and matching actual bytes. Nonempty bodies only. This is aligned with the proposed Store wire canonicalization with an explicitly proposed greater depth/size for nested telemetry.

| Proposed route under `/internal/v1/playerdata/journal/` | Operation | Request / 200 response byte ceilings |
| --- | --- | --- |
| `records` | `PublishGameJournalRecordV1` | 1048576 / 64 |
| `commands/pending` | `ListPendingGameJournalCommandsV1` | 512 / 8192 |
| `commands/read` | `ReadGameJournalCommandIntentV1` | 512 / 4096 |
| `commands/receipt` | `PublishGameJournalCommandReceiptV1` | 1024 / 64 |

All responses use `Cache-Control: no-store`. Non-200 bodies are empty. Guard order is aligned with the proposed Store wire: disabled/misconfigured →503; invalid peer →401; per-peer limiter →429 with `Retry-After` seconds ≥1; transport/shape validation; configured authority mismatch →401; handler. Proposed limit 120 requests/60 seconds/peer, burst no larger than 120; retries consume the same budget. Oversize →413; malformed/unsupported schema or operation →400; unknown intent →404; immutable-key conflict or stale stream generation →409; temporary dependency failure →503. An ingest 200 is exactly `{"contract_version":1,"result":"accepted"}` and means durable validated storage plus deduplication, not publication or complete session.

## 4. Shared types and canonical order

Every member is required. Only an explicit `null` union allows null. Bounds are inclusive; no floats, implicit coercion or numeric JSON for decimal-string types.

| Type | Exact representation |
| --- | --- |
| `uuid7` | Lowercase UUIDv7, `^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$` |
| `u64` | String `[1-9][0-9]{0,19}`, ≤18446744073709551615 |
| `counter` | String `0` or `[1-9][0-9]{0,18}`, ≤9223372036854775807 |
| `cursor` | `"0"` or `u64` |
| `key` | ASCII `^[A-Za-z0-9][A-Za-z0-9_.-]*:[A-Za-z0-9_./-]+$`, 3–512 bytes; game catalogue/ruleset-owned |
| `token` | ASCII `[A-Za-z0-9._:/-]{1,128}` |
| `hex64` | 64 lowercase hexadecimal characters, SHA-256 |
| `time` | `u64` Unix seconds UTC, ≤253402300799 |

AccountId, CharacterId, WorldId and ChannelId use the canonical accepted native identity encodings (`uuid7` here); never display names or adapter IDs. `session_id` is a new Game-minted hunting-analysis identity, **never** GameSessionId, admission identity, lease or authentication material. `segment_id`, `event_id`, `operation_id`, `party_id` are separately minted UUIDv7 values, never reused. A proposed `party_id` identifies only this journal party incarnation, not a new canonical game party authority.

Arrays have unique keys and ascending bytewise tuple order by the key columns specified below. Decimal indices compare numerically. Zero means measured zero; null means explicitly unknown where permitted. Empty arrays mean measured none, never failed collection. Total encoded bytes must also fit the route ceiling even when individual array counts fit; no truncation or pagination inside an immutable record is allowed. Capacity above these limits requires a separately accepted version; arbitrary extra segment boundaries are not authorized by this proposal. Missing measurement makes the entire record unavailable; no fabricated zeros.

### 4.1 Session boundaries and identity separation

A journal session runs from game entry to durable logout. Durable logout is a NEW game event to be added by the game owner (§2.1); a disconnect, transport timeout or expired lease alone does not prove logout. Reconnection handling must preserve this boundary using durable game evidence.

| Identity | Owner | Lifetime and purpose |
| --- | --- | --- |
| Journal `session_id` | Game journal producer | Newly minted at game entry, ends at durable logout; groups private analysis segments |
| GameSessionId | Game session authority | Game runtime/admission session lifecycle, independently defined; not an analysis identifier |
| CharacterLease | Game character authority | Character access/authority lease lifetime, independently acquired, renewed and released; not a journal session |

These are different identities with different owners and lifetimes. Never reuse or derive the journal identifier from GameSessionId or CharacterLease, and never transmit either as journal data. Any game-side correlation remains private to Game; exact GameSessionId/CharacterLease implementation paths and lifecycle details are unverified here.

A segment starts on entering a hunting area and ends on leaving that area, or after 15 minutes without a kill (900 seconds from entry if no kill occurs, otherwise from the last authoritative credited kill). After an idle close, no new hunting segment starts until a new hunting-area entry; kill activity alone is not an entry. Durable logout closes an open segment and finalizes the session. Non-hunting time is not fabricated as hunting duration.

## 5. Exact record schemas

### 5.1 Ingest envelope

Members in canonical order:

| Member | Type and constraint |
| --- | --- |
| `contract_version` | integer `1` |
| `operation` | literal `PublishGameJournalRecordV1` |
| `source_authority` | configured `token` |
| `stream_generation` | `uuid7`; new fenced generation after restore, never silently rotated |
| `account_id` | AccountId, owner at capture |
| `source_revision` | `u64`, contiguous from `"1"` per (authority, generation, account) |
| `record_kind` | `HUNT_SEGMENT` or `RARE_DROP` |
| `record` | matching object below |

The envelope digest is SHA-256 of exact canonical request bytes, computed locally; no self-referential digest field.

### 5.2 `HUNT_SEGMENT` record

| Member, in order | Type / meaning |
| --- | --- |
| `session_id`, `segment_id` | `uuid7`, immutable capture identities |
| `segment_index` | integer 0–65535, contiguous per session |
| `character_id`, `world_id`, `channel_id` | canonical IDs |
| `character_revision` | `u64`, character_revision from character command receipts (§4.3 of `docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md`), bound to the character at capture |
| `catalog_revision` | `token`, exact content/ruleset revision resolving all keys |
| `started_at`, `ended_at` | `time`; half-open interval [start,end), end ≥start |
| `duration_seconds` | `counter`, elapsed active hunting seconds, ≤end−start |
| `end_reason` | `AREA_EXIT`, `KILL_IDLE_TIMEOUT`, `CONTEXT_CHANGE`, `SESSION_END`, `RECOVERY_END` |
| `final_segment` | boolean; true iff `SESSION_END` established by durable logout |
| `experience_gained` | `counter`, positive awards in experience points, excludes losses |
| `experience_lost` | `counter`, losses in experience points, separate from gains |
| `level` | integer 1–65535 at segment end, aggregation cohort only |
| `vocation_key` | `key` at segment end |
| `places` | 0–256 Place rows, ordered by `place_key` |
| `loot`, `supplies` | each 0–4096 Item rows, ordered by `item_key` |
| `damage_dealt`, `damage_taken` | each 0–8192 Damage rows, ordered by (`damage_type_key`,`source_kind`,`source_key`), null source sorts first |
| `healing_done`, `healing_received` | `counter`, actual restored hit points, excludes overheal |
| `monsters_killed` | 0–4096 Kill rows, ordered by `monster_key` |
| `rare_drop_count` | integer 0–65535, exact number of distinct rare-drop event records referencing this segment |
| `party` | Party object or null for solo throughout interval |

Nested schemas (members appear in the following order):

| Row | Members, types and semantics |
| --- | --- |
| Place | `place_key`: `key` or null for unclassified location; `duration_seconds`: `counter` ≤segment duration. At most one null row. Durations sum to segment active duration; sequential attribution, no overlapping places. |
| Item | `item_key`: `key`; `quantity`: `u64` ≤9223372036854775807, units of items. Loot = acquired loot, supplies = consumed items; not inventory snapshots or currency valuation. |
| Damage | `damage_type_key`: `key` from pinned damage taxonomy; `source_kind`: `SELF`, `MONSTER`, `PLAYER`, `ENVIRONMENT`, `UNKNOWN`; `source_key`: `key` or null; `amount`: positive `counter`, actual hit points removed after mitigation, excludes overkill. Source means actor causing damage in both arrays, never victim. Only MONSTER carries a catalogue source key; all other kinds require null. No other player's identity. |
| Kill | `monster_key`: `key`; `count`: positive `counter`; authoritative credited kills under pinned ruleset, not inferred from damage |
| Party | `party_id`: `uuid7`; `members`: 1–64 Member rows ordered by `member_slot`; `loot`: 0–4096 PartyItem rows ordered by (`member_slot`,`item_key`) |
| Member | `member_slot`: integer 0–63, unique; `is_subject`: boolean; `active_seconds`: `counter` ≤segment duration |
| PartyItem | `member_slot`: integer 0–63 referencing members; `item_key`: `key`; `quantity`: positive `counter` |

Party slots are stable for this session's party incarnation and are anonymous to the receiving account. Exactly one member is the subject. Party loot is a separate distribution ledger: subject-slot totals equal personal `loot`; never add personal and party totals together. Capturing teammates' attributable loot requires game-owner acceptance of that privacy scope. Until then party capture stays disabled, not silently incomplete. Party/roster/world/channel/character-revision changes close an open segment as `CONTEXT_CHANGE`. A replacement segment requires a hunting-area entry under the new context. These changes do not end the journal session; only durable logout does. Session-relative party slots alone support a private split calculation, not identification or automatic payments; named participants and public sharing need separate acceptance.

Emit a disjoint delta at each segment close defined in §4.1; there is no hourly emission boundary. Segments never overlap, and gaps outside hunting intervals are permitted. Each segment binds its capture account, character and character_revision; session duration and hunting totals are sums of validated segments, not entry-to-logout wall time. Ownership transfer fences the old owner's capture before the commit; no post-cutoff facts enter an old-owner segment. Capture for the new owner requires a new game entry and a separately minted journal session. If durable logout is not yet proven, the old session remains incomplete rather than being falsely finalized by transfer.

If durable logout occurs without an open segment, emit a zero-duration, empty `SESSION_END` closure record at logout using the last captured context, or the entry context if there were no hunting segments. This record only marks finality, adds no hunting time or facts, and has the next contiguous segment index. A session without hunting therefore has only this closure record. Crash recovery may seal the last durable hunting interval as `RECOVERY_END` with `final_segment=false`; session completion still requires durable logout evidence. Lost measurement is an explicit gap, not a complete recap. No per-hit stream is sent.

### 5.3 `RARE_DROP` record

Canonical members: `session_id` (`uuid7`), `segment_id` (`uuid7`), `event_id` (`uuid7`, one authoritative loot occurrence), `character_id` (CharacterId), `character_revision` (`u64`), `occurred_at` (`time` within referenced segment), `catalog_revision` (`token`), `item_key` (`key`), `quantity` (positive `counter`), `monster_key` (`key | null`), `place_key` (`key | null`), `rarity_rule_key` (`key`).

One event per credited item line per occurrence; producer assigns distinct event IDs for distinct lines. The item/quantity is already included in segment loot and is never added again. Rule key resolves the pinned world's rare-drop threshold; Platform value-based highlighting is a separately labelled T74 derivation. Arrival before the segment is allowed but hidden pending validation. Unknown monster/place is null, never guessed. Bounds, owner, character_revision (the same receipt-derived revision as §5.2) and catalogue must match the referenced segment.

## 6. Identifiers, ordering, watermarks and replay

The NEW game-side implementation must commit measured facts, identities, canonical bytes and journal outbox atomically at its authoritative durability boundary; a publisher cannot reconstruct different bytes later. Delivery is at least once. Platform atomically stores validated records, digest, deduplication entries and projection work before acknowledging. A retry after response loss returns the same acknowledgement and has no second effect.

Idempotency scopes:

- Transport: (source_authority, stream_generation, account_id, source_revision).
- Segment: (source_authority, session_id, segment_id); also unique (session_id, segment_index).
- Drop: (source_authority, event_id), with immutable session/segment association.
- Command: (issuer_authority, operation_id); receipt binds exact intent via `binding_digest`.

All keys bind the entire original payload. Same key/different canonical bytes →409, quarantine reference and alarm; no overwrite. Replay republishes original envelopes and revisions, never creates new business identities. Duplicate segment/event under a new revision is conflicting evidence, not an additional fact.

Platform keeps separate highest-seen, highest-contiguous-durably-stored and highest-contiguous-applied watermarks per account stream generation. Only contiguous validated records advance the stored watermark; only semantically resolved records advance applied. Out-of-order valid records may be durably staged and acknowledged; a missing revision, missing segment or poison record blocks completeness, rates, records and T81 contribution for affected sessions. A final segment alone cannot prove completion; all indices 0…final and exactly each segment's `rare_drop_count` distinct drops must reconcile. Game seals that count when committing the segment; late delivery is allowed, late creation of additional occurrences for a sealed segment is not. A count mismatch quarantines the session rather than changing its immutable summary. Times are presentation/measurement facts, not ordering authority. Record absence is not zero/no sessions.

Game retains replayable original bytes for proposed 30 days after capture. Platform may command replay of an explicit contiguous range from the last proven watermark; the command receipt proves enumeration through that range, not Platform application. After ingest of the range and a matching terminal replay receipt, Platform can prove a source prefix; an actively producing stream remains labelled current only as of that prefix. Replay never targets another account. An expired/unavailable range requires explicit unavailable history and a separately agreed authoritative baseline; no blind watermark jump or fabricated replacement. Rebuild off-path from retained evidence plus verified tail, validate gaps/digests, then atomically switch projection generation under current privacy fences.

Restore fencing follows the existing game-restore reconciliation boundary: stop journal application, reconcile the authoritative restore notice and register an accepted new generation before accepting its records. Neither clock rollback nor source_revision reuse may lower durable privacy/ownership fences. Old-generation retries may only match retained exact duplicates; they cannot advance current state. Cross-generation baseline reconciliation remains an activation prerequisite, not authority granted by a random generation UUID.

## 7. Exact replay-command exchange

This requires a NEW game-side pull consumer (§2.1), following the pending/read/receipt pattern. Existing Character Commands and Store routes are unchanged. Replay has no gameplay or ownership mutation authority.

Common request head, in order: `contract_version` (1), `operation` (route literal), `source_authority` (configured Game `token`). Additional members:

| Request | Members after head |
| --- | --- |
| Pending | `after_source_revision`: `cursor`; `after_operation_id`: `uuid7` or nil UUID; `max_entries`: integer 1–32. Zero cursor iff nil UUID. |
| Read | `issuer_authority`: literal `OTERYN_PLATFORM_GAME_JOURNAL`; `operation_id`: `uuid7` |
| Receipt | `issuer_authority`: same literal; `operation_id`: `uuid7`; `binding_digest`: `hex64`; `outcome`: `COMMITTED` or `REJECTED`; `result_code`: enum below; `decided_at`: `time` |

Pending response canonical members: `contract_version` (1), `entries` (0–32 objects with ordered `operation_id`: uuid7, `source_revision`: u64). Order numerically by revision then UUID bytes; strictly after requested pair. No continuation token: next request uses last pair; empty page ends this scan, not all future work. Include unreceipted intents only. Rescan from zero every 30 seconds or eight full pages, whichever first, so changed pending membership cannot lose work.

Read returns exactly the stored intent. Canonical intent members:
`contract_version` (1), `command` (literal `ReplayGameJournalRecords`), `issuer_authority` (literal above), `audience` (literal `OTERYN_GAME_GAME_JOURNAL`), `source_revision` (`u64`, Platform monotone command-outbox sequence), `operation_id` (`uuid7`), `account_id` (AccountId), `stream_generation` (`uuid7`), `from_revision` (`u64`), `through_revision` (`u64`, ≥from, range ≤10000 records), `issued_at` (`time`), `expires_at` (`time`, issued+300 seconds).

Game authorizes account/authority/generation/range, binds digest before work and persists decision. It enqueues retained original records durably before `COMMITTED` / `JOURNAL_REPLAY_QUEUED`. Terminal refusal codes with `REJECTED`: `JOURNAL_RANGE_UNAVAILABLE`, `JOURNAL_OWNER_MISMATCH`, `JOURNAL_GENERATION_MISMATCH`, `JOURNAL_INTENT_EXPIRED`, `JOURNAL_UNSUPPORTED`. Unknown/ambiguous authority denies; dependency outage is retryable, not invented owner mismatch. Receipt 200 uses the exact ingest acknowledgement. Exact receipt retry is harmless; changed binding/outcome for the same operation →409. COMMITTED is immutable even if the intent subsequently expires; an exact completed retry returns original receipt. A successful replay queue receipt never claims records have reached Platform.

Poll every second; rescan/retry ceilings follow existing consumer conventions (backoff ≤10 seconds except obey a larger Retry-After). Proposed total HTTP deadline 6 seconds including connection/read; no unbounded retry in one request. Timeout/connection loss before or after either commit is **AMBIGUOUS**: retain original identity and bytes, retry/reconcile; never assume refusal, erase backlog or mint replacement records. Expired unprocessed commands get a terminal rejection; partially queued work resumes under the same durable command decision. No terminal receipt means pending/reconciling. Exact retries always precede expiry re-evaluation. Persistent conflicts, unavailability beyond retention or malformed evidence require owner reconciliation, not manual projection fabrication.

## 8. Privacy, retention and consumer truthfulness

The authorization subject is authenticated Platform AccountId, resolved server-side. Every list, detail, export, cache and background job must enforce `record.account_id == authenticated_account_id` plus current accepted privacy and ownership-boundary evidence. Staff, support, guild and party membership confer no journal access. No shared public caches, search index or public projection receives journal rows. Deny access when account binding, character revision, transfer cutoff, retention or privacy evidence is unknown/ambiguous.

`TransferCharacterOwnership` must fence capture at the authoritative ownership commit: pre-cutoff records of that character are hidden from both accounts at the cutoff and purged under §8.2; they never follow the character to the buyer. Buyer receives only post-cutoff records under a new session. Seller access to post-cutoff records is denied. Ambiguous/in-flight transfer freezes affected reads/ingest application pending authoritative reconciliation; stale public profiles cannot prove ownership. Final acceptance must specify how character revision/cutoff evidence is obtained; without it this feature stays off.

Proposed retention: Game replay bytes 30 days from capture; Platform private records, derived personal records and rare-drop history 90 days from segment end; operational payload quarantine at most 7 days and never beyond original retention or the §8.2 purge deadline; encrypted backup expiry at most 30 additional days. When a payload leaves the active stores, by retention expiry or by deletion, only a content-free tombstone (§8.2) remains for 120 days from its purge; replay matching a tombstone is rejected rather than resurrected. No indefinite personal-record exemption. These values require owner/privacy acceptance before activation.

Never send credentials, IP/device identifiers, chat, free text, names, account email, payment data, session tokens, GameSessionId, leases, fencing secrets, internal node endpoints, coordinates/routes, item instance identities, inventory contents or private counterpart AccountId/CharacterId. Party rows are anonymous slots; player damage sources are redacted. Diagnostics contain operation/result/timing, not payloads or account lists.

T73 sharing links remain disabled: owner-only scope does not authorize bearer-link disclosure. T81 is a separate default-off derived publication: daily cohorts by place, level band and vocation, ≥10 distinct complete sessions and ≥5 accounts, delay ≥24 hours, respect account exclusion immediately with recomputation/fencing. Publish no rows, identifiers, party data or rare occurrences. Suppress small cells and differencing across filters; no arbitrary drilldowns. Counts are not consent. Cohort definitions, privacy review and invalidation proof must be accepted separately before activation. Prices/profit/exp-per-hour remain labelled Platform derivations with price provenance; zero-duration segments never divide by zero. Incomplete evidence is unavailable, not a personal best.

### 8.1 Owner data export

Platform obligation; no Game route, member or command changes. On the authenticated owner's explicit request, Platform produces one JSON document containing only that account's own G-9, G-10 and G-11 data as currently serveable to that owner under each contract, including its unavailable, stale and gap labels. It excludes other accounts' data, counterpart identities, transferred-away pre-cutoff records, quarantined payloads, tombstones, fences and diagnostics. Limits: owner-only (the §8 authorization subject, never staff or a browser-supplied AccountId); fresh authentication and CSRF protection on the request; at most one export per account per rolling 24 hours across all three contracts; private no-store delivery to that owner only; the generated file is deleted within 24 hours. Every request, refusal and delivery is audited (account, time, families, result, size), never the payload. Default-off flag `playerdata.journal.export`. Admin export, support export and analytical or aggregate export of journal data remain forbidden.

### 8.2 Deletion and tombstones

Account deletion, character deletion, journal opt-out and character transfer (for that character's pre-cutoff records) hide all affected records, derived projections and T81 contributions immediately and stop ingest use. Within 24 hours Platform purges payloads, projections, caches, quarantine and staging for that scope from active stores. Only content-free tombstones remain: id (record or segment/event business id, or the deleted scope: AccountId, CharacterId with transfer cutoff, stream key), epoch (`stream_generation`) and digest (SHA-256 of the canonical record bytes, or of the canonical scope key). Tombstones are kept 120 days from the later of deletion and purge; they are the single minimal-fence form in this contract, replacing the rev 2 "deduplication digests and fences" wording. A replay or retry that matches a tombstone, or any record inside a deleted scope, is refused and never reinserted. After tombstone expiry, Game replay retention (30 days) has long ended and a deleted or opted-out stream accepts nothing without a newly admitted generation, so replay still cannot restore deleted data. Backups expire on their own schedule (at most 30 additional days, shorter than the tombstone lifetime). Tombstones are preserved outside the state a restore rolls back, and a restore from backup re-applies them before serving or ingest resumes; if they cannot be proven, the affected data stays unavailable.

### 8.3 Legal hold

No legal hold exists by default. Only the project owner may place one. Each hold is audited with who placed it, when, scope, reason and expiry; release is audited the same way. A hold suspends purge only for its scope until its expiry or release, after which §8.2 purge completes within 24 hours. A hold does not make held data visible, exportable, replayable or usable by any projection.

## 9. Flags, compatibility and rollback

Proposed flags, all false/unconfigured by default: Game `journal.sessions.publish`, `journal.sessions.replay_commands`, `journal.sessions.party_capture`; Platform `playerdata.journal.ingest`, `playerdata.journal.replay_commands`, `playerdata.hunts.read`, `playerdata.drops.read`, `playerdata.places.aggregate`, `playerdata.hunts.share` (last remains false under v1).
Disabled routes →503; disabled reads deny. Missing bounds, identity overlap, unsafe environment or incompatible peer version count as misconfiguration. Disabling publication never deletes a durable backlog. Disabling aggregation fences existing outputs; rollback cannot restore an older privacy allow, lower a watermark or mix generations. Capacity exhaustion must be observable and fail closed for journal completeness without fabricating gameplay facts; game owner must accept capture/backpressure behavior.

Version 1 is a closed schema. Unknown members/enums/versions are rejected, not ignored. Even additive wire fields require a negotiated new version/route or accepted revised schema and exact producer/consumer compatibility evidence. Semantic changes to accounting, ownership, rarity, retention or idempotency require explicit acceptance. Pin both implementations and fixture digests before enabling; dual-version migration must deduplicate by the same business identities and cannot sum two authorities. No automatic public-projection, direct-SQL or runtime-call fallback.

## 10. Golden payloads

These are fictional fixtures. Canonical wire is compact re-encoding in the displayed member order; no trailing newline. Catalogue keys require a matching accepted fixture catalogue. Digest fields in later fixtures must be calculated from exact intent bytes, never placeholders accepted as valid digests.

Valid final solo segment (measured zero damage is represented by empty arrays):

```json
{"contract_version":1,"operation":"PublishGameJournalRecordV1","source_authority":"game.test","stream_generation":"019a5000-0000-7000-8000-000000000001","account_id":"019a5000-0000-7000-8000-000000000002","source_revision":"1","record_kind":"HUNT_SEGMENT","record":{"session_id":"019a5000-0000-7000-8000-000000000003","segment_id":"019a5000-0000-7000-8000-000000000004","segment_index":0,"character_id":"019a5000-0000-7000-8000-000000000005","world_id":"019a5000-0000-7000-8000-000000000006","channel_id":"019a5000-0000-7000-8000-000000000007","character_revision":"1","catalog_revision":"fixture.v1","started_at":"1793836800","ended_at":"1793836860","duration_seconds":"60","end_reason":"SESSION_END","final_segment":true,"experience_gained":"100","experience_lost":"0","level":20,"vocation_key":"vocation:knight","places":[{"place_key":"place:cave","duration_seconds":"60"}],"loot":[{"item_key":"item:ruby","quantity":"1"}],"supplies":[],"damage_dealt":[],"damage_taken":[],"healing_done":"0","healing_received":"0","monsters_killed":[],"rare_drop_count":1,"party":null}}
```

Valid rare drop referencing that segment (already counted in loot):

```json
{"contract_version":1,"operation":"PublishGameJournalRecordV1","source_authority":"game.test","stream_generation":"019a5000-0000-7000-8000-000000000001","account_id":"019a5000-0000-7000-8000-000000000002","source_revision":"2","record_kind":"RARE_DROP","record":{"session_id":"019a5000-0000-7000-8000-000000000003","segment_id":"019a5000-0000-7000-8000-000000000004","event_id":"019a5000-0000-7000-8000-000000000008","character_id":"019a5000-0000-7000-8000-000000000005","character_revision":"1","occurred_at":"1793836830","catalog_revision":"fixture.v1","item_key":"item:ruby","quantity":"1","monster_key":null,"place_key":"place:cave","rarity_rule_key":"rarity:rare"}}
```

Valid replay intent (Platform → Game logical direction):

```json
{"contract_version":1,"command":"ReplayGameJournalRecords","issuer_authority":"OTERYN_PLATFORM_GAME_JOURNAL","audience":"OTERYN_GAME_GAME_JOURNAL","source_revision":"1","operation_id":"019a5000-0000-7000-8000-000000000009","account_id":"019a5000-0000-7000-8000-000000000002","stream_generation":"019a5000-0000-7000-8000-000000000001","from_revision":"1","through_revision":"2","issued_at":"1793836900","expires_at":"1793837200"}
```

Valid nested party accounting fixture (not a standalone request; subject loot is one ruby):

```json
{"party_id":"019a5000-0000-7000-8000-000000000010","members":[{"member_slot":0,"is_subject":true,"active_seconds":"60"},{"member_slot":1,"is_subject":false,"active_seconds":"60"}],"loot":[{"member_slot":0,"item_key":"item:ruby","quantity":"1"}]}
```

Invalid pending request →400: numeric cursor and nil cursor inconsistency (no work accepted):

```json
{"contract_version":1,"operation":"ListPendingGameJournalCommandsV1","source_authority":"game.test","after_source_revision":1,"after_operation_id":"00000000-0000-0000-0000-000000000000","max_entries":32}
```

Invalid Damage row →400: private counterpart field is forbidden; PLAYER source must be null:

```json
{"damage_type_key":"damage:physical","source_kind":"PLAYER","source_key":"player:someone","amount":"10","character_id":"019a5000-0000-7000-8000-000000000005"}
```

Conflict fixture: resend the valid final-segment payload above with only `experience_gained` changed to `"101"` →409, original bytes preserved. A correctly shaped record for an unproven account binding is refused/held unavailable regardless of valid JSON. A duplicate exact valid record →200 with no second totals or drops.

## 11. Acceptance checklist

Game owner/producer:

- [ ] Accept names, native identity encodings, ruleset keys, character revision/cutoff evidence and hunting boundary definition.
- [ ] Implement and prove area-entry/exit, 15-minute kill-idle and durable-logout capture, crash boundary and atomic facts/outbox; no claims of complete data across lost measurements.
- [ ] Prove durable IDs, byte-stable duplicate replay, conflict refusal, contiguous account revisions and restore generation fencing.
- [ ] Prove actual damage/healing/kill/loot attribution, party privacy authorization and catalogue rarity resolution.
- [ ] Accept payload bounds, capture capacity, replay retention and unavailable-range behavior; never silently truncate arrays.
- [ ] Prove mTLS purpose isolation, account partition authorization, replay-command receipt binding, refusal/expiry and timeout recovery.
- [ ] Supply exact pinned producer fixtures and accept this proposal through coordinator; no default activation.

Platform owner/consumer:

- [ ] Prove strict canonical validation, bounds, duplicate/conflict handling and transactional ingest acknowledgement.
- [ ] Prove owner-only reads/exports/jobs/caches; deny another account and staff without authority; transfer seller/buyer boundary and stale ownership failure.
- [ ] Prove out-of-order staging, independent stored/applied watermarks, missing/poison record handling, replay, rebuild and restore recovery.
- [ ] Prove disjoint sums, rare-drop non-double-counting, party non-double-counting, zero-duration handling and incomplete-session exclusion.
- [ ] Prove retention/deletion within 24 hours, 120-day tombstones and anti-resurrection including backup restore and replay; redact diagnostics.
- [ ] Prove owner export: owner-only, fresh authentication, once per 24 hours, audited, own data only; prove legal holds are owner-only, audited and scope-limited.
- [ ] Keep T73 sharing disabled; qualify T81 thresholds, delay, exclusion, differencing and restrictive privacy invalidation before separate activation.
- [ ] Keep flags off; pin producer/consumer versions and golden canonical byte digests, exercise timeout-before/after-commit and exact retry on both sides.

Joint completion requires accepted contract revisions and exact-version integration evidence. Documentation checks alone close neither G-9 nor implementation tasks T72/T73/T75/T81.

## 12. Open questions for the game owner (coordinator)

1. How will the new durable logout event prove entry-to-logout boundaries across reconnect and crash/recovery, and how is active hunting time measured within the agreed area-entry/exit and 15-minute kill-idle segments?
2. How will character command receipts (§4.3 of the character authority contract) supply authoritative character_revision and the exact `TransferCharacterOwnership` cutoff? How are in-flight records fenced during transfer and restore?
3. Which catalogue revision and key namespaces cover items, places, vocation, damage types, monsters and rarity rules? Can attribution resolve every required metric without guesses?
4. Can the NEW journal durability/outbox implementation provide contiguous per-account revisions, immutable replay bytes and generation fencing with these size/rate bounds? What baseline/reconciliation evidence is available after a restore or expired replay range?
5. Does the game owner accept anonymous party slots and per-slot loot disclosure to each party participant? What accepted policy would allow named settlement or sharing later?
6. Are 30-day producer replay, 90-day private history, 120-day minimal fences and backup/purge deadlines acceptable? What opt-out/erasure capture policy and legal requirements apply?
7. Are proposed replay-command-only control, certificate purpose, transport bounds, deadlines, rate limits and refusal codes implementable by the NEW journal pull consumer? What capacity/backpressure behavior is safe for gameplay?
8. Who signs off producer fixtures and exact-version joint qualification, and what further privacy acceptance is required for T81? Until answered and accepted, all journal features remain off.

Revision 3 note: owner decisions 1a–4a bear on questions 2 (transfer now purges pre-cutoff records), 4 (restore re-applies tombstones) and 6 (deletion, tombstone and legal-hold policy). They remain open until the coordinator accepts revision 3.
