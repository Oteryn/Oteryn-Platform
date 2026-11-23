# Agent HTTP transport

ADM-01 `oteryn.admin-control.v1`, pinned Game contract commit
`3d4ea6ed913359c2e5aafb7ab6b890a894ff32fa`. Platform only accepts incoming
agent requests; it never connects to Game. No operation is emulated.
The pin includes the Game ADM-G3 read contract. Portal agent transport and
console query submission require their existing opt-in flags, which default
to false. Phase-2 entries and the named `analytics.query` forms are contract metadata. See
[fixture provenance](fixtures/admin-control-v1/SOURCE.md).

`PORTAL_ADMIN_CONTROL_ENABLED` defaults to false (all six routes return 404).
When enabled, `PORTAL_ADMIN_CONTROL_AGENTS_JSON` must be a nonempty array of
closed objects: `cn`, `identity`, `allowed_worlds`, `allowed_nodes`, optional `result_key`.
Both scope arrays are required.
Each contains exact identifiers the peer can claim in hello or continue using
from a stored registration. An empty array grants no scope of that type.
For example, `"allowed_worlds":["world-a"],"allowed_nodes":[]` grants only
`world-a`. CN has the form
`oteryn-admin-agent/<agent_id>`. Identity is exactly one of
`{"san":"exact verified SAN"}` or `{"spki":"base64url SHA-256"}`. A result
key has `public_key` (base64url Ed25519), `not_before`, `not_after` (Unix seconds).
Only public key material belongs in this configuration. Duplicate CNs or
transport identities fail startup. Configure a current result key before
delivering requests. The transport requires a pool with at least two connections.

The trusted TLS adapter must insert both the existing guard's `VerifiedClient`
and a `VerifiedAgent` from the verified certificate. Neither is derived from
headers. The ordinary plaintext server cannot supply these extensions and
therefore denies enabled requests with 403.

The internal mTLS listener (`bin/oteryn-portal/src/internal_tls.rs`) inserts
`VerifiedAgent` through `AgentIdentities::verify` only while
`PORTAL_ADMIN_CONTROL_ENABLED` is on, the leaf has exactly one subject CN of the
form `oteryn-admin-agent/<agent_id>`, and that agent's configured identity
equals the leaf's verified single DNS SAN. The SAN is the authority; the CN only
selects the entry. That listener identifies peers by SAN only, so an agent
configured with an `spki` identity is refused there. Production ingress is not
enabled by this code.

Nginx/ingress integration must allowlist the dedicated agent SAN/SPKI identity
and verify its certificate CN, in addition to the existing internal identities.
Preserve TLS provenance in the trusted adapter; forwarding certificate identity
in public HTTP headers is insufficient. Allow at most 25 seconds per channel
poll (plus transport overhead), and at most 90 MiB encoded stream bodies.
Enable this path only behind the internal mTLS listener described above.

All endpoints are POST under `/internal/v1/admin-control/` with application/json.
Unknown structural members and duplicate JSON members are rejected. JSON depth
is at most 32; normal bodies at most 1 MiB; stream bodies at most 90 MiB.
Integers outside the safe JSON range must be decimal strings.

The `hello`, `channel`, `results` and `heartbeat` bodies follow the Game agent
framing ratified by the owner on 2026-11-19
(`docs/contracts/ADMIN_CONTROL_AGENT_WIRE_RATIFICATION.md`).

- `hello`: `agent_id`, `build_revision`, `catalog_digest` (32 bytes, unpadded
  base64url), `operations` (`operation`, `version`), `worlds`, `nodes`, positive
  `owner_epoch`, RFC3339 `clock`. Phase-2 catalog entries are omitted from the
  registered capabilities. Game advertises the 13 registered read executors at
  this pin. Each accepted operation's kind comes from the
  trusted catalog (`catalog::advertised_kind`). Other unknown operations or
  versions are refused with 400 `ADM_UNKNOWN_OPERATION`.
- `channel`: `agent_id`, `owner_epoch`, optional `wait_seconds` (0–25, default 0),
  optional `limit` (1–32, default 32). Returns a JSON array of at most `limit`
  signed request envelopes for qualified v1 reads. Delivery requires a current
  result key and configured scope. It is claimed once and records the owner epoch.
  Expired requests are never returned. Disconnected or ambiguous owners deny.
  `db.status`, `db.migrations.list`, and `admin.policy.get` are global reads
  with an empty signed target. Other Game reads require their catalogued scope.
- `results`: the bare closed ADM-01 result envelope. The current configured
  result key and TLS agent identity authenticate every receipt. The stored
  assignment, target scope and delivery epoch must match. A superseded owner
  or a request without a matching delivery receives 409 `ADM_AGENT_SUPERSEDED`.
  The stored operation and version select the payload schema. Exact retries are
  acknowledged after those checks without another result or audit row. New
  receipts and legal state transitions commit together before acknowledgment.
- `stream`: `agent_id`, `owner_epoch`, `request_id`, nonempty `chunks` array of
  `index` (0–63) and `data` (unpadded base64url). Decoded chunks are at most 1 MiB;
  the primary key and index bound limit the complete request to 64 chunks across
  posts. Exact retries succeed; changing an existing chunk conflicts. Posts are
  atomic. Request ownership is checked before persistence.
- `rollups`: `agent_id`, `owner_epoch`, `key` (1–256 bytes), object `payload`.
  Upserts the latest snapshot per agent/key in `analytics_cache`.
- `heartbeat`: `agent_id`, `owner_epoch`, RFC3339 `clock`, optional object
  `health`. After the epoch check it refreshes last_seen; it stores the latest
  health snapshot only when `health` is supplied. Stale owners are refused.

Health and analytics rollup objects carry source data. The transport stores
them for consumption through their respective projections.

Tests use `PORTAL_TEST_DATABASE_URL`; when unset each database test prints a
skip notice. PostgreSQL tests create isolated schemas and run migrations there.

## Staff-actions v1 catalog (S12-P2A)

`catalog::StaffOperation` registers five command definitions at version 1 against
Game commit `457713b25f153c2b09e3dffe420d22fd5379c8f7`. Pinned files and SHA-256
hashes live in `fixtures/staff-actions-v1/SOURCE.md`. Golden signatures and preview
digests are structural placeholders, never executable evidence.

`catalog::Catalog::default()` has an empty staff allowlist: every staff operation
returns `Disabled`, independently of `PORTAL_ADMIN_CONTROL_ENABLED`. A future
trusted caller must explicitly supply `Catalog::with_staff_allowlist`; no env
wiring, agent advertisement, dispatch, result union, revision allocation or store
change is included. Existing authority adapters still fail closed. Transport
activation and agent hello do not enable these definitions. Rollout requires the
contract's joint acceptance, result-union integration, environment trust separation
and snapshot/replay prerequisites.

Closed `StaffParams` plus `validate_staff_request` enforce conditional expiry,
positive safe-integer revisions (maximum 9007199254740991), at most 32 opaque
nonempty evidence refs, character targets for namelock, and preview presence only
for executing R4. Expiry must be RFC3339 UTC and follow issuance. Permission uses
existing scopes: warning/notation `game.sanction.write` R1; namelock
`game.sanction.ban` R2; temporary ban/unban `game.sanction.ban` R3; permanent ban
`game.sanction.ban` R4. Policy derives from operation and validated params; client
risk/permission claims must agree and never select policy. Structural validation
neither verifies signatures, preview matches, approval identity/freshness or account
ownership, nor grants execution authority.

Task authority: owner request S12-P2A, branch `codex/portal-rs-s12-p2a`, base main;
branch retained for owner review, no PR. Changes stay within portal-rs per owner
scope. No mapped live S12 Issue was found; Jira synchronization is unknown.
Runtime E2E is not applicable: disabled definitions and pure validation have no
actor-to-effect path.

Console query composition
-------------------------

With `OTERYN_ADMIN_CONSOLE` and `PORTAL_ADMIN_CONTROL_ENABLED`, the configured
console uses the request/result store. `PORTAL_ADMIN_CONTROL_SIGNER_KEY_FILE`
names the existing signer's 32-byte seed file. Its public identity and validity
window come from `PORTAL_ADMIN_CONTROL_SIGNER_KEY_ID`,
`PORTAL_ADMIN_CONTROL_SIGNER_NOT_BEFORE` and
`PORTAL_ADMIN_CONTROL_SIGNER_NOT_AFTER` (Unix seconds). Each agent needs its
configured result verification key. Missing custody or verification authority
refuses queries. No private material is included in browser responses.

Game migration 3076 creates the curated `admin_read` views and the
`oteryn_game_admin_read` NOLOGIN role. The deployed Game read login must have
`CONNECT` on the Game database and membership in `oteryn_game_admin_read`.
Enable individual reads in the pinned Game policy and keep the portal staff
console flag off unless this access is intended.

Admission rechecks staff permissions and the browser's durable MFA grant, live
agent scope and query capability, and the four-query per-actor limit. The signed
request has a 60-second expiry. Admission and its audit commit together, including
the admitting agent and its epoch. A new owner can complete a queued request.
The console finds its admission by actor and request ID, then checks the
operation, parameters, scope, result owner and signature before presentation.
New result transitions also record the completing owner’s epoch. Only
bounded, named presentation fields reach templates. Ambiguous targets refuse;
list operations need an unambiguous served node or world when none was selected.
