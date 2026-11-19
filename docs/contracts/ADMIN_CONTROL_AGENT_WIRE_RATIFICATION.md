# Admin control agent wire: ratification note

Status: RATIFIED by the owner on 2026-11-19 (Issue #1957, findings H039, H040 and H041).
Scope: the body framing of `POST /internal/v1/admin-control/{hello,channel,results,heartbeat}`.
ADM-01 leaves this framing open. Operation semantics, signing and the request
and result envelopes are unchanged.

Source: Oteryn-Game `crates/admin/agent/src/channel.rs` and
`docs/contracts/admin-control/v1/README.md` at commit
`3732e96aa2874204dcc97504f13681cf04a65c6e`. Game publishes no fixture files for
these bodies. The test bodies in
`portal-rs/crates/admin/src/control/http_tests.rs` are transcribed from
`channel.rs` lines 114-118, 123, 252 and 259 at that commit. The vendored
`request.schema.json` and `result.schema.json` in
`portal-rs/crates/admin/src/control/fixtures/admin-control-v1/` still match that
commit byte for byte (same SHA-256 as `SOURCE.md`).

## Decisions

1. **Hello.** Game's framing is authoritative:
   `{agent_id, build_revision, catalog_digest, operations:[{operation, version}], worlds, nodes, owner_epoch, clock}`.
   The body is closed. The agent never supplies the operation kind: Platform
   resolves it from its trusted catalogue. Phase-1 read operations are queries
   at version 1, and staff operations take their catalogued kind and version.
   An unknown operation or version refuses the whole hello with 400
   `ADM_UNKNOWN_OPERATION`. Scope and epoch checks are unchanged.
2. **Poll.** `{agent_id, owner_epoch, wait_seconds 0-25, limit 1-32}`, where
   `limit` defaults to 32. The response is a JSON array of signed request
   envelopes.
3. **Result.** The body is the bare signed result envelope. It is bound to the
   authenticated delivery session: the agent that claimed the request, at the
   owner epoch recorded when it was delivered. A stale or superseded epoch
   gets 409 `ADM_AGENT_SUPERSEDED`. This includes the same agent after a newer
   hello, and a request with no recorded delivery. A 409 response means
   superseded to the Game client.
4. **Heartbeat.** `{agent_id, owner_epoch, clock}` satisfies liveness on its own
   and is bound to `agent_id` and `owner_epoch`. An optional object `health` is
   stored when it is present and never required.

Platform's earlier framing (`build`, `advertised_operations` with `kind`, the
wrapped `{owner_epoch, envelope}` result, the `{requests:[...]}` poll body and
the heartbeat `payload`) is no longer accepted. The feature stays behind
`PORTAL_ADMIN_CONTROL_ENABLED`, which is off by default.
