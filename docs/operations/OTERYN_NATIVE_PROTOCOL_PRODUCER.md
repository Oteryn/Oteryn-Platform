# Oteryn native protocol producer — historical transitional operations record

## Status

`HISTORICAL PRODUCER COMPLETE — DISABLED BY DEFAULT — RECONCILIATION INPUT ONLY`

This document preserves the Platform/Game Gateway producer package delivered under historical coordination ID `OTS-20260804-native-protocol-selection`.

It is **not** current Oteryn-v2 rollout guidance and does not claim native gameplay availability, Oteryn-v2 consumer compatibility, staging readiness or production activation.

Current target authority is ADR 0031 plus `docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md`. Oteryn-v2 owns final gameplay admission, authoritative gameplay-session/lease/fencing semantics and `protocol-oteryn` gameplay semantics. Platform owns Identity, ticketing, World Registry and bounded Gateway pre-admission/control-plane behavior.

## Historical producer boundary

The merged Platform/Gateway package retained the existing Gateway API version 1 and added an optional bounded gameplay offer/selection path. It could:

1. validate request syntax before ticket redeem;
2. redeem the one-time Game Login Ticket;
3. obtain authoritative Platform world/character/policy context available to that historical implementation;
4. select one exact policy candidate offered by the client;
5. verify a selected endpoint readiness identity;
6. issue one opaque historical Game Session contract version 2 request;
7. return immutable selection metadata and a sanitized endpoint.

The producer was delivered disabled by default. No current authority should infer from this record that the historical Game Session v2 request, Otheryn readiness shape, account identifier shape or admission semantics are accepted by Oteryn-v2.

## Historical native identity

The retained artifact is pinned to:

```text
family: oteryn
native_protocol_version: 1
transport: tcp.tls13.protobuf.be32.v1
schema_revision: 2
schema_sha256: 9c67f19525400fb9890d2a3541ceb6d02eb955061540ad39ca1c1d891c06eba9
channel_id: 1
```

The SHA-256 value is the historical digest of `docs/contracts/oteryn_native_gameplay_v1.proto`. It remains pinned so repository validators can prove integrity of the old producer package. It is not a requirement that Oteryn-v2 adopt these exact bytes.

## Disabled-by-default invariant

The historical migration created no enabled native candidate rows and defaulted every new `game_world_protocol_candidates.enabled` value to `false`.

That fail-closed state remains the only safe interpretation of the package. Do not enable or advertise this historical producer merely because the repository artifact validates.

A future native-v2 rollout requires, at minimum:

- an exact accepted Oteryn-v2 admission/session/protocol contract;
- explicit mapping of Platform pre-admission fields to that contract;
- exact producer/consumer compatibility evidence;
- a Rust client that implements the accepted Oteryn-v2 `protocol-oteryn` contract;
- current World Registry/readiness semantics reconciled with the accepted consumer;
- cross-repository authorization/admission/gameplay E2E on exact revisions;
- deployment/network/TLS/secret evidence for the selected environment;
- rollback evidence and separate production activation authority.

If any of those is unavailable, native-v2 activation remains blocked. Missing fields or semantics must stay `UNKNOWN`; this historical Otheryn package cannot fill them by analogy.

## Platform facts that may remain reusable

Subject to exact Oteryn-v2 reconciliation, the following Platform-owned concepts remain architecturally valid under ADR 0031:

- Identity authenticates the user and issues one-time Game Login Tickets;
- Gateway redeems the ticket using service-authenticated Platform Identity infrastructure;
- World Registry owns Platform world/channel identity, routing policy and pre-admission selection policy;
- Gateway may carry bounded protocol/capability/endpoint selection metadata needed before connection;
- selection and ticket/session failures fail closed rather than silently downgrading to another gameplay family;
- secrets and bearer credentials are externally injected and redacted;
- production advertisement/activation is a separate gate.

These are Platform-side ownership principles. They do not assign gameplay packet semantics or authoritative admitted-session state to Platform.

## Historical Otheryn correspondence

Earlier revisions of this document described an Otheryn Game Session v2 issuer/readiness endpoint and native listener as the next consumer package. That statement is historical only.

The current native target is Oteryn-v2. Otheryn/Canary-era correspondence can be used as compatibility or migration evidence, but it must not be treated as the target consumer contract or as a prerequisite that authorizes Oteryn-v2 production.

## Reconciliation procedure

Before changing or enabling the producer:

1. read `docs/architecture/ARCHITECTURE_AUTHORITY.md`;
2. apply ADR 0031 and `OTERYN_V2_INTEGRATION_ARCHITECTURE.md`;
3. obtain the exact accepted Oteryn-v2 consumer/admission authority as read-only evidence;
4. compare every retained historical field and transition against that authority;
5. create a separately governed Platform implementation task for any required producer changes;
6. keep Legacy Canary Compatibility and native-v2 paths explicitly separated;
7. validate on exact heads and run the required cross-repository E2E;
8. activate only under a separately authorized environment gate.

Do not modify Oteryn-v2 from this Platform task unless the owner separately authorizes that repository.

## Historical rollback evidence

The old package remains deploy-first-safe because native candidates stay disabled unless explicitly configured. Historical rollback concepts — disabling advertisement, advancing policy revision, stopping new issuance and preserving database compatibility — are evidence about the old producer, not a complete native-v2 rollback runbook.

Any future rollout must define rollback against the accepted Oteryn-v2 contract and exact deployed revisions.

## Current production position

Repository validation of this historical producer is **not** production proof. Current production/native verification remains governed by `docs/agents/tasks/active/OTERYN-20260805-native-auth-production-verification.md` and the applicable deployment/production gates.

No production canary, native advertisement, listener activation, secret mutation, deployment or cross-repository write is authorized by this document.

## References

- ADR 0031 — `docs/architecture/adr/0031-native-oteryn-v2-integration-boundary.md`
- `docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md`
- historical contract — `docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md`
- historical schema — `docs/contracts/oteryn_native_gameplay_v1.proto`
- historical Platform producer — PR #542
- active production verification — `docs/agents/tasks/active/OTERYN-20260805-native-auth-production-verification.md`
