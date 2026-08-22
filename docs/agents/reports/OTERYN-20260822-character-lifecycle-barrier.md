# Character lifecycle barrier — 2026-08-22

## Result

`BARRIER_CLASSIFIED — SHARED PLATFORM SEMANTIC BASELINE ACCEPTED; NO NATIVE RUNTIME SLICE IS CURRENTLY SAFE TO IMPLEMENT`

Protected-main baseline: `8e609f05278816102a08fcbeb9d102642c8380a0`.

The generic cross-system Character Authority command/result semantics are no longer the shared blocker. Issue #919 / PR #920 already established `docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md`, including stable operation identity, idempotent retry, typed terminal/non-terminal outcomes, ambiguity/reconciliation and cross-command concurrency.

The remaining barriers are product-family-specific and stay fail-closed. Platform has enough accepted authority to know **how it must orchestrate** these mutations, but not enough producer/product authority to invent the game-owned lifecycle, name or placement semantics required to execute them.

No Oteryn-v2, Canary or other server/game repository was accessed by this task. Repository governance forbids that access in this invocation without separate explicit owner permission, so every game-owned detail not already accepted in Platform authority remains `UNKNOWN` rather than inferred.

## Barrier matrix

| Issue | Capability | Accepted on current Platform authority | Material information still missing | Runtime disposition |
|---|---|---|---|---|
| #317 | character deletion / grace / restore | Oteryn-v2 Character Authority owns current `AccountId <-> CharacterId`, lifecycle eligibility/state and authoritative schedule/cancel/restore/finalize result. Shared command semantics already define stable `operation_id`, duplicate/conflicting-reuse behavior, typed outcomes, ambiguity and reconciliation. Platform owns authenticated UX, Platform security/business gates, saga, notifications, audit and presentation. | Exact native lifecycle state machine; authoritative grace/effective/finalization facts; whether finalization is automatic or an explicit external command; exact producer command/IDL/transport mapping; durable receipt/reconciliation representation; producer behavior for lifecycle/session/concurrent-operation conflicts. | `BLOCKED` — no deletion/restore runtime or functional UI mutation should be implemented from Platform assumptions. |
| #319 | conflict-safe rename | Oteryn-v2 owns final name eligibility/uniqueness, current ownership/lifecycle/session eligibility and authoritative rename result. Shared command profile already requires canonical `AccountId`, `CharacterId`, stable operation identity, new-name intent and reconcilable result. Platform owns UX/security/business gates, orchestration/history and downstream reconciliation. | Exact producer command/IDL/transport mapping; authoritative implemented name-policy/result mapping; durable receipt/reconciliation representation; game-side concurrency/fencing behavior. Issue #319 also retains product-owned cooldown/fee/history/old-name behavior that must not be guessed. | `BLOCKED` — no native rename runtime should be implemented until the exact producer mapping and remaining product behavior are accepted. |
| #320 | player-selectable world/channel transfer | ADR 0029 defines Platform-owned canonical `WorldId` and scoped `ChannelId`; ADR 0030/0031 keep character placement and mutation game-owned. The shared command contract contains a capability-gated transfer profile with stable operation/result semantics. | A durable product decision that player-selectable native transfer is actually supported; accepted source/destination placement semantics; game-owned eligibility and treatment of houses/guilds/market/depot/inbox/quests/world-specific state and destination constraints; exact producer command/IDL/transport/receipt/reconciliation mapping. | `DECISION_REQUIRED + BLOCKED` — the generic transfer profile is not feature approval. No runtime work starts until the product decision and game-owned transfer contract exist. |

## Accepted responsibility boundary

### Platform owns

- authentication and server-side object authorization for the initiating Identity;
- resolution of the canonical Platform `AccountId` context;
- Platform-owned MFA, security, business, entitlement, cooldown and product gates where applicable;
- Platform-issued stable operation identity and semantic request fingerprint;
- user-facing saga state, notifications, privacy-safe audit and Platform-owned history/preferences;
- canonical `WorldId` and `ChannelId` topology policy/context when a separately approved transfer capability requires them;
- downstream orchestration into accepted public/account projection contracts.

### Oteryn-v2 Character Authority owns

- canonical game-owned `CharacterId`;
- current authoritative `AccountId <-> CharacterId` ownership;
- current character lifecycle and game-state eligibility;
- final name eligibility/uniqueness;
- authoritative logical world placement and transfer eligibility;
- mutation execution, transaction/concurrency safety and authoritative result/receipt;
- deterministic game-side resolution of races against gameplay sessions and sibling mutations.

Platform operation rows, cached projections, UI state, browser input and historical Canary rows are not authority for any of the above game-owned facts.

## Canonical identity rule

Native lifecycle commands use:

```text
Platform AccountId
Game-owned CharacterId
Platform WorldId + ChannelId only where topology context is actually applicable
```

`canary_account_id`, `canary_player_id`, numeric Canary world/channel values and direct/shared SQL remain `Legacy Canary Compatibility` only. They cannot be promoted into the native implementation merely because current compatibility code or schema already exists.

## Mutual exclusion and race policy

The shared accepted contract is already sufficient to establish the cross-operation safety rule:

1. Platform fails closed before submit when its own durable workflow state proves a mutually exclusive pending Bazaar/lifecycle/business operation.
2. Platform does not treat that pre-submit check as game authority and does not infer current game state from a stale projection.
3. Oteryn-v2 revalidates authoritative current ownership, lifecycle, placement and gameplay/session eligibility at execution time.
4. The game authority must deterministically resolve relevant races such as rename-vs-rename, rename-vs-delete, delete-vs-restore/finalize, transfer-vs-rename/delete/restore and Bazaar ownership-transfer-vs-other-character-mutation.
5. A timeout or lost response is `AMBIGUOUS` at Platform and is reconciled with the **same** operation identity. It is not permission to issue a new operation or fall back to direct Canary/native SQL.
6. Character Bazaar ownership transfer remains a separate higher-risk commercial saga. Authoritative ownership transfer completes before Platform wallet settlement; funds remain reserved while that game operation is pending or ambiguous.

The current `docs/contracts/CHARACTER_TRANSFER_CONTRACT.md` remains valid for its declared Character Bazaar / Canary compatibility scope only. It does not define native rename, deletion or player-selectable world-transfer semantics.

## What is executable now

### Documentation / routing

`PROVEN`: the Platform can safely persist this barrier classification and route the exact missing producer/product obligations to their existing owners.

### Platform runtime

`DERIVED`: there is no independent Platform-only runtime slice that can truthfully unblock #317, #319 or #320 today.

Reason:

- the generic orchestration envelope is already accepted;
- each remaining operation needs game-owned semantics or an exact producer mapping before a real command adapter can exist;
- #320 additionally needs an explicit product decision;
- adding Laravel operation tables, workers, endpoints or functional UI before those facts exist would encode guessed transport/state semantics or create an executable path with no accepted authoritative producer.

Non-functional mock scaffolding would not satisfy the product issues and would create misleading implementation state, so it is intentionally not created.

## No new architecture owner required

No duplicate ADR, shared command contract or architecture Issue is required from this barrier pass:

- ADR 0029 already owns canonical world/channel identity;
- ADR 0030 owns the Account Center / Characters native authority split;
- ADR 0031 owns the Native Oteryn-v2 versus Legacy Canary Compatibility boundary;
- Issue #919 / PR #920 already closed the reusable Character Authority command/result semantic gap;
- #317, #319 and #320 remain the product-specific owners for their respective lifecycle behavior.

The missing facts are therefore not a Platform architecture ambiguity that this agent may decide locally. They are external game-producer semantics and, for #320, an explicit product/capability decision.

## Evidence classification

### PROVEN

- protected `main` baseline is `8e609f05278816102a08fcbeb9d102642c8380a0`;
- #317, #319 and #320 are open and blocked;
- their current comments already acknowledge the terminal shared #919/#920 contract and preserve product-specific/runtime blockers;
- no other open PR references #317, #319 or #320 after excluding this task's PR #1226;
- branch search for `character-lifecycle` finds only this task branch;
- the active-work index exposes no overlapping character-lifecycle task owner;
- ADR 0029, ADR 0030, ADR 0031, the focused native lifecycle authority guide and the shared command contract agree on the authority split summarized above;
- no repository search result proves a later accepted player-selectable world/channel-transfer product decision.

### UNKNOWN

- exact Oteryn-v2 transport, endpoint, IDL and serialization for these character commands;
- exact durable result storage/retention and reconciliation query/event;
- exact game-internal transaction/locking/serialization implementation;
- exact deletion lifecycle and automatic-versus-explicit finalization model;
- exact implemented rename policy/result mapping;
- player-selectable world/channel-transfer product adoption;
- exact native transfer placement/eligibility semantics.

### CONFLICT

None in current accepted Platform authority. Historical Canary-oriented backlog text is lower-authority compatibility evidence and does not conflict once classified under ADR 0030/0031.

## Smallest unblock handoff

The next architecture/runtime gate is external to this Platform-only task:

> Accept an exact Oteryn-v2 Character Authority producer-side contract/mapping for deletion/restore and rename, including durable result/reconciliation behavior, and record an explicit product decision for whether native player-selectable world/channel transfer exists; if transfer is adopted, accept its game-owned placement/eligibility command/result semantics in the same authority chain.

After that evidence is durably available, rerun this barrier and select the **smallest individual product issue** whose producer/product dependencies are complete. Do not combine #317, #319 and #320 into one mega-implementation PR.

## Validation classification

This task changes documentation and task state only.

- application/runtime tests: `NOT_APPLICABLE` — no executable code, schema, endpoint, command adapter or UI changes;
- browser E2E: `NOT_APPLICABLE` — no user-facing executable lifecycle path is introduced;
- cross-repository integration: `NOT_APPLICABLE` — this bounded task deliberately delivers no producer/consumer implementation, and server/game repository access is outside current invocation authority;
- required final proof: full exact-head diff self-review plus repository-selected documentation/governance CI on the final PR head.
