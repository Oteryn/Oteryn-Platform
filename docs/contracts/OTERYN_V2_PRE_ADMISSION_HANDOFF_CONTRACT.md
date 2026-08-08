# Oteryn-v2 Native Pre-Admission Handoff Contract

## Status

`TARGET PLATFORM SEMANTIC CONTRACT — IMPLEMENTATION / CROSS-REPOSITORY CONSUMER / CUTOVER NOT IMPLIED`

This contract defines the Platform-side semantic boundary for the short-lived native pre-admission material produced for an authenticated, routed attempt to enter Oteryn-v2 gameplay.

It is subordinate to accepted Platform ADR 0031 and must remain compatible with the accepted Oteryn-v2 admission authority. Oteryn-v2 is read-only from this repository.

This contract deliberately does **not** freeze:

- HTTP/RPC/message-bus transport between Platform/Gateway and an issuance service;
- client-to-game admission wire framing or encoding;
- token/JWT/MAC/signature format or key-management primitive;
- exact TTL value;
- Oteryn-v2 replay/consume store;
- character-lease implementation or fencing algorithm;
- canonical `GameSessionId` representation;
- Oteryn-v2 FND-04 admission/session state-machine details.

Those details must come from the accepted producer/consumer authority before implementation or activation.

## Authority model

### Platform owns

Platform owns the authorization decision that an authenticated account may make **one bounded attempt** against one selected native route, subject to current Platform policy and accepted source evidence.

Platform-owned inputs include:

- successful authoritative Game Login Ticket redemption;
- canonical Platform `AccountId`;
- Platform account/security policy current at issuance;
- World Registry `WorldId` / `ChannelId` identity and routing policy;
- Gateway route/protocol selection;
- the accepted Platform projection of current character authorization/ownership evidence;
- the accepted fresh Oteryn-v2 runtime-status/readiness evidence required by the route.

Platform may issue the resulting material through a dedicated Platform authority invoked by Game Gateway. Gateway orchestration does not make Gateway the owner of gameplay-session state.

### Oteryn-v2 owns

Oteryn-v2 remains authoritative for:

- current canonical `AccountId <-> CharacterId` ownership at final admission;
- final character lifecycle/eligibility checks;
- final World/Channel applicability within the game domain;
- current GameNode/admission ownership generation;
- character lease acquisition;
- gameplay lease/fencing semantics;
- duplicate-session policy;
- canonical logical `GameSessionId` issuance;
- admitted-session lifecycle, reconnect and recovery;
- authoritative gameplay start.

The game domain may reject a Platform-authorized attempt after revalidation. Platform authorization is never proof that gameplay admission succeeded.

## Core distinction

```text
Game Login Ticket
  = authorizes Gateway to authenticate one login attempt

Platform native pre-admission material
  = authorizes one bounded attempt to the selected native game admission boundary

canonical game-domain GameSessionId / lease
  = exists only after authoritative Oteryn-v2 admission succeeds
```

These are three distinct lifecycle objects. They must never be reused as aliases for one another.

In particular:

- a Game Login Ticket is consumed at the Platform Identity/Gateway boundary and is never forwarded as the game credential;
- pre-admission material is not a `GameSessionId` and cannot prove lease ownership;
- `GameSessionId` is not issued by Platform/Gateway merely because pre-admission issuance succeeded.

## Canonical identity semantics

Native pre-admission semantics bind the attempt to canonical identities:

```text
AccountId
CharacterId
WorldId
ChannelId
```

Requirements:

- `AccountId` is the canonical Platform identity from ADR 0028/ADR 0031;
- `CharacterId` is the canonical game-owned identity accepted by ADR 0030/ADR 0031;
- `WorldId` and `ChannelId` are canonical Platform topology identities from ADR 0029;
- Canary numeric account/player IDs are forbidden as native authority fields;
- compatibility mappings may exist outside this native contract only inside explicitly named migration/compatibility adapters.

No client-supplied account/character/world/channel claim can establish authority merely by being present in a request.

## Issuance preconditions

Platform may produce native pre-admission material only when all required conditions are true at the issuance decision:

```text
Game Login Ticket redeem succeeded authoritatively
AND Platform Identity remains enabled / security policy satisfied
AND canonical AccountId is unambiguous
AND requested CharacterId is allowed by the accepted current character-authorization projection
AND requested WorldId / ChannelId are valid and policy permits login
AND selected route belongs to that WorldId / ChannelId
AND required Oteryn-v2 runtime observation is fresh, applicable and current-owner evidence
AND selected native protocol/revision/capability tuple is permitted by Platform policy and compatible with the accepted route evidence
AND no required dependency/evidence is UNKNOWN, conflicting, stale or unavailable
```

A Platform projection is sufficient only to decide whether Platform is willing to issue an attempt capability. It is **not** sufficient to establish final game ownership. Oteryn-v2 revalidates authoritative ownership at final admission.

If the character projection is stale enough that Platform cannot safely establish that the authenticated AccountId may attempt that CharacterId, issuance fails closed rather than relying on final game rejection as the normal authorization mechanism.

## Required semantic bindings

The pre-admission object must be semantically bound to enough immutable context that it cannot be replayed for a different admission target.

The accepted cross-repository representation must preserve equivalents of:

- one Platform-generated admission-attempt identity/correlation reference;
- canonical `AccountId`;
- canonical `CharacterId`;
- canonical `WorldId`;
- canonical `ChannelId`;
- selected native gameplay protocol identity/revision/capability identity required for admission compatibility;
- route/admission target identity or immutable route generation sufficient to reject use on another target;
- accepted runtime observation/revision/owner-generation reference where required to prevent stale-route admission;
- issuer identity;
- native game-admission audience;
- issuance time and expiry;
- contract/version identity;
- replay/single-admission state or equivalent anti-replay reference.

This list defines semantics, not field names or wire layout.

### Admission-attempt identity

A Platform-generated attempt identity is for:

- trace correlation;
- issuance idempotency/reconciliation;
- distinguishing retries from independent login attempts;
- security/audit evidence.

It is not a `GameSessionId` and does not become one after admission.

If the final producer API supports retry after an ambiguous issuance response, retry semantics must be keyed to the same attempt identity and must not mint multiple independently usable capabilities for one logical issuance attempt.

## Audience and scope

Pre-admission material is audience-restricted to the accepted Oteryn-v2 native game-admission boundary.

It must not be accepted as:

- an OAuth access/refresh token;
- a Platform web session;
- a Game Login Ticket;
- a generic Platform service credential;
- a credential for Canary compatibility admission;
- a credential for a different World/Channel/route;
- a post-admission gameplay reconnect credential unless the game-domain contract explicitly and separately defines such behavior.

The exact audience string is deferred to the accepted cross-repository encoding contract. The semantic audience is not optional.

## Lifetime and clock authority

Pre-admission material is short-lived.

Rules:

- authoritative server time determines issuance and expiry;
- exact TTL is selected by the accepted implementation contract from measured operational needs;
- client clocks do not extend validity;
- expired material is never revived;
- a route/runtime generation change may invalidate material before nominal expiry when the game-owned admission contract requires it;
- material must not remain valid long enough to function as a reusable account session.

Platform/Gateway must expose only bounded remaining validity needed by the client; it must not encourage caching for later gameplay sessions.

## Replay and single-admission semantics

Security invariant:

> One pre-admission authorization may yield at most one successful authoritative admission transition.

The accepted Oteryn-v2 contract must provide atomic consume or equivalent replay/fencing semantics across concurrent admission attempts.

At the Platform boundary:

- blindly retrying a consumed/ambiguous capability as though it were reusable is forbidden;
- two concurrent uses cannot legitimately create two canonical gameplay sessions from one authorization;
- rejection due to wrong route, wrong character, wrong revision or stale generation must not silently retarget the same material;
- a replayed/used authorization fails closed;
- a new independent login attempt requires fresh pre-admission material.

The exact consume store/algorithm belongs to Oteryn-v2/FND authority.

## Route and runtime-evidence binding

World Registry controls which route Platform may select; Oteryn-v2 controls runtime source facts.

Pre-admission material therefore cannot be valid merely because configured Platform state says `online` or `login_enabled=true`.

Issuance requires the accepted intersection defined by `OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`:

```text
Platform configured policy
AND fresh applicable current-owner Oteryn-v2 runtime evidence
```

The handoff must preserve enough route/revision/ownership-generation context for Oteryn-v2 to reject a capability that reaches:

- the wrong Channel;
- the wrong GameNode/admission owner;
- an obsolete route generation;
- an incompatible protocol/content/runtime revision;
- a target that recovered/restarted into a new ownership generation where prior material is no longer valid.

Exact generation encoding remains external authority.

## Character ownership and lifecycle revalidation

Platform issuance uses an accepted current authorization projection to avoid knowingly issuing attempts for a character the account no longer controls.

Oteryn-v2 final admission nevertheless revalidates authoritative current facts, including as applicable:

- AccountId ↔ CharacterId ownership;
- deletion/transfer/sale/lock lifecycle state;
- selected World/Channel applicability;
- ban/sanction state owned by the game domain or explicitly contracted enforcement policy;
- active/transitioning lease state;
- safe-entry/admission invariants.

A Platform cache hit never overrides a newer game-domain fact.

If a transfer or Character Bazaar ownership change races with issuance, final Oteryn-v2 ownership validation wins and the stale attempt is denied.

## Platform account-security changes after issuance

Ticket redemption and pre-admission issuance must use the current Platform security state at their authoritative commit points.

The final implementation contract must also define what happens if Platform security authority changes **after** pre-admission issuance but before game admission, for example:

- Identity disabled;
- credential compromise response;
- security generation advanced;
- administrative emergency revocation.

Acceptable designs may use very short lifetime, revocation/introspection, generation binding or another reviewed mechanism. This Platform architecture does not choose the primitive.

What is mandatory is an explicit, testable disposition. Silent indefinite validity after security revocation is not acceptable.

## Ambiguous outcome semantics

### Gateway / issuance ambiguity

If Platform cannot determine whether pre-admission issuance committed, it must not blindly create a second independently usable capability for the same attempt.

The final issuer API must choose one safe model:

- idempotent lookup by admission-attempt identity; or
- fail the attempt and require a new Game Login Ticket/login attempt after bounded reconciliation.

### Game admission ambiguity

If the client loses transport after presenting pre-admission material, Platform/Gateway must not infer from transport failure that admission did not occur.

The client must follow the game-owned admitted-session/reconnect/recovery contract if a canonical session may already exist. Replaying the original pre-admission material as a generic reconnect token is forbidden unless Oteryn-v2 explicitly defines a safe idempotent admission-recovery operation.

## Channel switching

A Channel switch crosses the admission boundary again.

Therefore:

- source-session authority does not authorize an arbitrary destination Channel;
- Platform/Gateway obtains fresh destination routing/readiness evidence;
- a fresh destination pre-admission authorization is required;
- Oteryn-v2 performs destination final admission/lease checks;
- accepted Oteryn-v2 architecture determines the fresh canonical `GameSessionId` for the destination transition;
- the old pre-admission material is never retargeted.

## Failure categories

The exact transport/status-code mapping is deferred, but implementations must preserve enough internal typed meaning to distinguish at least:

### Platform issuance failures

- unauthenticated / invalid Game Login Ticket result;
- Identity/security policy invalidated;
- character authorization unavailable/stale/conflicting;
- world/channel unknown or login policy denied;
- runtime evidence missing/stale/unavailable/invalid/superseded;
- no compatible native route/revision;
- pre-admission issuer unavailable;
- ambiguous issuance outcome.

### Oteryn-v2 final-admission failures

Platform expects the accepted game contract to distinguish internally as needed:

- invalid/expired/replayed pre-admission material;
- wrong issuer/audience;
- wrong Account/Character/World/Channel/route;
- stale route/runtime/admission-owner generation;
- incompatible protocol/content/runtime revision;
- authoritative ownership/lifecycle rejection;
- lease/session conflict;
- temporary game admission unavailability.

Public client errors may collapse sensitive distinctions to avoid enumeration. Correlation/operations telemetry retains the bounded internal category without credential material.

## Failure invariants

- no failure falls back to password authentication;
- no failure falls back to direct OAuth authentication by the game server;
- no native failure falls back to Canary compatibility admission implicitly;
- no wrong-route failure causes silent alternate-server retry with the same capability;
- no stale runtime observation is converted into authoritative `offline` state merely to produce an error;
- dependency `UNKNOWN` fails closed for issuance/admission where the dependency is required for safety.

## Logging, observability and privacy

Cross-boundary logging may use:

- admission-attempt/correlation identity;
- bounded service/issuer identity;
- WorldId / ChannelId where operationally appropriate;
- protocol/revision identity;
- typed success/failure category;
- timings;
- pseudonymous AccountId/CharacterId references only where authorized and operationally necessary.

It must not log:

- raw Game Login Ticket;
- raw pre-admission material;
- OAuth access/refresh tokens;
- passwords or hashes;
- private signing/MAC keys;
- MFA/recovery material.

Raw bearer-capability material must be redacted in application logs, traces, proxy logs and error payloads.

## Versioning and compatibility

Keep distinct concepts distinct:

- Platform private API version;
- pre-admission contract/envelope version;
- native gameplay protocol version;
- schema/content/runtime compatibility revisions;
- game-domain admitted-session state-machine revision.

A shared integer named `version` is not sufficient authority for all of them.

Mixed-version rollout must define:

- which Platform producer revision can target which Oteryn-v2 consumer revision;
- how unsupported versions fail closed;
- rollback order;
- whether already-issued material remains valid across a producer/consumer rollback;
- how route/runtime ownership-generation changes invalidate prior material.

Platform must not enable native routing until exact producer/consumer compatibility is proven for the selected revisions.

## Historical Platform Game Session v2 disposition

The retained Platform Game Session v2/native producer package is historical/transitional evidence only.

For reconciliation, each historical claim/field must be classified as one of:

```text
REUSE_PLATFORM_METADATA
MAP_TO_ACCEPTED_NATIVE_SEMANTIC
REJECT_LEGACY_OWNERSHIP
REPLACE_WITH_NEW_ACCEPTED_CONSUMER_FIELD
```

No historical field receives target authority merely because code, protobuf fixtures or validators already exist.

In particular, any historical implication that Platform/Gateway owns final admission, admitted-session identity, lease/fencing or gameplay protocol state is rejected by ADR 0031.

## Required contract proof before implementation/activation

A future implementation package must prove, on exact revisions:

- Platform producer and Oteryn-v2 consumer agree on canonical IDs and authority;
- exact envelope version/schema/encoding is pinned;
- issuer/audience validation works;
- expiry is server-authoritative;
- wrong route/channel/revision/generation fails;
- one authorization cannot produce two successful admissions under concurrency;
- stale character ownership is rejected authoritatively by Oteryn-v2;
- a transfer/sale race cannot authorize the former owner;
- stale runtime/admission-owner generation is rejected;
- security-revocation-after-issuance disposition is tested;
- ambiguous issuance/admission outcomes do not mint/replay duplicate authority;
- Channel switch uses fresh destination authorization;
- reconnect does not misuse pre-admission material;
- logs/traces contain no bearer credentials;
- rollback/mixed-version behavior is deterministic;
- complete native login-to-gameplay E2E succeeds before route activation.

## Current evidence classification

```yaml
platform_semantic_boundary: PROVEN_BY_ACCEPTED_ARCHITECTURE
platform_runtime_implementation: NOT_PROVEN_BY_THIS_CONTRACT
oteryn_v2_consumer_schema: UNKNOWN
oteryn_v2_admission_state_machine: UNKNOWN
oteryn_v2_lease_fencing_implementation: UNKNOWN
canonical_game_session_id_wire_form: UNKNOWN
native_cross_repository_e2e: NOT_PROVEN
staging_activation: NOT_PROVEN
production_activation: NOT_PROVEN
```

## References

- Platform ADR 0031 — native Oteryn-v2 integration boundary
- `docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md`
- `docs/contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md`
- `docs/contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`
- `docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md` — historical/transitional evidence only
- read-only Oteryn-v2 ADR-0003 — Platform Identity, Game Gateway and game admission boundary
