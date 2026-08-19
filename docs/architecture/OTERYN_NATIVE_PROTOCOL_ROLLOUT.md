# Oteryn native gameplay protocol rollout and rollback

Coordination ID: `OTS-20260804-native-protocol-selection`  
Canonical contract: `../contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md`

## Classification vocabulary

- `server-first-safe`: may deploy before the client while advertisement/activation is disabled.
- `client-first-safe`: may deploy before the producer while the client does not offer/select the feature in production.
- `backward-compatible`: preserves all currently supported behavior and contracts.
- `atomic-required`: activation requires exact producer and consumer revisions together.
- `breaking-migration`: requires coordinated removal or incompatible state conversion.
- `unverified`: not eligible for rollout until evidence exists.

## Staged order

| Stage | Repository/package | Default state | Classification | Merge/activation gate |
|---:|---|---|---|---|
| 0 | canonical contract and three correspondence PRs | documentation only | `backward-compatible` | exact-head docs CI, independent consistency/security PASS |
| 1 | Platform World Registry + Gateway offer/selection + Game Session v2 producer | native candidate disabled | `server-first-safe`, `backward-compatible` | current Gateway v1/Canary regression plus contract fixtures |
| 2 | Otheryn Game Session v2 consumer + native TLS/protobuf producer | listener/candidate disabled | `server-first-safe`, `backward-compatible` | Canary compatibility-profile regression, native golden producer tests, admission-negative tests |
| 3 | Rust `protocol-oteryn` adapter and codec support | not offered by production `Auto` | `client-first-safe`, `backward-compatible` | parser/encoder fixtures, fuzzing, normalized replay, no Tokio/API leakage |
| 4 | automatic selection integration | staging-only exact manifest | `atomic-required` | exact client/Platform/Gateway/Otheryn revisions and schema hash |
| 5 | bounded staging enablement | one controlled world/channel | `atomic-required`, initially `unverified` | real login, bind, snapshot, commands, deltas, disconnect and downgrade-negative E2E |
| 6 | bounded production canary | low percentage or explicit test cohort | `atomic-required` | security/observability/rollback rehearsal and owner authorization |
| 7 | broader native preference | native first, Canary retained | `backward-compatible` for supported clients | measured success/error/latency/resource thresholds |
| 8 | any Canary compatibility-profile removal | not authorized by this programme | `breaking-migration` | separate ADR, population evidence, notice and rollback plan |

## Safe merge order for the contract task

1. merge the canonical `Oteryn/Oteryn-Platform` contract PR;
2. refresh exact canonical revision links in `blakinio/Otheryn` and merge the producer correspondence PR;
3. refresh exact canonical and Otheryn revisions in `blakinio/otclient` and merge the client correspondence PR;
4. archive all three tasks and release ownership.

No runtime order is implied by documentation merge alone.

## Stage 1: Platform/Gateway producer extension

Required disabled-default controls:

- World Registry contains versioned gameplay candidate policy but native entries are disabled;
- Gateway accepts the optional bounded offer while preserving requests without it according to the current Canary contract;
- Gateway never advertises native unless the world policy, Otheryn deployment readiness and Game Session v2 issuer are all true;
- Game Session v2 is produced only for an explicitly selected candidate;
- all current ticket redeem, one-time semantics, response secrecy and private service identity tests remain.

Rollback: disable v2/native policy and retain current Gateway API/Canary behavior. No client change is required.

## Stage 2: Otheryn producer/session enforcement

Required disabled-default controls:

- native TLS listener is separately configured and off by default;
- Game Session v2 validation is available before native advertisement;
- Canary listeners/profiles and ASIO architecture remain unchanged;
- native frames cannot enter Canary parsers and Canary frames cannot enter the native parser;
- listener readiness reports exact native protocol version/schema/capability digest to deployment validation.

Rollback: stop advertising native, drain/close native sessions, disable listener. Never convert an active native session to Canary.

## Stage 3: Rust adapter

Required disabled-default controls:

- `protocol-oteryn` is independent from `protocol-canary`;
- production `Auto` offers only candidates backed by exact compiled support and schema hash;
- before Stage 4, native offer is disabled outside tests/development;
- unsupported Gateway selection fails before connection;
- no password path, protocol sniffing or post-failure fallback is added.

Rollback: remove native from local production offer; retain Canary adapter. Existing active native sessions end normally or are closed; no adapter switch.

## Stage 4/5 integrated staging manifest

The staging artifact must record:

```yaml
coordination_id: OTS-20260804-native-protocol-selection
client_sha: <exact>
platform_sha: <exact>
gateway_image_digest: <exact>
otheryn_sha_or_image_digest: <exact>
game_session_contract_version: 2
gameplay_family: oteryn
native_protocol_version: 1
transport: tcp.tls13.protobuf.be32.v1
schema_revision: 2
schema_sha256: <exact>
world_policy_revision: <exact>
selected_capability_digest: <exact>
fixture_manifest_revision: <exact>
```

Minimum real journeys:

1. OAuth/PKCE -> ticket -> Gateway native selection -> TLS/ALPN -> character bind -> full snapshot.
2. movement prediction -> authoritative reconciliation.
3. attack/follow selection and result lifecycle.
4. spell accepted/rejected/delayed/effect/completed paths.
5. item use/use-with/move and authoritative inventory/container deltas.
6. quick/corpse loot and capacity/ownership rejection.
7. chat and logout.
8. delta gap -> one bounded resync -> fresh snapshot.
9. disconnect -> no replay/resume -> fresh full login/session.
10. every downgrade-negative case in the threat model.

## Enablement rules

Native advertisement is allowed only when:

- exact Gateway and Otheryn readiness describe the same contract/native-protocol-version/schema/capability digest;
- the client build is in the exact supported-pair matrix;
- integrated staging evidence is from the exact candidate revisions;
- error, latency, CPU, memory, frame, command and resync metrics have bounded thresholds;
- rollback has been rehearsed without Canary removal;
- owner authorization exists for the target environment.

## Rollback sequence

1. Disable native candidate advertisement in World Registry/Gateway.
2. Confirm no new native Game Sessions are issued.
3. Let active native sessions drain for the bounded operator window or close them with a typed server notice.
4. Keep native listener available during the drain window, then disable it.
5. Keep Rust native code installed but remove it from production offers if client-side rollback is needed.
6. Preserve all evidence and exact revision manifests.
7. Fresh logins may select explicitly advertised Canary; an active/failed native session never switches adapters.

## Stop/hold conditions

Hold or roll back on any cross-native-protocol-version admission, session replay, credential leakage, unbounded allocation/CPU, incompatible schema selection, state corruption, command double-apply, unexplained revision conflict, material Canary regression, missing observability, failed rollback rehearsal or inability to identify exact deployed revisions.
