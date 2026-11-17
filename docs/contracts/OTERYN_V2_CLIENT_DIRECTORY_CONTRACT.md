# Client directory v1

Status: **ACCEPTED** — the owner lifted the DirectoryChannel hold on
2026-11-17 ("zgoda, zdejmij blokadę dla DirectoryChannel"); the programme
coordinator confirmed option (a) and the channel field list the same day.
This contract claims no Platform handler and no client conformance; the
Platform route is plan §5.2 / T36b, a separate task.

Fixtures: `docs/contracts/fixtures/client-directory-v1/`.
Consumer read: `Oteryn/Oteryn-Game@e2152d050038cecdb3728c222af2ac184e1cac5a`,
`crates/platform-client/src/lib.rs` (`parse_directory`),
`crates/platform-contracts/src/lib.rs` (`DirectorySnapshot`).

## Route

`GET /v1/client/directory`, joined by the client against the portal base URL
(plan §5.2). No body, no query. Every response carries
`Cache-Control: no-store, private`, `Pragma: no-cache` and
`Vary: Authorization`; a 200 is `Content-Type: application/json`.

| request | result |
|---|---|
| no `Authorization` header | 200, every `characters` is `[]` |
| `Authorization: Bearer <access token>` of the native OAuth client (the token `native-characters-v1` accepts) | 200, `characters` lists that account's characters on each world |
| bearer present but unknown, expired or revoked | 401, body `{"error":"unauthenticated"}` (as `native-characters-v1`) |

Characters are only ever the authenticated owner's (LCFA rev 2 forbids a
public character list of an account). A client that sends no token gets the
anonymous form; Platform never infers an owner from anything else.

## Body

```
{ "epoch": u64,
  "worlds": [ { "id": str, "name": str,
                "channels":   [ { "id": str, "name": str,
                                  "pvp_type": "OPTIONAL_PVP" | "OPEN_PVP",
                                  "region": str } ],
                "characters": [ { "id": str, "name": str } ] } ] }
```

| member | rule |
|---|---|
| `epoch` | unsigned 64-bit integer; increases when the directory changes |
| `worlds` | array, at most 256 |
| `worlds[].id`, `channels[].id`, `characters[].id` | string, 1-96 bytes UTF-8, no control characters; opaque reference |
| `worlds[].name`, `channels[].name`, `characters[].name` | string, 1-96 bytes UTF-8, no control characters; display text |
| `channels` | array, at most 128 per world; always present |
| `channels[].pvp_type` | exactly `OPTIONAL_PVP` or `OPEN_PVP` |
| `channels[].region` | string, 1-32 bytes UTF-8, no control characters; display/latency grouping label, not an address |
| `characters` | array, at most 256 per world; always present, `[]` when anonymous |
| whole body | at most 1 MiB (1048576 bytes) |

Wire keys of a character are `id` and `name`; `character_ref` and
`display_name` are the Rust field names in `platform-contracts`, not wire keys.
Platform emits every member listed above and no other member.

## No route discovery

The directory never carries a gameplay route or credential. A consumer rejects
the whole body (`ForbiddenGameplayField`) when any object at any depth has one
of these keys: `host`, `port`, `endpoint`, `endpoint_uri`, `protocol`,
`protocol_profile`, `ticket`, `credential`, `game_session`, `admission`,
`route`, `address`. Clients learn routes from the contract version, not from
the directory. Latency is measured by the client against the route it already
knows; a server load or "full" indicator is not part of v1.

## Consumer rules

A v1 consumer rejects the whole body (`InvalidPayload`) when it is not this
shape: a missing or wrongly typed required member, a limit exceeded, a string
out of bounds, or a `pvp_type` outside the two values above. Unknown members
other than the forbidden keys are ignored. The Game parser at the pinned commit
already enforces the limits, the `id`/`name` members and the forbidden keys; it
does not yet read `pvp_type` or `region` (a Game-side change, outside this
repository).

A new `pvp_type` value, a new required member or a change to a limit is a new
contract version.

## Vectors

| case | client outcome |
|---|---|
| `directory-anonymous` | accept |
| `directory-owner` | accept |
| `directory-rejected-route-key` | reject, `ForbiddenGameplayField` |
| `directory-unknown-pvp-type` | reject, `InvalidPayload` |
