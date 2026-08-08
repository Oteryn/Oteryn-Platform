# OTERYN-20260808 — Native Character Authority Command / Result Architecture Review

## Result

`PASS AFTER REVIEW REPAIR — SHARED NATIVE CHARACTER MUTATION SEMANTICS DEFINED WITHOUT RUNTIME AUTHORITY EXPANSION`

Issue #919 closes one architecture gap left after ADR 0030/0031 and the character-lifecycle authority reconciliation: product issues #317, #319 and conditional #320 no longer need to invent independent cross-system operation, idempotency, typed-result and reconciliation semantics.

## Evidence reviewed

- accepted ADR 0030 and ADR 0031 authority split;
- `docs/architecture/character-lifecycle/NATIVE_CHARACTER_LIFECYCLE_AUTHORITY.md`;
- open product issues #277, #317, #319 and #320;
- `docs/contracts/CHARACTER_TRANSFER_CONTRACT.md` accepted Bazaar ordering (`winner transfer -> wallet settlement -> completed`);
- accepted native PublicGameData projection contract and its later privacy-revocation strengthening;
- current main state and open PR/branch ownership for native Character Authority command/result work;
- PR #920 inline review findings.

No other open PR or active task was found owning this exact focused shared contract.

## Accepted design

### One reusable semantic command boundary

All native character mutations share these invariants:

- Platform issues one stable `operation_id` for one semantic mutation attempt;
- retries reuse that same operation identity;
- duplicate delivery cannot create a second authoritative mutation;
- conflicting reuse of the same operation identity with different semantics fails closed;
- Oteryn-v2 revalidates current ownership/lifecycle/game-state authority at execution time;
- Platform preconditions/projections are optimistic evidence only;
- outcomes are typed and durably reconcilable;
- timeout/transport loss becomes Platform-local `AMBIGUOUS`, never fabricated success/rejection;
- no distributed ACID between Platform and game persistence is assumed;
- no fallback from ambiguous native mutation to direct Canary/native SQL is allowed.

### Outcome semantics

The repaired contract distinguishes:

- `COMPLETED` — terminal authoritative mutation result;
- `REJECTED` — terminal authoritative rejection; the producer guarantees that operation identity cannot later commit;
- `ACCEPTED_PENDING` — optional non-terminal ownership of an asynchronous operation;
- `RETRYABLE_PENDING` — explicit non-terminal transient producer state that keeps the same operation identity;
- Platform-local `AMBIGUOUS` — response/commit state is unknown and must be reconciled by the same operation identity.

A terminal `REJECTED` ID remains terminal. A later deliberate user/business attempt may use a new `operation_id` only after current Platform gates are re-evaluated.

A producer-side `not found` reconciliation response does not automatically permit a fresh mutation identity unless the producer contract proves the original operation cannot later materialize.

### Command profiles

The contract defines shared profiles for:

1. create — game mints canonical CharacterId and duplicate replay cannot create a second character;
2. rename — game owns final name eligibility/uniqueness and result facts;
3. schedule deletion — game owns lifecycle transition/effective semantics;
4. cancel/restore deletion — current authoritative state wins over earlier UI/projection state;
5. finalize deletion — profile applies only if native lifecycle exposes explicit external finalization;
6. world/channel transfer — explicitly capability-gated; generic support does not approve the product;
7. account/Bazaar ownership transfer — subordinate to the commercial saga and preserves transfer-before-wallet-settlement ordering.

## Review findings and repairs

### P1 — Bazaar transfer/wallet ordering

Review found that the initial wording allowed an interpretation where wallet settlement might occur before authoritative game ownership was proven.

Repair:

```text
settlement-pending / funds reserved
  -> authoritative CharacterId ownership transfer
  -> game COMPLETED
  -> Platform wallet settlement
  -> commercial saga completed
```

Timeout after transfer submission keeps funds reserved and the saga pending/recovery-required. It does **not** execute wallet settlement. A later wallet timeout after game completion reconciles through wallet idempotency evidence and must not replay the game transfer.

This preserves the accepted `CHARACTER_TRANSFER_CONTRACT.md` invariant.

### P2 — retryable internal failure terminality

Review found that `retryable_internal_failure` appeared inside a terminal rejection taxonomy while idempotency required retries to reuse the same operation identity.

Repair:

- terminal `REJECTED` now means the producer guarantees the operation cannot later commit;
- transient retryable failures are explicitly non-terminal `RETRYABLE_PENDING`/pending/ambiguous semantics;
- retry/reconciliation reuses the same operation identity;
- a new operation identity is allowed only for a later new semantic attempt after a terminal rejection and fresh Platform gates.

This removes both infinite terminal-ID retry ambiguity and unsafe duplicate-operation minting.

## Important authority decisions

### Platform does not become Character Authority

Platform remains responsible for authentication/object authorization, security/business/product/entitlement gates, operation identity, saga/history, notification/audit and Platform-owned projections/preferences.

Oteryn-v2 remains responsible for current AccountId↔CharacterId ownership, CharacterId minting, lifecycle/game-state eligibility, mutation transaction/concurrency safety and authoritative result/receipt.

### No shared mutation SQL in the native target

Canary numeric identities and direct SQL remain explicit compatibility/migration details. They are not a shortcut implementation of the native command contract.

### Public projection remains separate

A successful mutation may update the Platform saga immediately from the authoritative result, but public search/profile/ranking/activity/guild/presence state still converges through `OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md` and its privacy-revocation fence.

## Concurrency model

The architecture does not require one global command queue or global ordering stream.

The future game authority may use transactions, locks, actor serialization, revisions, fencing or another mechanism, but it must deterministically resolve relevant races for the same character/ownership domain, including rename/delete/restore/transfer/Bazaar/session conflicts.

Platform may preclude obvious workflow conflicts but cannot resolve game-state races from stale projections.

## Security model

The semantic contract requires the future transport to prove service authentication, authorization/audience, replay controls, input/version validation and secret-safe observability. It intentionally does not select mTLS, token format, message signatures or another primitive.

`operation_id` provides idempotency/correlation semantics; it is not a credential or substitute for transport replay protection.

## Product impact

### #317 deletion/restore

The generic cross-system command/result dependency is defined at Platform architecture level. #317 still owns the actual deletion product lifecycle, exact native lifecycle profile and eventual runtime/browser implementation.

### #319 rename

The generic operation/idempotency/reconciliation dependency is defined. #319 still owns exact rename product behavior such as old-name policy, cooldown/fee behavior and user-facing implementation.

### #320 world transfer

Only reusable command machinery is defined. World transfer remains conditional on an explicit product/capability decision and accepted game-owned placement semantics.

### Bazaar/account ownership transfer

The common envelope may be reused, but ownership transfer remains higher-risk and subordinate to Bazaar/commercial invariants. Authoritative game transfer occurs before wallet settlement. No payment or ownership-transfer cutover is authorized.

## Deferred implementation decisions

Not selected by this task:

- transport, endpoint, IDL or serialization;
- exact Oteryn-v2 module/service names;
- exact operation ID encoding;
- game-internal locking/transaction model;
- durable receipt storage/retention;
- reconciliation query/event representation;
- name policy;
- deletion state machine and finalization mode;
- world-transfer product adoption/rules;
- Platform database tables/workers;
- staging or production rollout.

## Rollout principle

Cut over per command family. A native operation already submitted or ambiguous must be reconciled/fenced before any compatibility fallback for future operations. Rolling back by blindly executing the same business intent through Canary/direct SQL is forbidden because it can double-apply the mutation.

## Validation classification

This is architecture/documentation only.

- runtime/browser E2E: `NOT_APPLICABLE` — no executable producer, consumer, schema, endpoint, UI or product activation changes;
- required evidence: exact-head full-diff self-review, Agent Governance and repository-selected CI;
- no external repository mutation or production action is authorized.
