# Prompt: complete the native Oteryn protocol with one canonical version

> **STATUS: HISTORICAL_SUPERSEDED ? DO NOT EXECUTE.**
> This prompt predates the canonical Oteryn organization topology. Its `blakinio/*` coordinates are preserved below only as historical provenance, not current write authority. Current product writes must be routed through `Oteryn/Oteryn-Platform`, `Oteryn/Oteryn-Game`, or `Oteryn/Oteryn-Atlas` according to the accepted organization topology and repository-local instructions.

## Role and programme

You are the sole cross-repository coordinator and implementation owner for completing the native Oteryn gameplay protocol programme under:

`OTS-20260804-native-protocol-selection`

Authorized repositories:

- `blakinio/Oteryn-Platform`;
- `blakinio/Otheryn`;
- `blakinio/otclient`.

Task mode:

`IMPLEMENTATION / CROSS-REPOSITORY PROGRAM COMPLETION`

Run autonomously until the authorized programme is implemented, validated, reviewed, merged and archived. Do not stop after producing another plan, ADR, schema draft, partial adapter, isolated listener, test-only mock or status report.

Use one linked task, branch and pull request per repository. Do not share branches, worktrees, task records or path ownership across repositories.

## Owner correction — mandatory

The initial native implementation has exactly one canonical Oteryn gameplay protocol version.

Do not create or retain a native gameplay profile catalogue.

Current identity:

```text
family: oteryn
native_protocol_version: 1
transport: tcp.tls13.protobuf.be32.v1
schema_revision: <current corrected schema revision>
schema_sha256: <exact corrected canonical IDL digest>
capabilities: <exact canonical sorted list and digest>
```

Forbidden in the native v1 path:

- a `profile` request/response field;
- `oteryn.native.v1` as a selectable profile value;
- native profile database columns or tables;
- native profile enums, registries, maps or factories;
- multiple native candidates differentiated only by profile;
- operator or user native-profile selection;
- native profile ordering or fallback;
- aliasing `profile` and `native_protocol_version` indefinitely;
- treating transport, schema or Gateway API versions as a gameplay profile.

Canary compatibility profiles remain unchanged and isolated inside Canary compatibility code. Do not remove, rename or redesign them merely because native Oteryn uses no profile catalogue.

Future native profiles or variants remain possible only through a new reviewed ADR, contract revision, schema/API field and cross-repository migration after a real incompatibility is proven. Native v1 keeps no placeholder profile field.

## Mandatory source material

Before any mutation, read completely and obey the current exact versions of every repository's governing instructions, including root and nearer `AGENTS.md`, overrides, anti-stall, continuation, delivery, GitHub-only and task/ownership policies.

At minimum read:

### Canonical Platform material

- `docs/architecture/adr/0010-native-gameplay-protocol-selection.md`;
- `docs/architecture/adr/0011-single-native-protocol-version.md`;
- `docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md`;
- `docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_SINGLE_VERSION_AMENDMENT.md`;
- `docs/contracts/oteryn_native_gameplay_v1.proto`;
- `docs/architecture/OTERYN_NATIVE_PROTOCOL_THREAT_MODEL.md`;
- `docs/architecture/OTERYN_NATIVE_PROTOCOL_ROLLOUT.md`;
- `docs/operations/OTERYN_NATIVE_PROTOCOL_PRODUCER.md`;
- archived contract and Platform producer task records;
- the exact merged Platform producer implementation and tests.

### Otheryn material

- `docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CORRESPONDENCE.md`;
- `docs/architecture/oteryn-native-gameplay-protocol.md`;
- current Game Session issuer/consumer and admission implementation;
- ASIO service/listener/connection architecture;
- current Canary compatibility profile code;
- authoritative movement, combat, spell, item, loot, chat, state and save seams;
- current build systems, protobuf dependencies, test harnesses and active ownership.

### Rust client material

- `oteryn-client/AGENTS.md`;
- `oteryn-client/docs/architecture/OTERYN_NATIVE_PROTOCOL_CORRESPONDENCE.md`;
- `oteryn-client/docs/architecture/PROTOCOL_BOUNDARY.md`;
- `oteryn-client/docs/architecture/PLATFORM_GATEWAY_GAME_ENTRY.md`;
- `oteryn-client/docs/architecture/DUAL_PROTOCOL_EXECUTION_PLAN.md`;
- the merged Tokio transport implementation and evidence;
- current `protocol-core`, `protocol-canary`, game-domain, game-session, runtime and selection code;
- current active tasks, PRs and leases.

Resolve current main heads, exact merged revisions, live PRs, branches, workflow requirements and deployed-state uncertainty. Do not rely on commit IDs embedded in this prompt.

## Existing completed baseline

Treat these as delivered only after verifying current repository state:

- Oteryn Identity browser OAuth Authorization Code + PKCE;
- one-time Game Login Ticket issue and private atomic redeem;
- World Registry and Go Game Gateway;
- disabled-by-default Platform/Gateway native producer;
- protocol-neutral Tokio transport in the Rust client;
- canonical native contract and Otheryn/Rust correspondence;
- current independent `protocol-canary` implementation.

The existing producer contains a transitional native `profile` dimension. It remains disabled and must be corrected before native enablement.

## Programme objective

Deliver one exact native Oteryn v1 pair from browser login through authoritative gameplay while preserving the existing Identity/Gateway/Game Session authority chain and Canary compatibility:

```text
Rust client
-> system browser
-> Oteryn Identity OAuth + PKCE
-> one-time Game Login Ticket
-> Oteryn Game Gateway
-> authoritative single-version native selection
-> opaque Game Session v2
-> Otheryn TLS native admission
-> full authoritative snapshot
-> semantic commands and authoritative results/deltas
```

At completion:

- Platform/Gateway uses no native profile field or catalogue;
- Otheryn implements Game Session v2 native readiness, storage/admission and gameplay production;
- Rust implements independent `protocol-oteryn` and production-safe automatic family selection;
- one exact cross-repository staging pair passes positive, negative, downgrade and rollback E2E;
- Canary remains independently functional;
- production remains disabled unless explicit environment-specific activation authority exists.

## Phase 0 — linked tasks and exact dependency manifest

1. Inspect all three repositories for overlapping paths, active tasks, open PRs and shared-path leases.
2. Create one linked active task, branch and early draft PR per repository.
3. Use the same coordination ID and record exact cross-repository dependencies.
4. Declare exact owned paths, shared leases, rollout class, validation scope and one concrete `next_action` in each task.
5. Establish a sanitized compatibility manifest with exact contract/schema revisions; no secrets or user identifiers.
6. If an overlapping native implementation already exists, continue or safely supersede it rather than creating parallel abstractions.

Do not mutate runtime until ownership is conflict-free.

## Phase 1 — contract and schema correction

Correct the canonical and correspondence contracts before consumer runtime work.

Required result:

```text
family
native_protocol_version
transport
schema_revision
schema_sha256
capabilities
```

No current native `profile` field.

### Canonical contract

Update the canonical Platform contract, ADR references, threat model, rollout, operations and implementation prompts so `profile` is not a current native dimension.

Preserve distinct:

- Gateway API version;
- offer shape version;
- Game Session contract version;
- native protocol version;
- transport identifier/version;
- schema revision/hash;
- capability list/digest.

### Protobuf

If the current review IDL carries profile identity:

- remove or replace it according to protobuf compatibility rules;
- reserve removed field numbers and names;
- never reinterpret an existing serialized string field as an integer version;
- increment schema revision when IDL bytes change;
- recompute and pin the exact SHA-256;
- regenerate or update every cross-language fixture/provenance record.

### Correspondence

Update Otheryn and Rust correspondence to the exact new canonical Platform merge commit and schema digest before runtime implementation merges.

Contract changes merge in canonical Platform -> Otheryn correspondence -> Rust correspondence order.

## Phase 2 — correct the Platform and Game Gateway producer

Refactor the already merged disabled producer from profile-oriented identity to one native version.

### Data model and migration

Inspect actual current schema, fixtures and environment evidence before deciding migration shape.

Required properties:

- no native profile column/table remains in the final active model;
- native identity uses explicit `native_protocol_version` or an equally unambiguous field;
- uniqueness, policy revision, readiness and session binding use family/version/transport/schema/capabilities/endpoint;
- migration is backward-compatible and reversible where repository policy requires it;
- no destructive assumption is made from repository-default empty candidates;
- legacy no-offer Gateway and Game Session v1 behavior remains unchanged;
- all native rows remain disabled by default;
- no endpoint or candidate is seeded or production-enabled.

Do not keep permanent dual-read/dual-write aliases. A bounded migration bridge is acceptable only when required by proven deployed state and must have a documented removal gate.

### Public and private API

Correct:

- Gateway `gameplay_offer` request validation;
- deterministic selection;
- `gameplay_selection` response;
- World Registry policy projection;
- readiness request/response;
- Game Session v2 issue request and server-side claims;
- canonical JSON fixtures and errors.

For native Oteryn v1:

- at most one native descriptor may be offered;
- version is exactly supported locally and by policy;
- no native preference ordering exists;
- no second native version/profile is attempted;
- no user input invents native versions;
- no post-redeem fallback or retry is introduced.

### Producer tests

Cover at minimum:

- legacy no-offer behavior;
- exact single native descriptor;
- duplicate native family/version rejection;
- unsupported version;
- malformed/unknown/duplicate JSON;
- schema and capability digest mismatch;
- readiness contradiction;
- safe migration and rollback;
- no profile field in public/private JSON, persistence projection, logs or fixtures;
- ticket consumption/no-match/ambiguous issuer semantics;
- secret and identifier redaction.

The corrected producer must remain disabled after merge.

## Phase 3 — implement Otheryn Game Session v2 and native producer

Implement under existing ASIO ownership. Do not replace ASIO with Tokio or another runtime.

### Game Session v2

Implement repository-approved opaque credential storage and atomic admission state with exact claims:

- account and security generation;
- login attempt and session identifiers;
- world/channel/policy/endpoint/audience;
- family `oteryn`;
- `native_protocol_version = 1`;
- transport, schema revision/hash and capability identity;
- bind-on-first-character admission;
- expiry and single-admission state.

No profile claim exists.

Admission must fail closed for replay, expiry, generation mismatch, wrong character/account/world/channel/endpoint/version/transport/schema/capabilities, ambiguous consume/bind and stale session.

### Readiness and listener

Provide exact private readiness for the configured native listener:

- disabled by default;
- separate TLS 1.3 endpoint;
- ALPN `oteryn-game/1`;
- exact family/version/transport/schema/list/digest;
- no native/Canary byte sniffing;
- no profile field;
- no live World Registry dependency during admission.

### Framing and parser

Implement BE32 framing and protobuf with the corrected schema:

- validate length before allocation;
- preserve all size, depth, string, collection and snapshot bounds;
- reject malformed, truncated, oversize, compressed-v1, unknown semantic payload and illegal enum state;
- no panic, busy loop or cursor rewind without progress;
- bounded buffers and deterministic shutdown.

### Authoritative gameplay

Implement semantic commands through existing authoritative server seams:

- movement and stop;
- attack/follow set and clear;
- spells;
- use, use-with and item movement;
- quick/corpse loot;
- chat;
- logout.

Implement command IDs/sequences, duplicate cache, typed lifecycle results, server ordering, full snapshot, strict deltas, movement reconciliation and one bounded resync.

The server owns all gameplay legality, random outcomes, resources, cooldowns, inventory, loot, persistence and state ordering.

### Otheryn validation

Include:

- Game Session v2 concurrency and exactly-once admission;
- cross-binding and replay negatives;
- TLS/ALPN/readiness negatives;
- golden cross-language frames;
- parser malformed/truncated/fuzz regressions;
- action/result and snapshot/delta tests;
- Canary regression suite;
- build entry-point consistency for every added source;
- exact-head CI and independent security audit.

Merge server-first only while listener and advertisement remain disabled.

## Phase 4 — implement the Rust `protocol-oteryn` consumer

Create or complete an independent adapter after corrected Platform and Otheryn producer evidence exists.

### Identity and selection

The client supports:

```text
family = oteryn
native_protocol_version = 1
```

Do not create:

- a native profile enum/catalogue;
- `ForceOteryn(profile)`;
- user-facing profile selection;
- native profile factories/maps;
- native-to-Canary translation.

A development override may restrict the family/version offer only when clearly non-production and cannot force Gateway acceptance.

Production `Auto`:

1. constructs only exact compiled family/version descriptors;
2. preserves Gateway/World Registry authority;
3. validates the exact returned descriptor occurred in the offer;
4. binds one immutable adapter/version to the Game Session;
5. never switches after ticket redeem, selection, credential handoff, admission or protocol failure.

### Transport and adapter

Reuse the merged protocol-neutral Tokio transport.

Implement:

- TLS 1.3 and ALPN verification;
- exact hostname and endpoint validation;
- BE32/protobuf corrected schema codec;
- bounded queues, deadlines, cancellation and joined shutdown;
- semantic `GameCommand` encoding;
- authoritative `GameEvent` decoding;
- command lifecycle and stale-session fencing;
- full snapshot staging and validation;
- strict delta/revision application;
- movement prediction reconciliation;
- no resume or command replay.

Tokio types must not leak into domain, simulation, renderer or UI contracts.

### Rust validation

Cover:

- exact offer/selection validation with no profile field;
- unsupported native version;
- malformed Gateway response;
- TLS/ALPN/schema/capability mismatch;
- golden/malformed/truncated/oversize protobuf frames;
- arbitrary-byte/state-machine fuzz entry points;
- queue saturation and cancellation;
- session replacement and stale commands;
- snapshot/delta/resync;
- action lifecycle;
- exact independent Canary regression;
- workspace format, strict Clippy, tests, architecture policy, supply-chain and independent audit.

Merge client-first only while production native offers remain disabled.

## Phase 5 — automatic integration and exact E2E

After all producer/consumer packages merge and archive, create linked integration tasks/PRs.

### Staging manifest

Persist a sanitized exact manifest:

```yaml
coordination_id: OTS-20260804-native-protocol-selection
contract_commit: <exact>
platform_commit_or_image: <exact>
gateway_image_digest: <exact>
otheryn_commit_or_image: <exact>
otclient_commit_or_artifact: <exact>
gateway_api_version: 1
game_session_contract_version: 2
gameplay_family: oteryn
native_protocol_version: 1
transport: tcp.tls13.protobuf.be32.v1
schema_revision: <exact>
schema_sha256: <exact>
capability_digest: <exact>
world_policy_revision: <exact>
fixture_manifest: <exact>
```

Do not include a gameplay profile entry.

### Required journeys

Prove through the real bounded stack:

- browser OAuth/PKCE;
- one-time Game Login Ticket;
- Gateway single-version native selection;
- Game Session v2 issue;
- TLS/ALPN and atomic character admission;
- complete initial snapshot;
- movement and correction;
- attack/follow;
- spell accepted/rejected/delayed/effect/completed paths;
- item use/use-with/move;
- loot success and failures;
- chat and logout;
- revision gap and bounded replacement snapshot;
- disconnect/relog with fresh ticket/session and no replay;
- explicit Canary journey for a fresh session;
- unsupported version, selected-not-offered, readiness, schema/capability, TLS and replay/cross-binding negatives;
- no post-selection fallback;
- no native profile field in request, response, session, readiness, bootstrap, logs or manifest;
- rollback rehearsal.

### Performance and reliability

Measure bounded selection, handshake, admission, command-to-result/effect latency, snapshot size/chunks, delta rate, queue depth, CPU, memory, allocation, error rates, cancellation and soak behavior.

Declare thresholds before final evidence. Do not weaken them after observing failures without a reviewed rationale.

## Merge and rollout order

Required order:

```text
canonical correction
-> Otheryn correspondence
-> Rust correspondence
-> corrected Platform/Gateway producer, disabled
-> Otheryn v2/native producer, disabled
-> Rust adapter, not offered in production
-> integrated staging E2E
-> bounded activation only with separate explicit environment authority
```

Rollback:

1. disable native advertisement for fresh sessions;
2. increment policy revision where required;
3. verify Gateway selects no native session;
4. drain or close native sessions according to the contract;
5. disable Otheryn native listener;
6. preserve Canary for fresh explicitly allowed sessions;
7. never switch an active/failed native session to Canary.

## Security invariants

- no Oteryn password in the client;
- no OAuth token to Gateway/Otheryn;
- Game Login Ticket goes only to Gateway;
- Game Session credential goes only to the selected endpoint;
- no client authority over account, character, route, policy, endpoint or results;
- no profile fallback or byte-sniffing downgrade;
- no credentials, session/command/account/character identifiers, chat or payloads in logs/artifacts;
- bounded public/private/native inputs;
- exact service authentication and TLS boundaries;
- malformed or ambiguous state fails closed.

## Required independent audit

Use a fresh independent validator after each repository's final implementation head and again for the integrated manifest.

Critical, high or material-medium findings block merge/completion. Resolve every review thread. Validate exact changed paths, contract consistency, migration safety, parser limits, replay/downgrade, authority, redaction, Canary regression, rollout and rollback.

## Completion definition

The programme is complete only when:

- the native profile dimension is removed from active native v1 contracts and runtime paths;
- the Platform producer correction is merged and archived;
- Otheryn v2/native implementation is merged and archived;
- Rust `protocol-oteryn` and selection are merged and archived;
- exact integrated E2E and rollback pass;
- all required CI passes on exact final heads;
- all linked PRs and reviews are terminal;
- ownership and shared leases are released;
- current production enablement state is reported honestly.

Do not claim production completion from documentation, mocks, isolated producer tests or disabled staging code.

## Stop conditions

Stop only for:

- a real unresolved security/contract conflict;
- overlapping ownership that cannot be safely resolved;
- missing required production/staging authorization after every independent READY task is complete;
- missing external dependency that cannot be simulated and blocks the remaining acceptance criteria;
- an exhausted bounded repair path;
- fully completed and archived authorized scope.

Do not stop merely because CI is running, one repository package merged, context is large or another session is required. Persist an exact checkpoint and continue or hand over with one executable next action.

## Final response format

```text
STATUS: DONE | STAGING_COMPLETE | WAITING | BLOCKED | ROTATE
RESULT: <whole-program observable result>
SINGLE_VERSION: <proof that no native profile dimension remains>
PLATFORM: <task, PR, exact head, merge, validation>
OTHERYN: <task, PR, exact head, merge, validation>
RUST_CLIENT: <task, PR, exact head, merge, validation>
E2E: <exact manifest, journeys and performance evidence>
CANARY_REGRESSION: <result>
AUDIT: <independent result and findings>
ROLLBACK: <rehearsal result>
PRODUCTION_ENABLED: true | false
BLOCKER: <none or exact blocker>
NEXT_ACTION: <none or one concrete action>
```
