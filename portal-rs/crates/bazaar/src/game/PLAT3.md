# PLAT3-BAZAAR consumer candidate

Governing read-only contract: Oteryn/Oteryn-Game commit
`3b49a20feec3ba29700bb4b2fa28fa9b539b79a2`,
`docs/contracts/OTERYN_GAME_BAZAAR_AUTHORITY_V1.md`, CANDIDATE revision 1,
Game PR #3010. Base Platform commit:
`93f0aec532df11bd575ac3d2944a3b5587e3d585`.
This implementation does not accept or activate the candidate contract.
Shared registry allocations are unchanged: Bazaar decision event type **6**,
schema 1; types 7/10/11 belong to other PLAT3 consumers. The cross-repository
lock remains pending; this task does not invent a merged canonical revision.

## Delivered boundaries

- Exact ordered JSON codec for all nine intents and v2 receipts. SHA-256 covers
  original retained intent/receipt bytes. Rejected receipts have null result data;
  unknown result codes, unknown members, duplicates, escapes, noncanonical scalar
  encodings, wrong bounds and cross-operation echoes deny ingestion.
- Lock snapshots use the existing exact-byte SHA-256/Ed25519 verifier with separate
  provisioned current/retiring keys. Fixture trust is test-only. Keys are checked
  again wherever current signed evidence becomes usable; optional unsupported
  snapshot sections remain unavailable.
- Receipt ingestion is an extension of the existing Character receipt facility,
  with no Bazaar receipt URL. `extend_receipt_facility` preserves the supplied v1
  router and selects the v2 handler before its larger body bound. The six Bazaar
  routes require their separate purpose identity, bounded bodies, rate limiting,
  exact route-operation matching and no-store responses.
- Same-key exact receipt retries are acknowledged without reapplying effects;
  changed receipts retain the first record and durable conflict digest. Receipt,
  local terminal record and audit ingestion all use caller-owned transactions and
  the issuer lock. Errors must roll back; quarantine results must commit.
- `BazaarAuthorityDecisionV1` uses the pinned protobuf field tags. Intake validates
  the shared EventEnvelope, type/schema, durability/privacy/retention, transaction
  reference, identity, exact payload hash and retained intent/receipt hashes.
  Event retries compare immutable semantic envelope fields and exact payload bytes;
  raw protobuf field serialization order is not treated as canonical.
- Real `PlatformAccounts`, `CoinReservations`, `Evidence` and `ItemAuthority`
  adapters use transaction-local Platform data only. Absolute coin holds share
  the Wallet ledger, sorted account locks, replay protection and the caller's
  transaction. Settlement fences funding; completion requires delivery evidence
  before debit/credit, fee and excess release. There is no second coin balance.
- Item evidence binds current status, receipt, audit, listing, seller, world,
  exact asset/quantity/definition/attributes, custody revision, recovery floor,
  policy and watermark freshness. Status or an HTTP acknowledgement cannot pay.
  Full resync requires every retained listing and configured policy world plus
  prerequisite receipts; it cannot treat missing subjects as cancellation.
- The additive migration is exactly
  `20261106090001_plat3_bazaar_authority.sql`. Immutable evidence is retained;
  rollback means stop new dispatch and reconcile outstanding custody/funding.
  There is no destructive down migration or automatic evidence deletion.

## Configuration and remaining fail-closed seams

`AuthorityConfig::default()` disables every owner gate and has no accepted contract
hash. The exact pinned contract SHA-256 must be explicitly supplied. A configured authority
also needs a positive externally supplied `RecoveryFence` matching the durable
issuer floor. The restore coordinator must obtain that fence from its nonrollback
source; this module never bootstraps it from a receipt or database restore.
`Outbox::Settings` and its existing safe default wiring retain their preparation
semantics; setting those two old booleans cannot activate this candidate.

Explicit gates cover v2 acceptance, eligibility/escrow allocation, Inbox/item
coverage, gold fee/headroom policy, common recovery scope, and signing/retention.
They do not stand in for actual missing evidence. In particular:

- Existing private-listener and browser composition still default to unavailable.
  The new routers/adapters are exported for qualified composition, not enabled in
  the portal binary. Existing browser routers retain CSRF, sessions, rate limits,
  localized pl/en unavailable responses and public projection allowlists.
- Character binding/ownership-transfer dispatch and final-owner unlock remain
  pending: the existing CC v1 API has no qualified not-yet-listable transfer seam
  tied to this bind facility. A bind receipt or signed snapshot cannot substitute
  for the committed v1 ownership leg. Character auctions remain unavailable.
- Gold release/payout and gold-funded handoff remain pending until durable bank
  disposition/headroom and delivery/payout proofs are integrated. The old core's
  floor-rounded gold commission is never used for this candidate's ceil fee.
- Want-to-buy seller deposits remain unavailable until each offer has its own
  authoritative deposit binding. A sell listing's escrow cannot prove that offer.
- Terminal cancellation/expiry now requires return evidence. Item return requests
  remain pending and replay the same immutable local issuance key; a pending
  return hides the listing from the qualified item adapter.
- Competing delivery/return dispatch is denied after the first executable key.
  An expiry, timeout, changed recovery floor or terminal refusal never silently
  frees funding or permits compensation. Automatic retry-after-refusal and safe
  compensation require the no-late-commit orchestration seam.
- Quarantined subjects/operations stay unavailable. Operator repair and external
  trust revocation must suspend composition; there is no automatic unquarantine,
  inferred policy acceptance, finite retention choice or production enablement.
- `PlatformAccounts.session_hash` must be bound to the actual current browser
  session for listing mutations. Missing session evidence denies sudo, even if
  another session for that account has recently authenticated.

Thus this is a tested consumer foundation and item/coin evidence path, **not full
PLAT3-BAZAAR completion or rollout qualification**. Remaining orchestration and
binary/browser wiring are explicit undelivered scope, in addition to owner gates.

## Evidence and tests

Byte-exact Game fixtures, source paths/full SHA and exact-file hashes are under
`../../tests/fixtures/plat3/`. Every vector's exact wire hash is recomputed;
positive/tampered/rehash-without-resign Ed25519 tests use its synthetic public key.
Tests cover all nine intent/result schemas, disabled private routes, preservation
of the v1 receipt handler, concurrent duplicate publication, conflict quarantine,
transaction rollback, missing audit/status/receipt evidence, nonrollback floors,
and receipt-to-item-delivery-to-Wallet completion with exact replay.

PostgreSQL tests use `PORTAL_TEST_DATABASE_URL`; they print a skip when absent.
Validation must set it to a disposable PostgreSQL 18 database. No Game SQL, Game
writes, network calls inside a Platform transaction, production changes or workflow
changes are part of this work.

## Validation and delivery record

Branch: `codex/portal-rs-plat3-bazaar`. No PR creation, merge, Game writes,
workflow changes or production enablement are included.

Required validation uses the existing native Rust toolchain and PostgreSQL 18.6
harness, with `PORTAL_TEST_DATABASE_URL` set to a task-owned disposable database.
`RUST_TEST_THREADS=1` avoids exhausting the supplied test server's lock table;
no PostgreSQL settings or existing database data are changed. The required
commands are `cargo fmt --all --check`,
`cargo clippy -q --workspace --all-targets -- -D warnings`, and
`cargo test -q --workspace`. Exact final counts/head are reported with delivery.
The focused Bazaar suite has 91 passing tests, including ten PLAT3 conformance
and PostgreSQL flow tests plus two new cancellation regressions. Three unrelated
workspace browser tests remain pre-existing ignored tests requiring browser setup.

Changed files (repository relative):

- `portal-rs/Cargo.lock`
- `portal-rs/crates/bazaar/Cargo.toml`
- `portal-rs/crates/bazaar/src/game/PLAT3.md`
- `portal-rs/crates/bazaar/src/game/README.md`
- `portal-rs/crates/bazaar/src/game/authority.rs`
- `portal-rs/crates/bazaar/src/game/authority_evidence.rs`
- `portal-rs/crates/bazaar/src/game/authority_exchange.rs`
- `portal-rs/crates/bazaar/src/game/authority_http.rs`
- `portal-rs/crates/bazaar/src/game/authority_items.rs`
- `portal-rs/crates/bazaar/src/game/authority_tests.rs`
- `portal-rs/crates/bazaar/src/game/authority_wire.rs`
- `portal-rs/crates/bazaar/src/game/coin_ports.rs`
- `portal-rs/crates/bazaar/src/game/event_wire.rs`
- `portal-rs/crates/bazaar/src/game/game_store.rs`
- `portal-rs/crates/bazaar/src/game/game_wire.rs`
- `portal-rs/crates/bazaar/src/game/mod.rs`
- `portal-rs/crates/bazaar/src/items.rs`
- `portal-rs/crates/bazaar/src/items/service.rs`
- `portal-rs/crates/bazaar/src/items/tests.rs`
- `portal-rs/crates/bazaar/src/ports.rs`
- `portal-rs/crates/bazaar/src/repository.rs`
- `portal-rs/crates/bazaar/tests/fixtures/plat3/README.md`
- `portal-rs/crates/bazaar/tests/fixtures/plat3/SOURCE.md`
- `portal-rs/crates/bazaar/tests/fixtures/plat3/decisions.json`
- `portal-rs/crates/bazaar/tests/fixtures/plat3/foundation.proto`
- `portal-rs/crates/bazaar/tests/fixtures/plat3/plat3_authority.proto`
- `portal-rs/crates/bazaar/tests/postgres.rs`
- `portal-rs/crates/bazaar/tests/support/mod.rs`
- `portal-rs/migrations/20261106090001_plat3_bazaar_authority.sql`
