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

Serve schedules read/verification/refresh off startup and readiness. A matching
stored publication (all source fences plus independent SHA-256) is ready on the
first database lookup without refetching the producer. Otherwise the process
keeps catalogue reads closed while trying the explicitly configured source.
File reads and CPU verification run on blocking workers; network I/O is async.
No network fetch of Game source or raw `content/**` parsing is performed.

Only transient `Unavailable` failures retry. Configuration, schema, provenance,
integrity, G-2 and publication-key collision refusals stop closed immediately.
The retry policy is conservative and bounded:

| Setting | Default | Valid range |
| --- | --- | --- |
| `OTERYN_WORLD_CATALOG_RETRY_INITIAL_SECONDS` | 5 | 1–300 |
| `OTERYN_WORLD_CATALOG_RETRY_MAX_SECONDS` | 300 | initial–3600 |
| `OTERYN_WORLD_CATALOG_RETRY_MAX_ATTEMPTS` | 12 | 1–64, including the first immediate attempt |

Each failed attempt sleeps 75–100% of the exponentially increasing interval,
with independent jitter even at the cap. Each whole attempt has a 60-second
deadline; the URL transport retains its tighter 30-second deadline. Exhausting
the configured budget leaves the source closed and reports failure; operators
can retry by restarting after correcting the outage/configuration. Stored matching
publications require no producer availability and remain independently gated by
`world_visibility`. Startup/listening sockets and `/readyz` never await retries.

Verification checks canonical JSON, byte SHA-256, semantic snapshot identity,
provenance fences, support/completeness, keys/endpoints and resource bounds before
transactionally storing the immutable artifact and derived carrier cache.

`OTERYN_WORLD_CATALOG_PUBLICATION_ENABLED=true` additionally enables production
wiki publication; it defaults **off**. Only `supported + complete` categories
are projected from verified cache carriers. Unsupported, partial or unknown
categories return 503. A category missing required wiki names (or quest
classification) also refuses; no name is inferred from a native key. Wiki
home/search aggregate the complete, presentable categories; none
available means 503. A complete empty category may return an empty 200.

Publication atomically replaces a dedicated wiki pointer and rebuilds derived
`world_entities`. It does not switch the legacy/map pointer, build atlas tiles,
or insert/change `world_visibility` approvals. Refresh is idempotent per snapshot.
The existing `/admin/world` staff action independently approves or denies each
native key under the same RBAC, MFA, CSRF and transactional audit controls. Native
versions cannot be published through the legacy/map action or given Platform-authored
replacement names. Source changes/removals never inherit public facts from an old
snapshot; approvals alone cannot publish missing entries.

G-2 is resolved at Game `main` `669d5af5261a60d347167b7c880a4498aebe72ed` (merged #3025):
the producer omits hidden records, hidden NPC shop relations and secret achievements.
No visibility decoding/default is needed. If any record/payload carries a
`visibility` member, only the string `public` is accepted; anything else rejects
the whole artifact. Absence is the ordinary export form. Platform approval remains
an additional gate.

The CATALOG lane is merged. Its notes still explicitly keep all eight native
capabilities unsupported/unknown, so the real `main` export still cannot
publish wiki data. `/wiki/areas` remains 503 pending the later additive `area`
category. Atlas needs a Game-side atlas export; Today needs an accepted producer.
See [wiki capability requirements](CATALOG_WIKI_REQUIREMENTS.md) and the pinned
[SOURCE note](tests/fixtures/catalog-669d5af5/SOURCE.md).

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
