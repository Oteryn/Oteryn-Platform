# Client channel probe v1

Status: **ACCEPTED** — the Game client session confirmed v1 on 2026-11-17
with the measurement and timeout rules below. The owner
decided on 2026-11-17 that the channel picker shows a ping per channel,
because channels can sit in different regions. This contract claims no
Platform handler, no edge implementation and no client conformance; the
Platform route is a follow-up to plan task T36b, a separate task.

Fixtures: `docs/contracts/fixtures/client-channel-probe-v1/`.
Consumer: the Game client session (channel picker). There is no client parser
yet, so no Game commit or file is pinned.

## Why a separate contract

Before admission the client knows no per-channel endpoint: the only one it
learns comes in the NativeLoginGrant, which consumes the ticket. A probe
served by the central portal, such as one keyed by `channel_ref`, would
measure latency to the portal, not to the channel's region. So the latency
target lives on the regional edge, and the list that maps channels to targets
stays out of the gameplay directory (`OTERYN_V2_CLIENT_DIRECTORY_CONTRACT.md`
v1), which carries no route and is unchanged by this contract.

The contract has two parts: the probe list on the portal and the probe target
on each regional edge.

## Part 1: probe list

`GET /v1/client/channel-probes`, joined by the client against the portal base
URL (plan §5.2), the same base URL as the directory. No body, no query. Every
response carries `Cache-Control: no-store, private`, `Pragma: no-cache` and
`Vary: Authorization`, as the directory; a 200 is
`Content-Type: application/json`.

| request | result |
|---|---|
| no `Authorization` header | 200 |
| any `Authorization` header | 200, the same body; the bearer is optional and ignored, never validated and never a 401 |

### Body

```
{ "epoch": u64,
  "probes": [ { "channel_id": str, "probe_url": str } ] }
```

| member | rule |
|---|---|
| `epoch` | unsigned 64-bit integer; increases when the list changes |
| `probes` | array, at most 32768 (256 worlds × 128 channels); always present |
| `probes[].channel_id` | equals a directory `channels[].id`: string, 1-96 bytes UTF-8, no control characters; at most one entry per channel |
| `probes[].probe_url` | see below |
| whole body | at most 256 KiB (262144 bytes) |

Both limits apply; at full channel count the byte limit binds first.

`probe_url` is an absolute URL that:

- starts with exactly `https://`;
- has a non-empty host and no userinfo (`@`);
- has no explicit port (the edge listens on 443);
- has no query (`?`) and no fragment (`#`);
- has the path exactly `/v1/probe`;
- is at most 256 bytes.

It is a latency target of the regional edge, not a gameplay route: it grants
nothing and carries no ticket. Channels sharing an edge may share a
`probe_url`. A directory channel with no probe entry shows "—" in the picker;
a `channel_id` that is not in the directory is ignored.

Platform emits every member listed above and no other member.

## Part 2: probe target

`GET /v1/probe` and `HEAD /v1/probe` on each regional edge.

| request | result |
|---|---|
| within the per-IP budget | `204`, empty body, `Cache-Control: no-store`; no `Access-Control-Allow-Origin` header |
| over the budget | `429` with `Retry-After` (seconds) |

- No credential, no cookie and no ticket is sent or read; the edge sets no
  cookie.
- Rate limit: 30 requests per minute per source IP.
- The response never reveals load, version or a host name (no `Server`
  version, no host or instance header, no body).

The client times the round trip itself. Per refresh it opens one kept-alive
connection to each distinct `probe_url`, sends one untimed warm-up request
(which absorbs the TLS handshake) and then at most 3 timed `HEAD /v1/probe`
requests on that connection, so at most 4 requests per edge per refresh. It
refreshes at most once per 30 seconds while the picker is open, which is at
most 8 requests per minute per edge and stays within the per-IP budget. It
shows the median of the timed requests for every channel that uses that URL.
Each request times out after 2 seconds; a URL that timed out, failed,
redirected or answered 429 shows "—", and the client honours `Retry-After`
before it probes that URL again.

## No route discovery

The probe list never carries a gameplay route or credential. A consumer
rejects the whole body (`ForbiddenGameplayField`) when any object at any depth
has one of the twelve keys the directory forbids: `host`, `port`, `endpoint`,
`endpoint_uri`, `protocol`, `protocol_profile`, `ticket`, `credential`,
`game_session`, `admission`, `route`, `address`.

## Consumer rules

A v1 consumer rejects the whole body (`InvalidPayload`) when it is not this
shape: a missing or wrongly typed required member, a limit exceeded, a string
out of bounds, a repeated `channel_id`, or a `probe_url` that breaks any rule
of Part 1. Unknown members other than the forbidden keys are ignored. A body
over 256 KiB is rejected before parsing.

A change to a limit, a new required member or a change to the `probe_url`
rules is a new contract version.

## Security

- The probe list does not let an attacker learn game ports: it carries no
  forbidden key, and a `probe_url` has no explicit port and only the fixed
  path `/v1/probe`.
- `probe_url` hosts are an operator allowlist: Platform publishes only edge
  hosts the operator configured, never a value taken from a request.
- A client refuses to follow redirects on a probe; a 3xx is a failed sample.
- The probe is not a health or admission signal. A 204 says nothing about
  whether a channel is open, full or will admit the player; admission stays
  with the ticket and the NativeLoginGrant.

## Vectors

| case | client outcome |
|---|---|
| `probes-ok` | accept |
| `probes-rejected-route-key` | reject, `ForbiddenGameplayField` |
| `probes-rejected-http-url` | reject, `InvalidPayload` |
| `probes-rejected-query` | reject, `InvalidPayload` |
| `probe-target-204` | accept (one round-trip sample) |
