# Oteryn-v2 Character Authority Command / Result Contract

## Status

`ACCEPTED PLATFORM CONSUMER / ORCHESTRATION ARCHITECTURE CONTRACT — OTeryn-v2 TRANSPORT AND RUNTIME IMPLEMENTATION DEFERRED`

This contract defines the semantic boundary that Oteryn Platform must use when orchestrating native character mutations owned by Oteryn-v2 Character Authority.

It does **not** define or authorize the Oteryn-v2 transport, endpoint, IDL, encoding, persistence schema, internal transaction/locking algorithm, command queue implementation, worker framework, deployment, product activation or production cutover.

## Authority

This contract is subordinate to accepted ADR 0030 and ADR 0031.

For the native target:

- Oteryn Platform owns authenticated Account Center/portal UX, Platform authorization, security/business/product gates, orchestration/saga state, notifications, audit and Platform-owned projections/preferences;
- Oteryn-v2 Character Authority owns canonical `CharacterId`, current `AccountId <-> CharacterId` ownership, authoritative character lifecycle/game-state eligibility, mutation execution and authoritative mutation outcome;
- Platform operation rows, caches, public projections and historical Canary rows never prove current ownership or successful native mutation;
- direct/shared SQL, `canary_account_id`, `canary_player_id` and operation-specific Canary write principals remain `Legacy Canary Compatibility` only.

A Platform command authorizes an **attempt to perform one semantic mutation**. It does not transfer game-domain authority to Platform.

## Scope

The shared semantic contract applies to these command profiles:

1. create character;
2. rename character;
3. schedule deletion;
4. cancel/restore deletion;
5. finalize deletion when the accepted native lifecycle requires an explicit command;
6. world/channel transfer only if that product capability is separately adopted;
7. account/Bazaar ownership transfer as a higher-risk profile subordinate to the Bazaar/commercial saga.

The common envelope does not make every listed profile automatically enabled or product-approved.

## Common semantic command envelope

The exact wire field names are deliberately deferred. Every accepted native implementation must preserve the following semantics.

```text
CharacterCommand
  operation_id
  command_family
  command_semantic_version
  authoritative_subject_scope
    AccountId
    CharacterId?                 # absent before create result
    WorldId / ChannelId?         # only when applicable
  intent
  precondition_evidence?         # advisory/optimistic, never final authority
  platform_policy_revision?      # Platform-owned business/security context
  correlation / causation context
```

### `operation_id`

`operation_id` is a stable Platform-issued identifier for **one semantic mutation attempt**.

Rules:

- all retries of the same semantic attempt reuse the same `operation_id`;
- retrying with a new `operation_id` is a new mutation attempt and must not be used merely because a response was lost;
- Oteryn-v2 must deduplicate or otherwise deterministically resolve repeated delivery of the same operation identity;
- the operation identity is scoped strongly enough that one command family/semantic payload cannot collide with another;
- reuse of one `operation_id` with a materially different command family, subject or intent must fail closed as an operation-identity conflict;
- `operation_id` is not an authentication credential and must not be treated as one.

The exact UUID/version/encoding belongs to the eventual producer contract. Stability and uniqueness semantics do not.

### Command family and semantic version

Every command identifies its semantic family and compatible semantic version.

A consumer or producer must not silently reinterpret the same command version with incompatible meaning. Breaking changes require a new semantic version or a separately negotiated compatible contract.

Transport/API versioning and command semantic versioning may evolve independently.

### Canonical subject identity

Native command identity uses accepted canonical identifiers:

- `AccountId` from Platform identity authority;
- game-owned `CharacterId` after character creation;
- canonical Platform-issued `WorldId` / `ChannelId` when topology context is required.

Rules:

- browser-supplied identifiers are request input only and never establish authorization;
- Platform resolves/authorizes the initiating Identity to the applicable canonical AccountId server-side;
- Oteryn-v2 revalidates current AccountId↔CharacterId ownership before executing an owner-scoped mutation;
- Canary numeric IDs may be carried only inside an explicitly named compatibility adapter and never become native authority;
- display name is mutable presentation data, not stable command identity.

### Intent payload

The intent contains only the business/game-domain facts required by the specific command profile.

The intent must not contain a copied character blob, hidden game state, credentials or arbitrary client-controlled server state.

The command profile defines which intent facts are required. Exact wire fields remain deferred.

### Preconditions and revisions

Platform may include known revision/precondition evidence to detect stale requests early, for example known ownership/lifecycle/topology/policy revision.

Such evidence is **optimistic evidence**, not final authority.

Oteryn-v2 must still revalidate current authoritative ownership, lifecycle and game-state eligibility at execution time. A stale Platform snapshot cannot force the game domain to honor an obsolete state.

## Platform pre-submit gates

Before issuing a native character mutation, Platform must establish the Platform-owned gates applicable to that product flow, including:

- authenticated Identity;
- object-level authorization to the intended AccountId/CharacterId relationship using accepted authority, not browser input alone;
- CSRF/rate-limit/recent-MFA policy where the product requires it;
- Platform business/product/entitlement/cooldown policy;
- mutually exclusive Platform workflow state such as Bazaar settlement, another pending service or an already terminal operation;
- canonical topology validation for Platform-owned WorldId/ChannelId policy when applicable.

A Platform-owned business denial should normally stop before sending a game command. It is not evidence that the game domain would also reject the mutation.

## Oteryn-v2 execution authority

For every received command Oteryn-v2 remains responsible for authoritative game-domain revalidation and execution, including as applicable:

- current AccountId↔CharacterId ownership;
- character existence and lifecycle state;
- current gameplay/session state when it affects eligibility;
- game-owned uniqueness/name rules;
- game-owned placement/world membership;
- guild/house/market/mail/depot/quest or other gameplay obligations when the command profile says they affect eligibility;
- concurrency with another mutation of the same character;
- authoritative transaction/locking/fencing needed to make the mutation safe.

Platform must not duplicate these checks by reading stale game persistence and then treat them as authoritative.

## Result / receipt semantics

A command must produce, or later be reconcilable to, a durable game-authoritative operation outcome.

The exact representation is deferred, but the semantic result must bind at least:

```text
CharacterCommandResult
  operation_id
  command_family
  command_semantic_version
  outcome_state
  authoritative_subject_identity
  authoritative_result_facts?     # only fields owned by this command
  authoritative_result_revision?  # comparable/reconcilable producer evidence
  rejection?                      # typed when rejected
  recorded/effective semantics?   # when meaningful to the command profile
```

### Outcome classes

The native producer contract must distinguish these semantic classes.

#### `COMPLETED`

Terminal authoritative result. The mutation represented by the operation identity is durably committed under game authority.

A completed result must contain enough authoritative result facts for Platform to update the user-visible saga and to trigger/reconcile downstream projections without inventing facts.

#### `REJECTED`

Terminal authoritative result for this operation identity: the requested mutation did not complete and the reason is represented by an approved typed rejection.

A rejection must not leak unnecessary private gameplay state.

#### `ACCEPTED_PENDING`

Optional non-terminal result for implementations that legitimately execute asynchronously.

It means the game authority has accepted responsibility for the operation identity, **not** that the mutation completed. Platform remains pending/reconciling until a terminal result is observed.

#### Platform-local `AMBIGUOUS`

`AMBIGUOUS` is a Platform orchestration state, not a game success/failure result.

It is entered when Platform cannot know whether the game authority received/committed the operation, for example timeout, connection loss or response loss.

Platform must reconcile by the same `operation_id`. It must not fabricate completion, rejection or issue a different semantic mutation merely to obtain certainty.

## Typed rejection taxonomy

The final producer contract may use different machine codes, but it must preserve enough structure for Platform to distinguish at least the following semantic categories where applicable:

- `invalid_command` — malformed/unsupported intent or semantic version;
- `authentication_or_service_authority_failed` — producer channel/service authority failure, normally transport/security level rather than a domain mutation result;
- `ownership_mismatch` — current authoritative AccountId↔CharacterId relation does not authorize the mutation;
- `character_not_found_or_not_applicable` — subject no longer exists or the command cannot apply, without leaking private existence where policy forbids it;
- `stale_precondition` — optimistic revision/precondition no longer matches;
- `lifecycle_conflict` — deletion/restore/rename/transfer state conflicts;
- `session_conflict` — current gameplay/admission/session state blocks the operation;
- `name_conflict_or_policy` — game-owned create/rename name rules reject the intent;
- `topology_or_placement_conflict` — destination/source placement is invalid for a transfer profile;
- `operation_identity_conflict` — same operation identity was reused with incompatible semantics;
- `capability_not_supported` — command profile/version is not supported by the target authority;
- `dependency_unavailable` — authoritative dependency cannot safely answer now;
- `retryable_internal_failure` — no terminal success is claimed and retry/reconciliation policy applies;
- `non_retryable_internal_failure` — terminal producer failure class where retrying the same semantic request is not appropriate without operator/product action.

Platform-owned product/business-policy denial is separately represented in Platform orchestration and should not be misreported as a game-domain rejection unless the game contract also owns that policy.

## Idempotency and conflicting reuse

The command boundary assumes at-least-once request delivery is possible.

For one `operation_id`:

- first accepted command establishes the semantic fingerprint for that operation;
- an exact semantic retry returns/reconciles to the same authoritative operation outcome or current non-terminal state;
- duplicate delivery cannot create a second character, second rename, second deletion schedule, second transfer or second ownership transfer;
- a materially different payload under the same operation identity fails closed;
- a retry after ambiguous transport failure reuses the same identity;
- Platform must retain enough local operation metadata to know which semantic request an operation ID represented.

Exactly-once network delivery is **not** assumed.

## Reconciliation contract

Every command implementation must provide an authoritative way to reconcile an operation identity or equivalent durable command state.

The exact query/API/event mechanism is deferred.

Required semantics:

```text
submit operation_id X
  -> COMPLETED                    => terminal success
  -> REJECTED                     => terminal rejection
  -> ACCEPTED_PENDING             => reconcile X until terminal/bounded operational handling
  -> timeout / transport loss     => Platform state AMBIGUOUS
                                     reconcile X
                                     never mint X2 as a blind replacement
```

A reconciliation response that means "operation not found" is not automatically permission to mint a new operation unless the producer contract guarantees that the original operation can no longer appear later. Retrying the **same** operation identity remains the safe default.

Platform may additionally reconcile authoritative current character state, but state comparison alone must not erase the operation receipt when durable operation history is required for audit/business recovery.

## Concurrency and mutual exclusion

No distributed ACID transaction between Platform persistence and Oteryn-v2 persistence is assumed.

The game authority must serialize, fence, version-check or otherwise safely resolve conflicting character mutations under its own persistence model.

At minimum future implementations must prove deterministic behavior for relevant races among:

- rename vs rename;
- rename vs schedule deletion;
- schedule deletion vs restore/cancel;
- finalize deletion vs restore/cancel;
- transfer vs rename/delete/restore;
- ownership transfer/Bazaar settlement vs other character mutations;
- any mutation vs active gameplay/session state when the product contract disallows it.

Platform may add stricter workflow exclusion before submission, but authoritative game-state races are resolved by Oteryn-v2 and returned as a typed result/conflict.

## Command profiles

### 1. Create character

Authoritative mutation owner: Oteryn-v2 Character Authority.

Native request identity:

- canonical `AccountId`;
- no authoritative CharacterId exists yet;
- stable Platform `operation_id`;
- accepted creation intent required by the future native product/game contract.

Oteryn-v2 revalidates account/game creation eligibility and game-owned naming/character rules.

`COMPLETED` must return the newly minted canonical game-owned `CharacterId` plus the authoritative initial public/account facts explicitly owned by the creation result.

Duplicate retry of the same operation cannot create a second character.

Public/account projections update from authoritative result/source projections; a Platform placeholder ID must never become CharacterId.

### 2. Rename character

Authoritative mutation owner: Oteryn-v2 Character Authority.

Request semantics include:

- canonical `AccountId`;
- canonical `CharacterId`;
- stable operation identity;
- requested new name intent;
- optional optimistic current-name/lifecycle revision evidence.

Oteryn-v2 owns final name eligibility/uniqueness and current lifecycle/session eligibility.

`COMPLETED` should expose enough authoritative old/new-name/result revision facts for Platform saga/history reconciliation without making Platform the name registry.

Public search/profile/guild/Bazaar presentation reconciliation follows `OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md`; stale search state cannot reserve or preserve authority over a name.

### 3. Schedule deletion

Authoritative mutation owner: Oteryn-v2 Character Authority.

Request identifies the current AccountId/CharacterId and the stable operation. Platform may carry its accepted user confirmation/business policy revision, but the game owns lifecycle eligibility and actual deletion state.

`COMPLETED` must expose the authoritative resulting lifecycle state and effective/grace/finalization semantics only if the native lifecycle contract actually supports them.

Platform must not invent a deletion deadline from a compatibility-era Canary timestamp rule.

Public/account presentation updates only after authoritative lifecycle/result evidence and under the projection/privacy contracts.

### 4. Cancel / restore deletion

Authoritative mutation owner: Oteryn-v2 Character Authority.

The command reuses canonical AccountId/CharacterId and a new stable operation identity for the restore/cancel semantic attempt. It may reference the prior schedule operation when useful, but current authoritative lifecycle state controls.

`COMPLETED` establishes the authoritative restored/cancelled lifecycle outcome. Repeated delivery cannot perform multiple transitions.

A restore request racing finalization must return a deterministic terminal or reconcilable outcome; Platform cannot assume the earlier UI-visible grace state is still current.

### 5. Finalize deletion

This profile exists only if the accepted native lifecycle makes finalization an explicit external command.

The common contract does **not** require Platform to drive deletion finalization. Oteryn-v2 may own automatic finalization internally.

If an explicit command exists, Oteryn-v2 must revalidate that finalization is currently allowed and must serialize it against restore/cancel and other character operations. `COMPLETED` must provide an authoritative tombstone/finalization result suitable for Platform cleanup/reconciliation.

### 6. World / channel transfer

This profile is **capability-gated** by Issue #320 and a separate product decision. Its presence here does not approve world transfer.

When adopted:

- Platform supplies canonical source/destination `WorldId`/`ChannelId` policy context as applicable;
- Oteryn-v2 owns current authoritative placement, transfer eligibility and mutation;
- Platform topology projections do not prove the character can be moved;
- Oteryn-v2 revalidates game-owned houses/guilds/market/depot/quest/world-specific obligations defined by the future transfer product contract;
- `COMPLETED` returns authoritative resulting placement using canonical identifiers;
- timeout after potential commit enters reconciliation by operation identity.

No Canary world column or numeric channel ID defines native capability semantics.

### 7. Account / Bazaar ownership transfer

This is a higher-risk profile subordinate to the Character Bazaar/commercial saga.

Platform owns commercial eligibility, listing/auction/buy-now workflow, wallet/settlement policy and the decision to request ownership transfer only after the commercial saga reaches its permitted handoff point.

Oteryn-v2 owns the authoritative CharacterId ownership mutation.

The command must bind:

- stable Platform operation identity;
- canonical `CharacterId`;
- expected current source `AccountId`;
- canonical destination `AccountId` authorized by the completed/eligible Platform commercial flow;
- accepted correlation to the Platform commercial operation without exposing secrets/payment payloads.

Oteryn-v2 revalidates current ownership and game-state transfer eligibility.

`COMPLETED` returns authoritative new ownership facts/receipt. Platform then advances the commercial saga from authoritative game result; it must not treat wallet settlement alone as proof that game ownership moved.

A timeout between commercial settlement and game result is a recovery state, not permission for a second blind transfer or compensating direct SQL write.

## Platform saga state

Platform may persist bounded orchestration state such as:

- immutable Platform operation/public identifier;
- authenticated Identity reference and canonical AccountId;
- canonical CharacterId when it exists;
- command family/version and a semantic request fingerprint;
- Platform policy/business revision;
- pending/completed/rejected/recovery-required state;
- authoritative receipt/result reference and bounded typed failure category;
- timestamps only where they are Platform-owned or supplied by authoritative result;
- reconciliation counters/metadata appropriate to operations policy;
- privacy-safe audit and notification metadata.

Such state never becomes game-domain ownership/lifecycle authority.

## Downstream projections and privacy

Character mutations frequently affect public/account read models. The command channel does not create a second projection pipeline.

Rules:

- user-visible Platform saga state may update directly from the authoritative command result;
- public character/search/ranking/activity/guild/presence state reconciles through `OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md` or another accepted source projection owned by that domain;
- Platform may use the authoritative command result as bounded reconciliation evidence where the projection contract allows it, but it must not fabricate unrelated public game facts;
- restrictive Platform privacy decisions remain an independent upper bound over fresh game facts;
- a rename/delete/restore/transfer result must trigger or eventually reconcile every affected Platform index/cache/search surface by stable identity;
- stale projections cannot reverse a newer authoritative mutation or a newer privacy deny.

## Security boundary

The future transport must provide service-to-service authentication and authorization appropriate to a privileged mutation channel.

At minimum implementation must define and test:

- authenticated producer/consumer identity;
- audience/scope restriction;
- replay/deduplication behavior in addition to operation idempotency;
- request size/schema validation;
- command semantic-version validation;
- authorization failure before privileged mutation;
- secret/token redaction;
- correlation/audit without full sensitive payload logging;
- rate/abuse controls appropriate to the caller and command family.

This contract deliberately does not choose mTLS, signed tokens, message signatures or another primitive.

## Observability

Future implementations should expose bounded metrics/traces for:

- command family and semantic version;
- accepted/completed/rejected/pending outcomes;
- operation duplicate/replay/conflicting-reuse counts;
- reconciliation age and ambiguous-operation backlog;
- typed conflict/rejection classes;
- projection reconciliation lag after completed mutations.

Logs must not contain credentials, bearer tokens, voucher/payment payloads or unnecessary private gameplay state.

## Legacy Canary Compatibility

Existing Canary mutation/read contracts remain useful only inside explicitly named compatibility or migration tasks.

Rules:

- Canary direct SQL cannot implement this native semantic contract by implication;
- `canary_account_id`, `canary_player_id` and Canary numeric world/channel IDs remain adapter details;
- a compatibility path must name the remaining consumer, exact privileges, rollback and removal gate;
- do not run native and legacy mutation paths concurrently for the same semantic operation without an explicit cutover/fencing design;
- legacy fallback after a native command becomes ambiguous is forbidden because it could create a second authority/mutation;
- migration must prove native producer/consumer idempotency and reconciliation before disabling the legacy operation.

## Versioning and compatibility

Breaking semantic changes require explicit contract revision when changing any of:

- ownership/authority split;
- canonical identity meaning;
- operation identity/idempotency semantics;
- terminal outcome meaning;
- reconciliation guarantees;
- conflicting-reuse behavior;
- typed conflict classes required by Platform recovery;
- per-command authoritative result facts.

Wire evolution is allowed when these semantics remain compatible.

## Rollout and rollback

Native mutation rollout is per command family, not one global flag.

Before one family cuts over:

1. accepted Oteryn-v2 producer/consumer semantic mapping exists;
2. canonical identity mapping is proven;
3. stable operation identity and duplicate/conflicting-reuse tests pass;
4. typed terminal/ambiguous/reconciliation behavior passes;
5. cross-command concurrency cases pass;
6. Platform saga and downstream projection reconciliation pass;
7. legacy consumer inventory/removal gate is known;
8. rollback does not require replaying an uncertain mutation through a second authority.

Rollback may route **new** operations back to a proven compatibility path only when the cutover design says that is safe. An already submitted native operation must first reach a reconciled terminal state or an explicitly proven cancellation/fencing state.

## Validation requirements for later runtime implementation

At minimum every implemented command family must prove on exact producer/consumer revisions:

1. server-side owner authorization and foreign-object denial;
2. canonical identity validation;
3. duplicate same-operation replay produces one mutation/result;
4. same operation identity with changed intent fails closed;
5. timeout before receive, during execution and after commit all reconcile deterministically;
6. stale ownership/lifecycle precondition is rejected or reconciled safely;
7. relevant concurrent command races produce one coherent authoritative state;
8. gameplay/session conflicts behave according to the command profile;
9. Platform saga never advances from ambiguous transport state as if completed;
10. downstream projection/search/cache reconciliation follows the accepted projection/privacy contracts;
11. rollback/cutover cannot double-apply one semantic operation;
12. logs/audit omit secrets and unnecessary private state.

User-facing implementations additionally require the product-specific EN/PL and zero-retry browser/E2E evidence owned by Issues #317/#319/#320 or their successors.

## Deferred details

Still intentionally `UNKNOWN` / external authority:

- exact Oteryn-v2 service/module name for Character Authority;
- exact transport/endpoint/IDL/serialization;
- exact operation identifier encoding;
- exact result persistence/retention mechanism;
- exact query/event used for reconciliation;
- exact game-internal transaction, locking and serialization strategy;
- exact name/lifecycle/world-transfer product policies;
- exact automatic-vs-explicit deletion finalization behavior;
- exact world-transfer capability adoption;
- exact staging/production rollout topology.

These unknowns do not allow Platform to fail open, invent game state or fall back to direct native game-table mutation.

## Non-authorization

This contract authorizes no:

- Laravel runtime implementation;
- Platform database migration;
- Oteryn-v2 or Canary repository write;
- command endpoint or message-broker deployment;
- deletion/rename/world-transfer product activation;
- Character Bazaar ownership-transfer cutover;
- payment/entitlement activation;
- staging or production mutation.

## References

- ADR 0030 — Native Character Portfolio / Account Center v2
- ADR 0031 — Native Oteryn-v2 Integration vs Legacy Canary Compatibility
- `docs/architecture/character-lifecycle/NATIVE_CHARACTER_LIFECYCLE_AUTHORITY.md`
- `docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md`
- Issue #919 — focused shared command/result architecture owner
- Issues #317, #319, #320 — product-specific lifecycle consumers
