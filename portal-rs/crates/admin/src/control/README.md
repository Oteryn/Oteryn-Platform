# Agent HTTP transport

ADM-01 `oteryn.admin-control.v1`, pinned Game contract commit
`5ff65ff759b5d5278d8531728ae604e865b1870e`. Platform only accepts incoming
agent requests; it never connects to Game. No operation is emulated.

`PORTAL_ADMIN_CONTROL_ENABLED` defaults to false (all six routes return 404).
When enabled, `PORTAL_ADMIN_CONTROL_AGENTS_JSON` must be a nonempty array of
closed objects: `cn`, `identity`, optional `result_key`. CN has the form
`oteryn-admin-agent/<agent_id>`. Identity is exactly one of
`{"san":"exact verified SAN"}` or `{"spki":"base64url SHA-256"}`. A result
key has `public_key` (base64url Ed25519), `not_before`, `not_after` (Unix seconds).
Only public key material belongs in this configuration. Duplicate CNs or
transport identities fail startup. The database pool needs at least two
connections: duplicate-result serialization holds one connection while the
existing store commits its audited state transition on another.

The trusted TLS adapter must insert both the existing guard's `VerifiedClient`
and `VerifiedAgent::from_verified_certificate_cn` from the verified certificate.
Neither is derived from headers. The ordinary plaintext server cannot supply
these extensions and therefore denies enabled requests with 403. This packet
does not configure a TLS listener or enable production ingress.

Nginx/ingress integration must allowlist the dedicated agent SAN/SPKI identity
and verify its certificate CN, in addition to the existing internal identities.
Preserve TLS provenance in the trusted adapter; forwarding certificate identity
in public HTTP headers is insufficient. Allow at most 25 seconds per channel
poll (plus transport overhead), and at most 90 MiB encoded stream bodies.
Do not enable this path until the verified connection adapter is wired.

All endpoints are POST under `/internal/v1/admin-control/` with application/json.
Unknown structural members and duplicate JSON members are rejected. JSON depth
is at most 32; normal bodies at most 1 MiB; stream bodies at most 90 MiB.
Integers outside the safe JSON range must be decimal strings.

- `hello`: `agent_id`, `build`, `catalog_digest` (32 bytes, unpadded base64url),
  `advertised_operations` (`operation`, `operation_version`, `kind`),
  `served_worlds`, `served_nodes`, positive `owner_epoch`, RFC3339 `clock`.
- `channel`: `agent_id`, `owner_epoch`, optional `wait_seconds` (0–25, default 0).
  Returns `requests` (at most 32 signed envelopes). Delivery is claimed once;
  expired requests are never returned. Disconnected or ambiguous owners deny.
- `results`: `owner_epoch`, `envelope` (closed ADM-01 result envelope). Agent ID
  must match TLS evidence. A configured result key verifies the signature before
  storage. Exact duplicate envelopes succeed without another result/audit row,
  including after a terminal result. Conflicting transitions are refused.
- `stream`: `agent_id`, `owner_epoch`, `request_id`, nonempty `chunks` array of
  `index` (0–63) and `data` (unpadded base64url). Decoded chunks are at most 1 MiB;
  the primary key and index bound limit the complete request to 64 chunks across
  posts. Exact retries succeed; changing an existing chunk conflicts. Posts are
  atomic. Request ownership is checked before persistence.
- `rollups`: `agent_id`, `owner_epoch`, `key` (1–256 bytes), object `payload`.
  Upserts the latest snapshot per agent/key in `analytics_cache`.
- `heartbeat`: `agent_id`, `owner_epoch`, object `payload`. Refreshes last_seen
  and stores the latest health snapshot. Snapshot updates refuse stale owners.

Payload objects are opaque operation/health/analytics data, not executable
commands. Future consumers must validate the advertised operation's versioned
result schema before displaying its contents. No UI is added by this packet.

Tests use `PORTAL_TEST_DATABASE_URL`; when unset each database test prints a
skip notice. PostgreSQL tests create isolated schemas and run migrations there.
