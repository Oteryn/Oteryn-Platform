# Legacy cutover import

```
oteryn-portal import identity --source-url <mysql-url>
oteryn-portal import all --source-url <mysql-url> --apply
```

The destination is `DATABASE_URL`. Migrate it before invoking the importer. The
source must be a quiesced cutover copy, accessed with a SELECT-only database user.
The adapter additionally opens a repeatable-read, consistent-snapshot, READ ONLY
transaction, uses UTC, and never executes source mutations. Do not pass production
URLs to tests. A source URL on a command line can appear in process listings; use
an isolated operator session. Driver statement logging is disabled on both connections.
Diagnostics contain only compiled table names, row ordinals, counts and fixed
refusal codes, never source identifiers, values, credentials or database errors.

Without `--apply`, PostgreSQL is also in a READ ONLY transaction. The importer
checks mappings, source invariants, destination types, NOT NULL and CHECK
constraints. It writes neither records nor import markers. Uniqueness and foreign
key constraints are additionally enforced by PostgreSQL on apply; a collision
rolls back the entire area. A successful dry run alone is not cutover approval.

Apply commits one transaction per area, in this order for `all`: identity,
content, commerce, support, characters. If one area refuses, previous successful
areas remain committed and later areas are not attempted. Import identity first
when selecting areas individually. Import map identifiers resolve cross-area
references. A PostgreSQL advisory transaction lock serializes all apply operations.
Committed source keys are skipped on replay, including timestamps and ledger
entries. Keep the map permanently. This is a one-source, one-time cutover tool,
not synchronization: do not reuse a destination/map for another source database.
Do not change source rows after applying an area. No upserts overwrite live data.

Commerce apply requires `PORTAL_IMPORT_COMMERCE_ENABLED=true`; characters apply
requires `PORTAL_IMPORT_CHARACTERS_ENABLED=true`. Both default to disabled. Dry
runs can inspect those areas without enabling writes. These flags do not enable
live money or game features. No game commands, deliveries, payments, notifications
or session tokens are created by the importer.

## Credentials and identity

Canonical `identities.account_id` values become account UUIDs; integer identity
IDs are only map keys. Argon2id password hashes are copied byte for byte. The
current destination verifier only implements Argon2id despite the planned bcrypt
compatibility: bcrypt source rows refuse until the login crate supports them.
The importer never converts or resets passwords. Active legacy identities have
no email-verification state in Laravel; their existing creation timestamp is used
as the grandfathered verification timestamp required by Rust login. An explicit
source verification timestamp takes precedence if present. Laravel
AES-256-CBC TOTP envelopes are authenticated before decryption using
`PORTAL_IMPORT_LARAVEL_KEY` (raw 32 bytes or Laravel's `base64:` form).
`OTERYN_MFA_KEY` is the existing destination MFA encryption key (64 hex digits).
The TOTP seed is retained, decoded from Base32, and re-encrypted with the existing
Rust AES-256-GCM format and account UUID as associated data. Recovery-code hashes
are decrypted from the Laravel array and copied only when already compatible with
the target's 32-byte SHA-256 representation, without rehashing. The repository's
Laravel generator actually uses password hashes for recovery codes, which cannot
be converted without the original plaintext. Such arrays refuse identity import;
the destination needs legacy-hash verification/storage support outside this scope.
The last-used
TOTP timestep is preserved. Encryption keys are never stored in the import map.
Missing keys or malformed envelopes refuse the whole identity area.

Disabled and terminated accounts remain suspended and closed respectively.
Privacy booleans and native security generations are retained. Scheduled deletion
uses its original request/scheduled timestamps and destination grace constraints.
Existing role keys can be reused only when unassigned; their permissions are
replaced with exactly the source permission set in the same transaction. Existing
assigned roles refuse import, avoiding silent privilege changes. Unknown
permissions refuse through the destination allowlist.

**Current destination gaps:** `identity_recovery_keys` has no Rust destination
table. Any such row refuses the complete identity area, even if already used or
revoked: no key is dropped or silently invalidated. Email-change cooldowns also
lack a compatible target and refuse identity import. Schema work outside this
task's allowed paths is required before importing datasets containing these states.
Passkeys have a Rust destination but no source table in the Laravel migrations.

## Content and files

News, managed pages, events and announcements retain English content and Polish
translations. Wiki articles become guides, with immutable revision history and
current translations. Incompatible locales, versions, missing authors and
publication states refuse rather than inventing attribution. Legal page version
history has no source table; import does not invent past legal acceptances.

Media metadata alone is insufficient: set `PORTAL_IMPORT_MEDIA_ROOT` to a local
copy of the editorial-media disk. Paths must remain inside that root after symlink
resolution. Files are bounded, decoded through the existing image validator, and
checked against source sizes, dimensions and SHA-256. Original and thumbnail bytes
are preserved. Missing files or mismatches refuse the content area.

## Commerce and support

Any auction not in completed/cancelled/expired state refuses commerce, including
escrow, settlement, cancellation and recovery states. Reserved balances must be
zero, balances nonnegative and ledger sums equal materialized balances. Wallet
history becomes immutable adjustment operations and entries preserving every
signed available/reserved delta and timestamp. No credit is minted by importing a
payment. Source payment order/attempt must map unambiguously to one provider
reference; unprocessed/unknown provider events refuse. Premium intervals,
request/entitlement UUIDs and revisions are preserved, with deltas derived from
successive interval durations so the Rust ledger sum remains valid.

Tickets, public/internal messages, player reports, sanctions and their embedded
appeal workflows are imported. Submission request keys are replaced by mapped
UUIDs because old client retries are not continued after cutover. Deleted message
authors cannot satisfy the target foreign key and refuse. Permanent active
sanctions lack the target's required independent approval evidence and refuse;
the importer never fabricates an approval. Account suspensions and sanction
history are separate controls and both retain their source state.

Profile preferences import guild visibility, main-character selection and native
character UUIDs, including unbound historical guild opt-outs. The Rust profile
table has no comment or other visibility switches yet. Nonempty comments or
nondefault unsupported preferences refuse the characters area rather than expose
previously hidden data. Full profile migration needs the destination schema to
support those fields first.

## Tests

`tests/legacy.json` is an entirely synthetic dataset. The in-process `Source`
fixture uses the same planning/apply engine without MySQL. PostgreSQL tests read
`PORTAL_TEST_DATABASE_URL`; when absent they print a skip notice. Each test creates
and drops its own schema and runs the embedded migrations. Tests cover all areas,
read-only dry runs, idempotence, partial-write rollback, malformed input, missing
maps, open auctions, balance disagreements and unsupported credential/privacy data.

## Tables intentionally not imported

The following source tables have no compatible durable destination, or are
explicitly transient/retired. They are not silently represented as successful
business imports. Tables outside the five selected areas are not queried.

| Source table | Reason |
| --- | --- |
| `admin_audit_events` | No compatible target audit payload/actor contract; historical audit remains in source archive. |
| `character_auction_bids` | Retired marketplace history; new marketplace starts empty after the auction refusal guard. |
| `character_auction_transfers` | Retired marketplace history; new marketplace starts empty after the auction refusal guard. |
| `character_auction_watches` | Retired marketplace history; new marketplace starts empty after the auction refusal guard. |
| `character_bootstrap_intent_authority` | Runtime evidence/projections/commands must be republished through authenticated native contracts; no safe cutover write mapping. |
| `character_bootstrap_intents` | Runtime evidence/projections/commands must be republished through authenticated native contracts; no safe cutover write mapping. |
| `character_command_intents` | Runtime evidence/projections/commands must be republished through authenticated native contracts; no safe cutover write mapping. |
| `character_command_receipts` | Runtime evidence/projections/commands must be republished through authenticated native contracts; no safe cutover write mapping. |
| `character_restore_locks` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `character_restore_notices` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `character_restore_reopens` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `client_release_artifacts` | Distribution signing and release metadata require their own trusted release pipeline. |
| `client_releases` | Distribution signing and release metadata require their own trusted release pipeline. |
| `client_update_generations` | Distribution signing and release metadata require their own trusted release pipeline. |
| `client_update_policies` | Distribution signing and release metadata require their own trusted release pipeline. |
| `game_catalog_audit_events` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `game_catalog_creature_snapshots` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `game_catalog_entities` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `game_catalog_entity_identifiers` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `game_catalog_entity_snapshots` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `game_catalog_entity_translations` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `game_catalog_import_runs` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `game_catalog_item_snapshots` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `game_catalog_loot_snapshots` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `game_catalog_profile_entities` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `game_catalog_profile_overrides` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `game_catalog_profile_relations` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `game_catalog_profiles` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `game_catalog_relation_snapshots` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `game_catalog_releases` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `game_catalog_snapshots` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `game_catalog_validation_findings` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `game_catalog_wiki_links` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `game_channels` | Runtime evidence/projections/commands must be republished through authenticated native contracts; no safe cutover write mapping. |
| `game_login_tickets` | Transient authentication/session state is intentionally invalidated at cutover. |
| `game_public_projection_snapshots` | Runtime evidence/projections/commands must be republished through authenticated native contracts; no safe cutover write mapping. |
| `game_public_projection_state` | Runtime evidence/projections/commands must be republished through authenticated native contracts; no safe cutover write mapping. |
| `game_public_projection_watermarks` | Runtime evidence/projections/commands must be republished through authenticated native contracts; no safe cutover write mapping. |
| `game_world_protocol_candidates` | Runtime evidence/projections/commands must be republished through authenticated native contracts; no safe cutover write mapping. |
| `game_worlds` | Runtime evidence/projections/commands must be republished through authenticated native contracts; no safe cutover write mapping. |
| `homepage_template_settings` | Retired homepage templates; new Rust homepage owns presentation. |
| Pre-native account link table | Retired account link; canonical UUIDs are imported directly from identities. |
| `identity_email_change_requests` | No compatible table in the current Rust schema; retain source archive for separate migration. |
| `identity_security_events` | No compatible legacy event vocabulary; do not fabricate Rust authentication events. |
| `identity_web_sessions` | Transient authentication/session state is intentionally invalidated at cutover. |
| `managed_page_legal_versions` | No compatible table in the current Rust schema; retain source archive for separate migration. |
| `native_account_character_projection_state` | Runtime evidence/projections/commands must be republished through authenticated native contracts; no safe cutover write mapping. |
| `native_account_character_rows` | Runtime evidence/projections/commands must be republished through authenticated native contracts; no safe cutover write mapping. |
| `native_account_character_snapshots` | Runtime evidence/projections/commands must be republished through authenticated native contracts; no safe cutover write mapping. |
| `native_admission_attempts` | Runtime evidence/projections/commands must be republished through authenticated native contracts; no safe cutover write mapping. |
| `native_assignment_epoch_locks` | Runtime evidence/projections/commands must be republished through authenticated native contracts; no safe cutover write mapping. |
| `native_device_session_credentials` | Transient authentication/session state is intentionally invalidated at cutover. |
| `native_device_session_families` | Transient authentication/session state is intentionally invalidated at cutover. |
| `native_game_evidence_observations` | Runtime evidence/projections/commands must be republished through authenticated native contracts; no safe cutover write mapping. |
| `native_game_evidence_witness_stores` | Runtime evidence/projections/commands must be republished through authenticated native contracts; no safe cutover write mapping. |
| `native_game_signing_trust_key_versions` | Runtime evidence/projections/commands must be republished through authenticated native contracts; no safe cutover write mapping. |
| `native_game_signing_trust_profiles` | Runtime evidence/projections/commands must be republished through authenticated native contracts; no safe cutover write mapping. |
| `native_runtime_status_reports` | Runtime evidence/projections/commands must be republished through authenticated native contracts; no safe cutover write mapping. |
| `native_scope_assignments` | Runtime evidence/projections/commands must be republished through authenticated native contracts; no safe cutover write mapping. |
| `oauth_access_tokens` | Transient authentication/session state is intentionally invalidated at cutover. |
| `oauth_auth_codes` | Transient authentication/session state is intentionally invalidated at cutover. |
| `oauth_clients` | Transient authentication/session state is intentionally invalidated at cutover. |
| `oauth_device_codes` | Transient authentication/session state is intentionally invalidated at cutover. |
| `oauth_refresh_tokens` | Transient authentication/session state is intentionally invalidated at cutover. |
| `password_reset_tokens` | Transient authentication/session state is intentionally invalidated at cutover. |
| `payment_order_transitions` | No destination transition-history table; intent state and supported provider events are imported. |
| `payment_reconciliation_entries` | No compatible reconciliation-case table; resolve cases before cutover. |
| `payment_reconciliation_resolutions` | No compatible reconciliation-case history table. |
| `player_companion_session_analyses` | Transient authentication/session state is intentionally invalidated at cutover. |
| `product_catalog_presentations` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `product_catalog_products` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `product_catalog_versions` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `store_active_catalog` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `store_fulfilment_units` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `store_offers` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `store_operations` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `store_order_lines` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `store_orders` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `store_price_list_imports` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `store_purchase_requests` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `store_receipts` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `store_reconcile_states` | New versioned Rust catalog and delivery contracts require their own import/reconciliation path. |
| `support_notification_deliveries` | Delivery bookkeeping is not replayed across cutover. |
| `wiki_article_category` | No destination category taxonomy. |
| `wiki_categories` | No destination category taxonomy; guide text is retained separately. |
| `wiki_category_translations` | No destination category taxonomy. |

The recovery-key table is counted and explicitly refused as described above.
Category tables are listed here because the Rust wiki has no category schema.
