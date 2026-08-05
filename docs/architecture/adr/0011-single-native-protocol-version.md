# ADR 0011: One native protocol version without a profile catalogue

- Status: Accepted
- Date: 2026-08-05
- Coordination ID: `OTS-20260804-native-protocol-selection`
- Supersedes: the current native-profile dimension in ADR 0010 and contract revision 2
- Does not supersede: Canary compatibility profiles, login/API versions, transport/schema versioning, authority, downgrade or rollout rules

## Context

The initial cross-repository contract introduced a `profile` field with the concrete value `oteryn.native.v1`. The disabled-by-default Platform/Game Gateway producer subsequently implemented that field.

The product direction is simpler: the first native Oteryn client and server have one canonical native gameplay protocol. There is no current product need for multiple native profiles, a profile catalogue, profile ordering, a user-facing profile choice or profile-specific rollout policy.

Keeping a live profile dimension now would create configuration, migrations, validation, compatibility combinations and downgrade cases that provide no current product value. It would also make the word “profile” easy to confuse with player, character or game settings.

## Decision

The initial native Oteryn gameplay implementation SHALL expose exactly one canonical native protocol version.

Current identity:

```text
family: oteryn
native_protocol_version: 1
transport: tcp.tls13.protobuf.be32.v1
schema_revision: 2
schema_sha256: 9c67f19525400fb9890d2a3541ceb6d02eb955061540ad39ca1c1d891c06eba9
capabilities: <exact canonical sorted list and digest>
```

There is no current native `profile` field, profile identifier, profile table, profile enum, profile selector, profile preference order or user-facing chooser.

The following concepts remain distinct:

- Gateway login API version;
- gameplay-offer shape version;
- Game Session contract version;
- adapter family;
- native protocol version;
- transport identifier/version;
- schema revision and hash;
- capability list and digest.

Selection may still choose between independently supported protocol families such as Canary compatibility and native Oteryn. For native Oteryn v1, however, `family = oteryn` plus `native_protocol_version = 1` identifies the sole native gameplay contract. No second native alternative exists in the current offer or World Registry policy.

## Transitional producer state

The already merged Platform/Game Gateway producer is disabled by default, seeds no candidate and contains a transitional `profile = oteryn.native.v1` dimension.

That producer MUST NOT be enabled for native gameplay until a corrective implementation package:

1. replaces current native `profile` fields and storage with `native_protocol_version` or an equally explicit single-version field;
2. updates Gateway offer/selection, World Registry policy, Game Session v2 claims, readiness and tests consistently;
3. updates Otheryn and Rust correspondence before their runtime implementations merge;
4. proves no legacy Gateway v1 or Canary-compatible behavior regresses;
5. passes exact cross-repository compatibility and downgrade-negative evidence.

Because the producer is disabled and initially stores no candidate, this correction should occur before consumer activation. Implementations must still inspect real persisted and deployed state before choosing a migration strategy; repository defaults are not proof that every environment has no rows.

## Future extension

Future incompatible native variants remain possible, but v1 does not reserve or expose a live placeholder profile field.

A future need for multiple variants requires all of:

1. a new reviewed ADR and canonical contract revision;
2. a concrete incompatibility that cannot be represented by native protocol version, transport version, schema revision or capabilities;
3. an explicit new field and semantics introduced at that later revision;
4. migrations and backward-compatibility behavior across Platform, Gateway, Otheryn and Rust;
5. supported-pair, downgrade, rollout, rollback and E2E evidence.

Protobuf field numbers removed during the correction remain `reserved`. JSON and database fields are introduced later only when the need is real. This preserves extensibility without carrying unused complexity in the initial implementation.

## Canary boundary

Existing Canary-compatible server/client profiles are legacy compatibility implementation details and are not removed by this ADR. They must remain isolated inside Canary compatibility boundaries and must not be copied into the native Oteryn design.

## Consequences

Positive:

- fewer protocol combinations and downgrade surfaces;
- simpler World Registry, Gateway and Game Session state;
- clearer language for agents and operators;
- easier exact-pair testing and rollout;
- future variants remain possible through deliberate contract evolution.

Costs:

- the disabled producer must be corrected before activation;
- the canonical contract, implementation prompts and correspondence documents need synchronized amendments;
- any code already using `profile` must migrate consistently rather than aliasing two names indefinitely.

## Rejected alternatives

### Keep `profile` but configure only one row

Rejected because it preserves the unnecessary public/storage dimension and invites agents to create additional profiles prematurely.

### Rename `profile` to `variant` now

Rejected because it still creates an unused dimension. A future variant field belongs to the future contract revision that needs it.

### Use one generic version number for API, session, transport and gameplay

Rejected because these contracts evolve independently and one field would become ambiguous.

### Remove all versioning

Rejected because exact compatibility, rollout and security binding require explicit versions and immutable schema/capability identity.
