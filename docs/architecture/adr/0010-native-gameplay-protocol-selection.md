# ADR 0010: Gateway-owned native gameplay protocol selection

- Status: Accepted for contract; runtime unimplemented and disabled
- Date: 2026-08-04
- Coordination: `OTS-20260804-native-protocol-selection`
- Canonical contract: `docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md`

## Context

Oteryn already has a browser OAuth/PKCE, one-time Game Login Ticket, private ticket redeem, World Registry, Game Gateway and Game Session chain. Otheryn already serves profile-driven Canary-compatible gameplay through ASIO. The Rust client is designed for independent `protocol-canary` and future `protocol-oteryn` adapters.

Automatic native selection must not create a second login authority, overload Gateway API `protocol_version`, infer protocol from attacker-controlled gameplay bytes or permit a post-authentication downgrade.

## Decision

1. The Rust client sends a bounded supported-candidate offer in the existing Gateway login request.
2. The Platform World Registry owns the ordered allowed candidate policy per world/channel.
3. Gateway performs the sole final deterministic selection after ticket redeem and binds it to Game Session contract version 2.
4. Otheryn validates and atomically consumes the exact bound session at first character admission.
5. Native v1 uses a separate TLS 1.3 endpoint, ALPN `oteryn-game/1`, big-endian 32-bit length framing and protobuf schema revision 1.
6. Native and Canary are independent adapter families. No wrapping, packet translation or byte-guess fallback is allowed.
7. Native v1 has explicit command IDs, command and stream sequences, action results, authoritative state revisions, initial snapshots, deltas and bounded resynchronization.
8. Native v1 has no session resume, command replay after disconnect or in-session adapter switch.
9. Contract and disabled producers/consumers may merge server-first; enablement requires exact three-repository staging evidence and an atomic rollout decision.

## Consequences

### Positive

- selection stays inside the existing authoritative entry chain;
- downgrade decisions occur once and are session-bound;
- client/server responsibilities are explicit;
- native semantics do not inherit Canary packet constraints;
- server-first deployment is possible while advertisement remains disabled;
- legacy Canary compatibility remains available as an explicit selected candidate.

### Costs

- Game Session producer/consumer contract must advance to version 2;
- World Registry needs versioned candidate policy;
- Otheryn needs a separate native TLS listener and protobuf producer;
- Rust needs a new independent adapter and golden fixtures;
- exact integrated compatibility evidence becomes mandatory for every supported pair.

## Rejected alternatives

- Gateway candidate set followed by client selection: adds state and downgrade ambiguity after ticket consumption.
- game-server wire negotiation: duplicates Gateway/World Registry authority and occurs too late.
- protocol sniffing/fallback: attacker-controlled, ambiguous and not session-bound.
- native-over-Canary translation: preserves the wrong packet constraints and duplicates failure semantics.
- replacing Otheryn ASIO with Tokio: unrelated and unsupported by profiling evidence.

## Rollback

Disable native advertisement in World Registry/Gateway first. Existing native sessions drain or close according to operator policy; they never switch to Canary. Canary remains separately selectable for fresh sessions. Runtime packages must retain feature flags and old contract acceptance until the bounded rollback window closes.
