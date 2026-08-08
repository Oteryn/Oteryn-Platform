# Oteryn Native Gameplay Protocol Contract — historical transitional package

## Status

`HISTORICAL / TRANSITIONAL RECONCILIATION INPUT — NOT CURRENT NATIVE AUTHORITY`

Historical contract revision: `2`  
Historical schema revision: `2`  
Canonical schema SHA-256: `9c67f19525400fb9890d2a3541ceb6d02eb955061540ad39ca1c1d891c06eba9`  
Historical coordination ID: `OTS-20260804-native-protocol-selection`

This file preserves the repository contract that accompanied the disabled-by-default Platform/Game Gateway native producer delivered before ADR 0031. It is retained so the merged producer, protobuf schema, fixtures, validators and earlier cross-repository correspondence remain reproducible and reviewable.

It is **not** the current architecture authority for Oteryn-v2 gameplay protocol semantics, final gameplay admission, admitted-session state, lease/fencing, reconnect semantics or game-owned persistence.

## Current authority

Apply the repository architecture authority order before using this historical package:

1. `docs/architecture/adr/0031-native-oteryn-v2-integration-boundary.md` is the accepted durable decision for the Platform native-v2 versus Legacy Canary Compatibility boundary.
2. `docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md` is the focused current Platform-side target architecture.
3. Oteryn-v2 owns authoritative gameplay admission/session/lease/fencing and `protocol-oteryn` gameplay semantics. Exact Oteryn-v2 contract bytes and unfinished admission/session details must come from the accepted Oteryn-v2 authority, not from this file.
4. Platform owns Identity, OAuth/PKCE, one-time Game Login Ticket lifecycle, World Registry policy/routing and Game Gateway pre-admission orchestration within ADR 0031.
5. Current Canary-compatible Game Session v1 remains separately governed by `GAME_SESSION_CANARY_CONTRACT.md` for its declared compatibility scope.

ADR 0031 supersedes the former target ownership model in this document wherever this file assigned gameplay protocol semantics, final admission or authoritative admitted-session behavior to Platform/Gateway or to the historical Otheryn correspondence package.

## What this file still proves

This historical record proves only that the Platform repository once coordinated and validated a disabled producer package with an exact native tuple, schema, fixtures and fail-closed selection constraints. It does **not** prove:

- an Oteryn-v2 native consumer exists for these exact historical bytes;
- Oteryn-v2 accepted these Game Session v2 claim shapes;
- the historical `game_account_id`, admission-state or bind-on-first-admission model is the current native contract;
- the historical parser, reconnect, state-reconciliation or command semantics are current Oteryn-v2 authority;
- staging or production native activation;
- permission to enable the historical producer.

Platform PR #542 and its associated artifacts remain transitional producer evidence only. Any future implementation must reconcile the still-useful Platform pre-admission pieces against the accepted Oteryn-v2 contract instead of treating this historical package as producer authority for the game domain.

## Preserved historical identity

The disabled historical producer was built around exactly one native descriptor:

```text
family = oteryn
native_protocol_version = 1
transport = tcp.tls13.protobuf.be32.v1
schema_revision = 2
schema_sha256 = 9c67f19525400fb9890d2a3541ceb6d02eb955061540ad39ca1c1d891c06eba9
```

The corresponding schema remains at `docs/contracts/oteryn_native_gameplay_v1.proto` and the deterministic fixtures remain under `docs/contracts/fixtures/oteryn-native-v1/`.

These values are historical package identity, not a declaration that Oteryn-v2 must adopt them unchanged.

## Preserved single-version invariants

The following text is intentionally retained because repository validators use it to prove that the historical package did not reintroduce the removed profile catalogue or multiple native descriptors. These are **historical artifact invariants**, not current Oteryn-v2 architecture authority:

- At most one candidate may have `family = oteryn`; when present, its `native_protocol_version` is exactly `1`.
- It contains no `profile` key, alias or placeholder.
- The current contract has no placeholder for that future work.
- `protocol-canary` and `protocol-oteryn` were separate historical adapter families; no profile-shaped alias joined them.

Future Oteryn-v2 protocol evolution follows Oteryn-v2-owned authority plus an explicit Platform consumer contract where Platform must carry selection metadata. It does not extend this historical file.

## Preserved historical parser/security bounds

Repository audits also retain the former package's exact defensive bounds so the old schema/fixture evidence remains internally checkable:

- TLS 1.3 and ALPN `oteryn-game/1` were required by the historical native listener design;
- framing used an unsigned 32-bit big-endian length prefix;
- ordinary frame limit: `1,048,576` bytes;
- historical snapshot chunk limit: `512 KiB`;
- historical complete initial snapshot limit: `16 MiB`;
- historical ClientHello deadline: `5 seconds` after TLS completion;
- historical selection/admission behavior was fail closed, single admission, replay resistant and terminal after a binding/transport failure;
- the package used `bind_on_first_admission`, allowed no second candidate after selection, required no plaintext or cross-family fallback, and required credential/identifier redaction;
- production canary enablement required separate owner authorization.

These bounds remain useful regression evidence for the bytes that the repository already produced. They must not be copied into Oteryn-v2 as current protocol requirements unless the Oteryn-v2 authority independently accepts them.

## Historical artifact validation

The existing `scripts/validate_native_protocol_contract.py` and `native-protocol-contract*` workflows continue to validate integrity and fail-closed properties of this retained historical package. Their successful result means only that the historical Platform artifact remains self-consistent with its protobuf schema and fixtures. It is not Oteryn-v2 conformance evidence and does not elevate this file above ADR 0031.

## Migration / reconciliation rule

Before any native-v2 Platform producer is enabled or treated as ready:

1. identify the exact accepted Oteryn-v2 admission/session/protocol contract revision;
2. compare the Platform pre-admission producer fields and semantics with that authority;
3. classify every historical field as reusable Platform metadata, mapped compatibility data, rejected legacy ownership, or newly required consumer data;
4. implement only separately accepted Platform-side changes;
5. prove producer/consumer compatibility on exact revisions;
6. retain rollback and keep Legacy Canary Compatibility explicitly separated;
7. perform staging/E2E and production activation only under their separate authorization gates.

Missing Oteryn-v2 contract evidence remains `UNKNOWN`; it is never filled from this historical document by analogy.

## Historical references

- coordination ID `OTS-20260804-native-protocol-selection`;
- Platform PR #542 — disabled-by-default producer package;
- `docs/contracts/oteryn_native_gameplay_v1.proto`;
- `docs/contracts/fixtures/oteryn-native-v1/`;
- `docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_SINGLE_VERSION_AMENDMENT.md`;
- `docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_MIGRATION.md`;
- `docs/architecture/OTERYN_NATIVE_PROTOCOL_THREAT_MODEL.md`;
- `docs/architecture/OTERYN_NATIVE_PROTOCOL_ROLLOUT.md`;
- ADR 0010 / ADR 0011 — historical decisions superseded by ADR 0031 for target native protocol ownership/family-selection semantics.

For current work, start from `docs/architecture/ARCHITECTURE_AUTHORITY.md`, ADR 0031 and `docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md`.