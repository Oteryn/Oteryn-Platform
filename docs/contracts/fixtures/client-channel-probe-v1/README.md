# Client channel probe v1

Byte-exact vectors for `GET /v1/client/channel-probes` (the probe list on the
portal) and `GET /v1/probe` (the latency target on a regional edge). Contract:
`docs/contracts/OTERYN_V2_CLIENT_CHANNEL_PROBE_CONTRACT.md` (ACCEPTED). A
change to any file here is a contract change.

- Consumer: the Game client session (channel picker), pending its
  confirmation. There is no client parser yet, so no Game commit is pinned.
- Platform has no handler yet; the list route is a follow-up to plan task T36b
  and the target is served by the regional edge. Base:
  `Oteryn/Oteryn-Platform@1bb15ad3af64a975dd01da79f5d210f5806d6077`.
- The channel ids are those of `client-directory-v1/directory-anonymous.json`.
  Both channels there are `eu-central`; the two edges are named `probe-eu1`
  and `probe-eu2` so the vector does not contradict that label.
- Manifest digests: `portal-rs/crates/gameauth/src/oauth/oauth_fixture_tests.rs`,
  which also checks each vector against its declared `client_outcome`.

## Cases

`manifest.json` lists the files with byte length and SHA-256 and one entry per
case. `client_outcome` is what a conforming v1 consumer does with the response:
`accept`, or `reject:<error>` with the Game `PlatformClientError` variant.
Files have no trailing newline; `probe-target-204.body` is empty (0 bytes).

| case | request | body | client outcome |
|---|---|---|---|
| probes-ok | `GET /v1/client/channel-probes` | `probes-ok.json` (two channels, two edges) | accept |
| probes-rejected-route-key | `GET /v1/client/channel-probes` | `probes-rejected-route-key.json` (entry has `endpoint`) | reject: `ForbiddenGameplayField` |
| probes-rejected-http-url | `GET /v1/client/channel-probes` | `probes-rejected-http-url.json` (`http://` probe_url) | reject: `InvalidPayload` |
| probes-rejected-query | `GET /v1/client/channel-probes` | `probes-rejected-query.json` (probe_url with `?channel=main`) | reject: `InvalidPayload` |
| probe-target-204 | `GET /v1/probe` on an edge | `probe-target-204.body` (empty), 204, `Cache-Control: no-store` | accept (one round-trip sample) |

Each reject vector is `probes-ok.json` with exactly one defect. They are bodies
a conforming Platform never sends; they pin the consumer's fail-closed
behaviour.
