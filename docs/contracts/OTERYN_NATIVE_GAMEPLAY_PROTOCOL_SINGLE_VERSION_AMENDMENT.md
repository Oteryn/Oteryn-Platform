# Native gameplay protocol single-version amendment

Coordination ID: `OTS-20260804-native-protocol-selection`  
Status: `NORMATIVE — SUPERSEDES THE INITIAL NATIVE PROFILE DIMENSION`  
Decision: [`ADR 0011`](../architecture/adr/0011-single-native-protocol-version.md)  
Status note: incorporated into canonical contract revision 2 and retained as migration rationale  
Canonical schema SHA-256: `9c67f19525400fb9890d2a3541ceb6d02eb955061540ad39ca1c1d891c06eba9`  
Amends: [`OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md`](OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md)

## Purpose

This amendment records the product-owner decision that the first native Oteryn gameplay implementation has one canonical version and does not implement a native profile catalogue.

All authority, authentication, ticket, Game Session, TLS, framing, schema, capability, action, state, downgrade, rollout and rollback rules in the canonical contract remain in force except where this amendment explicitly replaces the native `profile` dimension.

## Normative replacement

Every current native occurrence of:

```text
profile = oteryn.native.v1
native gameplay profile
selected profile
family/profile tuple
cross-.../profile/... binding
transport/profile revision
```

is replaced for the initial native implementation by:

```text
family = oteryn
native_protocol_version = 1
```

The canonical native identity is:

```text
family
native_protocol_version
transport
schema_revision
schema_sha256
selected_capability_digest_sha256
```

The transport identifier remains independently versioned. It is not called a gameplay profile.

## Public Gateway offer and response

The corrected native candidate/descriptor contains:

```json
{
  "family": "oteryn",
  "native_protocol_version": 1,
  "transport": "tcp.tls13.protobuf.be32.v1",
  "schema_revision": 2,
  "schema_sha256": "<64 lowercase hex characters>",
  "capabilities": ["<canonical sorted capability tokens>"]
}
```

The corrected `gameplay_selection` returns the same fields plus the authoritative policy, endpoint and capability digest.

There is no native `profile` field in the corrected request, response, persisted policy, readiness identity or Game Session v2 claim.

Gateway API `protocol_version: 1` remains the Gateway login API version and MUST NOT be reused as `native_protocol_version`.

## Offer and selection cardinality

The cross-family offer may contain independently supported families, including Canary compatibility and native Oteryn.

For the initial native family:

- at most one `family = oteryn` descriptor is allowed;
- its `native_protocol_version` is exactly `1`;
- no ordering or preference exists among native Oteryn alternatives because no second native alternative exists;
- client or operator input cannot invent another native version;
- no user-facing native version/profile chooser exists.

A future native protocol version may be added through a reviewed contract revision. Multiple variants within one version are not represented in the current contract.

## World Registry and persistence

The initial World Registry policy may order protocol families/routes, but it does not own a native profile catalogue.

Corrected native uniqueness and binding use:

```text
world
channel
family
native_protocol_version
transport
schema_revision
schema_sha256
capability identity
endpoint
```

The merged disabled producer currently contains profile-oriented names/storage. Those are transitional implementation details and MUST be migrated before native enablement.

The corrective implementation must inspect actual persisted/deployed state and use a safe migration. It must not assume that default-empty repository fixtures prove every environment contains no rows.

## Game Session v2 and readiness

Game Session v2 claims and Otheryn readiness/bootstrap bind exactly:

```text
family
native_protocol_version
transport
schema revision/hash
capability list/digest
world/channel/policy/endpoint/audience
```

They do not bind a native profile.

The same credential cannot be reused across another family, native protocol version, transport, schema, capability set, world, channel, endpoint, character or connection.

## Protobuf and wire schema

Where the review IDL contains a native profile field, the corrective implementation must remove it or replace it with an explicit native protocol version field according to protobuf compatibility rules.

Removed field numbers/names remain `reserved`. Do not silently reinterpret one serialized field from string profile identity into an integer protocol version.

A new canonical schema revision and SHA-256 are required if the checked-in IDL bytes change.

## Future profiles or variants

No live placeholder field is retained in v1.

A future profile/variant concept may be introduced only by a later ADR and contract revision after a real incompatibility is identified. That later change must define field semantics, migrations, supported pairs, downgrade behavior, rollout, rollback and exact E2E.

Extensibility is preserved by contract and schema evolution, reserved protobuf fields and distinct version dimensions—not by maintaining an unused current profile catalogue.

## Canary compatibility

This amendment does not remove or rename Canary compatibility profiles used by existing Canary/Otheryn compatibility code. It applies only to the new native Oteryn gameplay contract.

`protocol-canary` and `protocol-oteryn` remain independent adapters.

## Activation gate

Native advertisement, Game Session v2 native issuance, Otheryn native admission and Rust production Auto offering remain disabled until:

1. Platform/Gateway profile-oriented runtime/storage is corrected;
2. Otheryn and Rust correspondence adopt this amendment;
3. Otheryn and Rust runtime packages implement the corrected identity;
4. exact cross-repository tests prove there is no profile field or profile selection in the native v1 path;
5. downgrade-negative, rollback and Canary regression tests pass.
