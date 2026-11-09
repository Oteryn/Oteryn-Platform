# Downloads core

`Downloads::publish` accepts an actor UUID, a `PublishAccess` adapter and an
`Upload` containing exact public metadata/policy bytes. The caller supplies
channel-specific pinned public Root metadata through `Downloads::new`.
No private keys, package upload, network fetch, signing or staff HTTP mutation
exists in this crate. The operator adapter reads account status and RBAC.
An account must be positively active and its channel publication permission must be `Some(true)`.

All operations require `FeatureFlags { downloads: true }`. The binary mounts
GET routes with the flag off unless `OTERYN_DOWNLOADS_ENABLED=true` is set.
The `downloads-publish` operator command supplies channel public Roots from
`OTERYN_DOWNLOADS_ROOT_STABLE` / `OTERYN_DOWNLOADS_ROOT_BETA` file paths, requires
an active actor with `downloads.manage`, and calls the same Rust API. Setting
the HTTP flag does not configure publication trust anchors. See the accepted
[origin contract](../../../../docs/contracts/OTERYN_V2_CLIENT_UPDATE_ORIGIN_CONTRACT.md).

## Bounded TUF upload profile

Verification uses the locked `tough` library without its HTTP feature. Every
uploaded signature envelope is additionally checked for unique decoded key IDs
before verification, including delegated roles; repeated signatures cannot count
toward any threshold. No HTTP feature is enabled.
Verification uses an in-memory transport that can only read uploaded
files. Each channel has a separate repository under `/updates/{stable|beta}`:

- Root, Targets, Snapshot and Timestamp roles with valid signatures and expiry;
- consistent snapshots; metadata names `N.root.json`, `N.targets.json`,
  `N.snapshot.json`, `N.stable.json` or `N.beta.json`, and `timestamp.json`;
- one terminating delegation named for the channel, restricted to
  `channels/<channel>/*`, with no top-level targets or nested delegations;
- exactly the policy target `channels/<channel>/policy-v1.json` and the policy's
  artifact target descriptors in that delegation;
- the pinned Root and any sequential Root rotations included in the bundle.

The bundle is bounded to 32 metadata files, 1 MiB per file, 8 MiB total, and a
256 KiB policy. Root rotations are bounded to 16 per upload. Metadata bytes are
stored unchanged in `client_releases`; `client_channels` holds the active head.
Each immutable publication retains actor, timestamp and previous head as its
local audit trail, ready for future integration with the central audit owner.

Channel locking prevents concurrent forks. Policy, Snapshot, Timestamp and
channel-role versions advance; Root/Targets cannot decrease. Previously used
versioned filenames and release/artifact identities cannot change bytes.
Release ordering uses numeric sequences, never the display version. Rollback
must select an already-known immutable release in a newer signed policy with
`rollback_authorization: "explicit"`. Support floors and revocations cannot
be silently reversed. Database triggers prohibit update/delete of publication
history. Metadata verification and all checks finish before the active pointer
changes in the same transaction.

## GET surface

- `/updates/<channel>/metadata/<filename>` serves exact uploaded metadata.
  Current unversioned aliases (`root.json`, `targets.json`, `snapshot.json`,
  `<channel>.json`, `timestamp.json`) resolve through the active publication.
- `/updates/<channel>/policy-v1.json` serves the active policy target bytes.
- `/updates/<channel>/targets/<hash>.channels/<channel>/policy-v1.json`
  serves historical policy bytes using the consistent-snapshot target name
  requested by the selected verification library.

Mutable aliases fail unavailable when required metadata expires; immutable
versioned files remain retrievable for in-flight refreshes. Missing channel,
release or artifact does not fall back. Package bytes must be supplied by the
release distribution origin separately; the portal never proxies them.
All JSON responses prohibit transformation. Policy consumers must verify the
TUF target hash; the convenience policy URL alone is not a trust authority.

## Compatibility evidence and remaining work

The owner-pinned Game commit
`857f15f376907c592bb28d854fccff18a15cdf87` was inspected read-only.
`apps/client/src/bin/oteryn-launcher.rs` only reads `current.txt` and starts an
installed executable. There is no TUF implementation or updater policy parser
in that commit's Rust application/crate sources to use as a compatibility oracle.
The policy fixture therefore checks Platform's documented schema v1 (ADR 0035),
not an asserted launcher contract. See `tests/fixtures/SOURCE.md`.
The origin contract fixes the distribution profile and paths. Actual launcher
policy parser compatibility still requires client evidence before activation.
No production activation or client compatibility is claimed. No player/staff pages are included.

## Tests

Set `PORTAL_TEST_DATABASE_URL` to an isolated PostgreSQL 18.6 test database.
Tests create and drop UUID-named schemas, apply embedded workspace migrations,
and cover publication/GET byte identity, invalid signatures, expiry/tampering,
replay/rollback, immutable history, account/RBAC denial, invalid input and
concurrent publication. Eight database tests run, including operator publication
and signed contract bundles; without the variable each prints an explicit skip
notice. Test signing keys are generated in memory at runtime.
No SQL query macros are added, so no new SQLx offline query data is required.
