# World data store

## Native catalog source (current)

The production binary reads only the locked CATALOG_EXPORT_V1 offline artifact,
with an independently configured SHA-256, into `world_catalog_candidates` and its derived
`world_catalog_records` capability cache. It does **not** read Game's raw
`content/**` layout. The older raw-directory APIs/configuration below remain
compatibility test infrastructure, with no binary caller.

Configure the administrator-selected source and exact provenance together:

- Exactly one of `OTERYN_WORLD_CATALOG_FILE` (read-only local artifact) or
  `OTERYN_WORLD_CATALOG_URL` (HTTPS; HTTP is restricted to loopback);
- `OTERYN_WORLD_CATALOG_SHA256`: independently supplied lowercase 64-hex digest
  of the exact artifact bytes, including final LF;
- `OTERYN_WORLD_CATALOG_SOURCE_REVISION`: exact source revision inside the export;
- `OTERYN_WORLD_CATALOG_AUTHORITY_EPOCH`;
- `OTERYN_WORLD_CATALOG_RULESET_ID`;
- `OTERYN_WORLD_CATALOG_PROFILE_ID`;
- `OTERYN_WORLD_CATALOG_SNAPSHOT_ID`: exact `sha256:` snapshot identity.

Boot performs a bounded read of the configured file or URL. It never reads raw
Game content, rewrites an artifact, follows redirects, uses ambient HTTP credentials
or bypasses TLS verification. URL reads have a 30-second deadline and the same
256 MiB body bound as files. Both the configured transport hash and semantic digest,
canonical encoding, all
provenance fences, manifests, identities, endpoints and tombstones are verified
before a transaction stores an immutable, idempotent **inactive** candidate.
Missing/partial configuration, mismatched fences or changed bytes refuse intake.
Diagnostics contain only stable failure categories. The database cache is derived,
rebuildable evidence and confers no native authority.

Boot is chosen because it deterministically reads the administrator-selected exact
candidate when a process starts; there is no new mutation endpoint or CLI. It
writes only `world_catalog_candidates` and `world_catalog_records`, never
`world_active_version`, public
entities, visibility approvals or audit/account tables. An intake failure leaves
public serving cold. Existing admin `world_visibility` approval remains mandatory
for public reads and cannot activate an offline candidate.

CATALOG_EXPORT_V1 is accepted. The Game native adapter is not merged at the
current evidence pin; the pinned transport still permits only inactive intake. All real reference
fixture capabilities are unsupported/unknown. Even a synthetically supported
carrier cannot publish: v1 has no accepted capability payload revision, current
epoch proof or authenticated production publication profile. No activation switch
is exposed. A separately accepted Game publication/payload successor is required
before projecting candidates into public `world_entities`. See the exact
[contract snapshot](tests/fixtures/catalog-d34807f5/CONTRACT.md) sections 11–15 and
[pinned provenance](tests/fixtures/catalog-d34807f5/SOURCE.md).

Supported entities, relations and tombstones are materialized in the same
transaction as candidate intake. Cache rebuilds are atomic and repeatable.
`Candidate::require_complete` and `cached_entities` refuse unsupported, partial
or unknown inventory; cached reads also reject missing or corrupted records.
These generic carrier reads are not native publication authorization.

See [the exact wiki capability requirements](CATALOG_WIKI_REQUIREMENTS.md) and
the test-only synthetic supported-cache-to-HTTP-200 proof.

## Historical directory-parser API

T14 provides a trusted Rust service API, with no HTTP routes or UI. Construct
`WorldStore` with a PostgreSQL pool and `WorldOptions`; its default options disable
every operation. Callers must authorize import, rollback, report access and
allow/deny operations before invoking this service. Apply the workspace migrations
before use.

`import_directory(root, full_commit_hash, mappings)` accepts an immutable local
source directory. It performs no fetching or Git operations. The caller verifies
the directory's provenance against the supplied full lowercase SHA-1/SHA-256 hash.
`import_configured_directory` additionally accepts explicit exact file exclusions
and local schema URI aliases. Configuration is deserializable JSON (`ImportConfig`).
Changing the mapping or a visibility veto is a trusted configuration change.

Each mapping specifies a root-relative content glob, schema file, entity kind,
key field (or nested JSON pointer), records-array pointer and schema scope:

- `document`: validate the entire file, including its envelope;
- `records`: validate every record with a source schema defined for individual
  records; missing/non-array envelopes still fail extraction.

Every JSON file under `content/` must match exactly one mapping or an explicit
exact exclusion. Overlapping mappings, mapped/excluded conflicts, unmatched
mappings, empty imports, duplicate kind/key pairs and invalid keys fail closed.
Non-JSON assets are outside this importer. Files are limited to 16 MiB and imports
to 100,000 records. Content symlinks are refused. Schemas and their references must
resolve within the supplied `tools/content-schema/` directory. URI aliases map
source schema identifiers to local schema files; network retrieval is disabled.

Validation stops at the first failure and reports the source file, RFC 6901 JSON
pointer and diagnostic message. Failed attempts retain a UUIDv7 version id and
persisted diagnostics. Invalid commit hashes and disabled calls are rejected
before creating an attempt. An interrupted process can leave an `importing`
attempt; it cannot expose partially published documents. Such attempts can be
inspected with `import_report`; a fresh import creates a new attempt.

Documents, ready status, active pointer and previous pointer are committed in one
transaction under the permanent singleton row lock. Rollback uses that same lock
and swaps the pointers. Older versions are retained; this task adds no pruning.
Database publication failures roll back documents and pointers and record a failed
attempt separately. Import reports describe validation/publication status, while
`active_version_id` identifies the current version.

`get_entity` and keyset-paginated `list_by_kind` join the active version and
`world_visibility` in one statement. Missing allow-list rows mean hidden. Allow
and deny decisions are per kind/key and survive switches. A mapping's
`hidden_when_true` boolean pointers veto publication even if an entity is allowed;
the pinned Game configuration uses `/secret` for achievements. Reader APIs never
expose arbitrary historical versions. Authoritative raw documents remain in the
database for trusted tooling.

## Historical pinned raw-layout reference (not a native source)

`config/game-857f15f3.json` describes the owner-authorized read-only reference
`Oteryn/Oteryn-Game@857f15f376907c592bb28d854fccff18a15cdf87`. It imports
24,955 records from achievements, terrain, world objects, areas (cities, regions,
hunting places and islands), houses, transitions and worlds. It validates against
the unmodified JSON schemas in that same tree, including local URI references.
All other JSON paths are individually excluded, so newly added content requires
an explicit mapping/exclusion decision.

This is a bounded configuration for matching schemas, not a claim that every
Game family has a usable shard schema. In particular, the Item, Creature, NPC,
spell and other authoring schemas describe different inputs from the materialized
shards. The Quest tree schema references
`oteryn:schema/quest-completion-donor-extension/v1`, which the Game Python validator
constructs dynamically rather than supplying as a JSON schema file. These families,
indexes, source evidence, runtime sidecars and assets are explicitly outside this
reference configuration. No replacement schemas or game values are invented.
The generic importer can accept new mappings when matching source schemas exist.

Small unmodified Game fixtures under `tests/fixtures/game/` include `SOURCE.md`
with repository, full commit and original paths. Other fixtures are explicitly
synthetic. The small copied shard is validated even without PostgreSQL.

## Tests

Run the workspace acceptance commands from `portal-rs/`:

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

Database tests read `PORTAL_TEST_DATABASE_URL`. When it is unset, each prints a
skip notice (`cargo test -- --nocapture` shows notices). When set, a connection or
migration failure fails the test. Use a disposable PostgreSQL **18.6** database:
each test creates a unique schema, applies the real workspace migrations and drops
the schema on success. Failed tests can leave diagnostic schemas behind.

The real-tree database test uses `PORTAL_TEST_GAME_DIR` (default
`/tmp/oteryn-game`) and skips with a notice if the clone is unavailable. It verifies
the exact commit and clean content/schema paths, imports the configured surface
from the complete tree, and checks that a secret achievement stays hidden even
after an allow decision. It never fetches or changes the reference repository.
