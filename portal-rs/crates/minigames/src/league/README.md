# PLAT3-T94 — world outcomes and guild season consumer

Game producer and contract baseline:
`9668e0a04e1a631ebfc8752772102a1fd98ce8fa` (Game PR #3015).
See `fixtures/SOURCE.md` for byte-exact fixture provenance. Game is read-only.

## Running the feature

`OTERYN_LEAGUE_ENABLED=true` selects public `/pl/league`, `/en/league`, navigation,
the dedicated private listener and the worker. `serve` uses `GameAuthority`;
`worker` schedules `FeedJob` through the existing leased scheduler. Missing trust
configuration yields localized 503. Production never loads fixture authority.

Required deployment inputs (also documented in `e2e/ENVIRONMENT.md`):

- `OTERYN_LEAGUE_BASE_URL`: advertised HTTPS Platform endpoint without trailing slash.
- `OTERYN_LEAGUE_BIND`: private listen address.
- `OTERYN_LEAGUE_SOURCE_AUTHORITY`: exactly `oteryn:game:primary`.
- `OTERYN_LEAGUE_CLIENT_SPKI_SHA256`: `sha256:` plus 64 lowercase hexadecimal
  characters, SHA-256 of the dedicated Game leaf certificate's DER
  SubjectPublicKeyInfo. Bare lowercase hex is also accepted and normalized.
- `OTERYN_LEAGUE_CLIENT_CA_DER`, `OTERYN_LEAGUE_CERTIFICATE_DER`,
  `OTERYN_LEAGUE_PRIVATE_KEY_DER`: external client CA, server certificate and
  PKCS8 server key files.
- `OTERYN_LEAGUE_RECOVERY_FILE`: operator-owned independent nonrollback storage,
  JSON with exactly `source_authority`, `projection_epoch`, `recovery_generation`.
  Counters are positive canonical decimal strings. Serve and worker reread it on
  every admission/use. Provision or advance it through the recovery control plane,
  never ordinary restart. Fence writers must hold PostgreSQL advisory lock
  `94000003` while changing file/state. Never restore this file with the database.

The former `OTERYN_LEAGUE_PUBLIC_KEYS`, `OTERYN_LEAGUE_PEER_CERTIFICATE_SHA256`,
`OTERYN_LEAGUE_MAX_ENVELOPE_BYTES` and `OTERYN_LEAGUE_SUBJECT_LIMIT` settings are
retired. Provision the new SPKI pin explicitly; a whole-certificate digest is
not an SPKI digest. Key rotation requires updating deployment trust and restarting
serve/worker. Persisted admissions are rechecked against current trust.

## Game wire and authorization

Game initiates HTTP/1.1 over TLS1.3 with a dedicated mTLS client identity for
purpose `OTERYN_GAME_WORLD_EVENT_GUILD_SEASON`. Platform validates the client CA,
validity and clientAuth EKU, negotiates ALPN `http/1.1`, then compares the verified
leaf SPKI digest to its configured pin. That pin authorizes only
`oteryn:game:primary` and these canonical JSON POSTs:

| Route | Maximum bytes |
| --- | --- |
| `/internal/v1/game-public/world-event-outcome` | 16384 |
| `/internal/v1/game-public/guild-season` | 262144 |
| `/internal/v1/game-public/league-watermark` | 512 |

There are no admission/signature headers, Ed25519 payload signatures, native ANL
envelopes or league-cut requests on this wire. Forwarded certificate headers
cannot create the private verified-peer extension. A CA-valid alternate client
receives 401. Purpose keys must be distinct from Character/Bazaar/public projection
identities; deployment owners provision that separation.

Responses are at most 256 bytes and always `Cache-Control: no-store`.
200 contains the canonical accepted result; non-200 responses have empty bodies,
using 400/401/409/429/503 (contract diagnostics 5002–5007). Initial enqueue returns
retryable 503; 200 follows only after the worker commits. Identical retries use a
stable local receipt identity. Game retries 429/503 with bounded backoff up to 10s.
There is no pull API or inbound Game listener, and page rendering never calls Game.

## Completeness, recovery and audit

Each subject publication is a complete snapshot. Empty lists withdraw display;
they do not mean VOID or zero standings. The worker validates canonical bytes,
registered World identity, immutable question/occurrence identity, revision order,
correction lineage and final-state rules. Equal revision with different content,
revision gaps and regressions fail closed. SCHEDULED/LOCKED observations retain
controls but never open predictions or appear as decided outcomes.

Game acknowledges retained snapshots/correction lineage before advancing either
family watermark. Platform processes its durable queue in arrival order. Serving
requires no pending snapshots, both fresh watermarks, and reconciliation of all
previously known retained subjects in the configured epoch. Unknown old subjects
stay hidden and keep serving unavailable. Epoch changes require the independent
operator fence; incoming or operator-fence regressions return 409. A fence update
never clears quarantine or invents missing correction history. Immutable CLOSED
records cannot be rewritten by recovery.

Watermarks are monotone within an epoch, complete_through <= observed_at, with no
future time beyond 1s uncertainty. Every serve checks
`now - complete_through + 1 <= 30`. Staleness closes serving with 503; it cannot
fabricate results. No snapshot manifest or extra feed is required.

Native ANL audit envelopes remain Game-owned. Platform retains canonical admitted
snapshots, their verified SPKI identity, local receipts and derived adapter audit
payloads. These local records are not native Game provenance. For compatibility,
historical SQL columns named `signature` and `peer_certificate_sha256` store the
verified SPKI identity; old signature metadata and certificate digests fail the
new grammar. No migration or old migration checksum change is needed.
Local audit expiry uses immutable snapshot observation timestamps; first observed
final-freeze retention is preserved across retries. Raw inbox/audit/history is
bounded to P90D; minimal non-reuse, receipt and final-state controls persist.

## Policy and resource bounds

Owner acceptance flags `accepted_sources`, `accepted_season_rules`,
`accepted_final_freeze`, `accepted_pki` default true. `OTERYN_LEAGUE_POLICY` may
supply these boolean members; malformed policy refuses. Policy flags never
substitute for deployment trust or Game source facts. Outcome mappings, seasons,
scoring, guild attribution and correction windows remain Game-owned. There are no
predictions, stakes, rewards, Wallet/Store effects or inferred season-pass sources.

Defaults: 16 TLS connections, 2s handshake, 5s exchange, 120 requests/minute,
64 queued requests per worker tick. Lower connection/batch caps through
`OTERYN_LEAGUE_CONNECTION_LIMIT` and `OTERYN_LEAGUE_BATCH_LIMIT`; configure the
request cap in 1..1200 with `OTERYN_LEAGUE_REQUESTS_PER_MINUTE`.
Deployment trust and recovery state have no guessed defaults.

## Tests

The E2E launcher generates disposable certificates, replays every pinned JSON
fixture without additional headers, then publishes the two family watermarks.
The real listener, worker and public pages are exercised in en/pl at 1440/390
widths. A second CA-valid certificate proves pin rejection; pausing watermarks
proves stale public refusal. This is local stub qualification, not a deployment.

Rust regressions cover exact header-free Game wire admission and alternate peer
refusal, canonical parsing, conflicts, recovery, unknown retained subjects,
retention and staleness. Database tests use `PORTAL_TEST_DATABASE_URL` and isolated
schemas; absence emits a skip notice. Run formatting, workspace Clippy with
warnings denied, workspace tests and `e2e/specs/league.spec.mjs`.
