# PLAT3-T94 — world outcomes and guild season consumer

Owner follow-up decision **2026-11-06** accepts candidate proposals. The actual
Game contracts/fixtures remain pinned to `3b49a20feec3ba29700bb4b2fa28fa9b539b79a2`;
this owner decision does not claim that Game at that SHA implements a publisher.
See `fixtures/SOURCE.md`. No Game repository is modified.

## Running the feature

`OTERYN_LEAGUE_ENABLED=true` is the **one** feature flag for public `/pl/league`,
`/en/league`, navigation, the dedicated private publisher listener and the worker.
The previous `OTERYN_LEAGUE_PUBLIC_ENABLED` flag is removed. `serve` wires the real
`GameAuthority`; `worker` schedules `FeedJob` every second through the existing
leased outbox scheduler. No production composition uses the `Unavailable` adapter.
Missing trust configuration still yields localized 503, never trusted fixture data.

Required deployment inputs (see `portal-rs/e2e/ENVIRONMENT.md`):

- `OTERYN_LEAGUE_BASE_URL`: the HTTPS **Platform admission** audience, without a
  trailing slash; `OTERYN_LEAGUE_BIND`: its private listen address.
- `OTERYN_LEAGUE_SOURCE_AUTHORITY`: exact dedicated outcome/season purpose name.
- `OTERYN_LEAGUE_PUBLIC_KEYS`: JSON object mapping key IDs to 32-byte Ed25519 public
  keys in lowercase hex. It contains no private key. Remove a key to revoke it on
  process restart; queued proofs are reverified and public cut use checks trust.
- `OTERYN_LEAGUE_CLIENT_CA_DER`, `OTERYN_LEAGUE_CERTIFICATE_DER`,
  `OTERYN_LEAGUE_PRIVATE_KEY_DER`: external CA, server certificate and PKCS8 key
  files; `OTERYN_LEAGUE_PEER_CERTIFICATE_SHA256`: exact dedicated Game leaf pin.
- `OTERYN_LEAGUE_RECOVERY_FILE`: operator-owned **independent nonrollback storage**,
  UTF-8 JSON with exactly `source_authority`, `projection_epoch`,
  `recovery_generation` (positive decimal strings). Serve and worker reread it on
  every proof use. Provision/bump it through the recovery control plane, never
  ordinary restart. Fence writers must hold the same exclusive PostgreSQL advisory lock `94000003` while updating the file/state, so in-flight proof transactions cannot cross the change. Restoring this file along with the database defeats the fence
  and is forbidden operationally. A different/older database generation cannot
  serve until a complete authenticated cut and correction lineage reconcile.

Example public configuration: keys and certificate paths are deployment-owned;
no sample private key or production certificate is checked into this repository.

## Transport and admission profile

The pinned Game contract specifies **Game-initiated** HTTP/1.1, TLS 1.3 mTLS POSTs
at these Platform routes, with exact unchanged snapshot/watermark bodies:

- `/internal/v1/game-public/world-event-outcome` (16384 bytes)
- `/internal/v1/game-public/guild-season` (262144 bytes)
- `/internal/v1/game-public/league-watermark` (512 bytes)

There is no pull-feed endpoint or inbound Game listener in the pinned contract.
Platform does not invent one or query Game while rendering pages. HTTP responses
are the return direction: accepted only after worker commit, otherwise empty
400/401/409/429/503, all no-store. First staging normally returns retryable 503;
the publisher retries identical bytes (backoff <=10s) until committed 200.

The candidate defines ANL payloads but **no detached-signature framing or full-cut
manifest wire**. The following is a separate **Platform admission profile v1**,
chosen under the owner follow-up, not silently attributed to the Game contract.
A real Game publisher must implement this profile alongside its normative bodies;
the pinned Game SHA itself has no live producer. The E2E producer implements it
only for local testing; that is not live Game producer/consumer qualification. The stub registers its synthetic Platform World metadata after migrations; outcomes and standings enter solely through the real private listener.

Every private POST carries:

1. `x-oteryn-league-admission`: standard-base64 of canonical JSON in this exact
   member order: `contract_version` (1), `source_authority`, `projection_epoch`,
   `recovery_generation`, `key_id`, `request_id` (UUIDv7), `envelope` (base64 of
   exact existing ANL EventEnvelope protobuf bytes, empty for cut/watermark or
   withdrawal or SCHEDULED/LOCKED observation), `finalized_at` ([] or one authenticated first-freeze Unix second).
2. `x-oteryn-league-signature`: lowercase hex Ed25519 signature over UTF-8:
   `oteryn-platform-league-admission-v1\n<audience>\nPOST\n<path>\n<base64 admission>\n<SHA256 exact body>`.

TLS client CA validation, TLS1.3/HTTP1.1/ALPN and exact certificate pin all precede
HTTP access. Public forwarded-certificate headers are never trusted. The signature
binds route, audience, recovery fence, exact body, native envelope and freeze time.
Purpose identities/keys must be distinct from Character/Bazaar/public projections.

Decisions use registered ANL types 10/11, revision 1, DURABLE_AUDIT, privacy class
RESTRICTED_PLAYER_LINKED, PLAT3_AUTHORITY_AUDIT_RETENTION_V1 and TransactionEventRef.
Nonterminal observations retain immutable question/occurrence controls and participate in the complete cut, but are never displayed as decided and never open predictions. The current admission profile accepts the closed non-personal envelope subset in
`admission::Envelope`; other optional fields/schema revisions refuse rather than
being silently discarded. Payload/record/evidence hashes, UUID domains, stable
transaction counts and exact prost bytes are checked. World IDs must exist in Platform’s registry and are locked during proof use; a deleted/unregistered World cannot serve. No hash alone supplies a
missing native receipt: Game's authenticated purpose attests retained evidence.

A full-cut control POST is additionally required at
`/internal/v1/game-public/league-cut` (max 262144 bytes), signed with the same
profile. Canonical body order: `contract_version:1`,
`operation:"PublishLeagueCutV1"`, `source_authority`, `projection_epoch`,
`recovery_generation`, `subjects`. Each subject has exactly `family`, `world_id`,
`subject_id`, `projection_revision`, `record_sha256`, `withdrawn` (boolean).
Sort by family/world/subject; include **every** retained subject and explicit
withdrawal, including VOID/CLOSED. A withdrawal's record hash is SHA256(empty
bytes), matching the absent record; it never means a void result or zero score.
Publish the cut first, replay required decisions/corrections in order, then the
family watermarks. The worker atomically exposes the cut only on exact full-set
match, with no unknown previous subjects and both fresh watermarks. Missing
history, partial enumeration, quarantine or mismatched freeze stays unavailable.
The cut contains final subject revisions; intermediate native corrections may be
replayed to reach those revisions. Epoch/recovery changes never rewrite a CLOSED
record, mint a generation or silently clear a quarantine.

## Owner defaults

- The four owner acceptance choices (`accepted_sources`, `accepted_season_rules`,
  `accepted_final_freeze`, `accepted_pki`) default **true**. Override with the
  corresponding boolean members of `OTERYN_LEAGUE_POLICY` JSON. Missing policy
  uses accepted defaults; malformed policy refuses. These are policy choices,
  never substitutes for actual signatures, fences, source facts or key material.
- Outcome mappings/aggregation, season calendar/challenges/scoring, guild cut
  attribution/eligibility/tombstone names and correction windows come from the
  authenticated Game authority and immutable supplied rules, not Portal guesses.
  No concrete supported live emitter is asserted by the pinned Game code.
- No eligible prediction windows are supplied by this contract. The conservative
  default is **zero open windows**; predictions, stakes, rewards and Wallet/Store
  effects do not exist in this consumer. No season-pass source is inferred.
- Watermark interval <=10s, stale bound 30s with 1s uncertainty, exact positive
  fences, immutable final freeze and P90D audit/history ceilings follow the contract.
- Unspecified local resource bounds: 16 concurrent TLS connections; 2s handshake,
  5s exchange; publisher cap 120/minute (configurable 1..1200 through
  `OTERYN_LEAGUE_REQUESTS_PER_MINUTE`); 64 queued requests processed per worker tick;
  cut enumeration <=1000 retained subjects. Lower these caps with
  `OTERYN_LEAGUE_CONNECTION_LIMIT`, `OTERYN_LEAGUE_BATCH_LIMIT`, and
  `OTERYN_LEAGUE_SUBJECT_LIMIT`, respectively. ANL admission envelope ceiling 4096
  bytes (can only be lowered through `OTERYN_LEAGUE_MAX_ENVELOPE_BYTES`). These are
  conservative Platform caps, not permission to enlarge Game ANL hard ceilings.
- Deployment trust values and recovery state have **no guessed defaults**. Supply
  them explicitly. Retention scheduling is wired into the real worker; raw inbox
  and audit data expire from immutable ANL timestamps. Minimal request/receipt,
  final-state and occurrence non-reuse controls persist separately.

## Tests and local proof

The local launcher generates disposable TLS and Ed25519 keys, starts the isolated
fixture Game publisher and the real serve/worker roles. No test trust shortcut,
fixture loader, clock override or signing private key exists in production paths.
`e2e/specs/league.spec.mjs` drives both locales and 1440/390 viewports, checks native
outcomes/standings and checks a bad signature followed by stale-cut public refusal.
The source snapshot file/wire strings stay byte-exact; synthetic ANL envelope
identity, signatures and initial freeze commit timestamp are test-only provenance.

Rust PostgreSQL tests use `PORTAL_TEST_DATABASE_URL`, isolated schemas, and the
complete migration chain. Absence emits a skip notice. Run formatting, full
workspace clippy with warnings denied, workspace tests, and the league Playwright
spec against `e2e/local.mjs`. This proves the Platform user path with a local Game
stub; a deployed Game emitter and environment rollout remain separate evidence.
