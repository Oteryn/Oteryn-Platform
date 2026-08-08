---
task_id: OTERYN-20260808-native-character-command-contract
repository: blakinio/Oteryn-Platform
project_lane: oteryn-platform-core
issue: 919
status: completed
architecture_pr: 920
merge_sha: 1b6d0f2649b95a972d39b714c735e78c46076f9f
---

# OTERYN-20260808 native Character Authority command contract — closeout

## Terminal result

`DONE — NATIVE CHARACTER AUTHORITY COMMAND / RESULT ARCHITECTURE ACCEPTED ON MAIN`

PR #920 was squash-merged to protected `main` as `1b6d0f2649b95a972d39b714c735e78c46076f9f` after exact-head review, repair of all review findings and a fresh all-green validation generation.

## Accepted boundary

`docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md` now defines the reusable Platform-side semantic boundary for native Oteryn-v2 character mutations.

The accepted shared semantics include:

- canonical Platform `AccountId`, game-owned `CharacterId`, and canonical `WorldId`/`ChannelId` where applicable;
- one stable `operation_id` for one semantic mutation attempt;
- same-operation retries reuse the same identity;
- duplicate delivery cannot double-apply an authoritative mutation;
- conflicting reuse of one operation identity fails closed;
- Oteryn-v2 revalidates current ownership/lifecycle/game-state authority at execution time;
- Platform preconditions and projections remain optimistic evidence only;
- terminal `COMPLETED` and terminal `REJECTED` are distinct from non-terminal `ACCEPTED_PENDING` / `RETRYABLE_PENDING` and Platform-local `AMBIGUOUS`;
- timeout/response loss requires reconciliation by the same operation identity;
- no distributed ACID or one global command queue/order stream is assumed;
- native/legacy mutation fallback is fenced so an ambiguous native operation cannot be blindly replayed through Canary/direct SQL.

## Command profiles

The contract provides reusable semantics for:

1. create character;
2. rename character;
3. schedule deletion;
4. cancel/restore deletion;
5. explicit finalize deletion only when the native lifecycle exposes that external command;
6. world/channel transfer only if the product capability is separately adopted;
7. account/Bazaar ownership transfer as a higher-risk profile subordinate to the commercial saga.

World transfer is not product-approved by this architecture task. Issue #320 remains responsible for that decision and game placement semantics.

## Review findings repaired

The first all-green release candidate was deliberately not merged after PR review found two semantic defects.

### P1 — Bazaar ordering

The initial wording could be read as allowing wallet settlement before authoritative game ownership was proven.

Final invariant:

```text
funds remain reserved
  -> authoritative Oteryn-v2 CharacterId ownership transfer
  -> game COMPLETED
  -> Platform wallet settlement using existing idempotency evidence
  -> commercial saga completed
```

A timeout after game transfer submission keeps funds reserved and requires reconciliation. It does not execute wallet settlement. A wallet timeout after game completion cannot replay the game transfer.

This preserves `docs/contracts/CHARACTER_TRANSFER_CONTRACT.md`.

### P2 — retryable failure terminality

The initial taxonomy made `retryable_internal_failure` look like a terminal `REJECTED` while also implying retry of the same operation identity.

Final invariant:

- terminal `REJECTED` means the producer guarantees that operation identity cannot later commit;
- transient retryable failures remain non-terminal/reconcilable and retain the same operation identity;
- a later new operation identity is permitted only for a genuinely new semantic attempt after terminal rejection and fresh Platform gates.

Both review threads were resolved after verifying the repaired text.

## Downstream authority

The mutation channel is not a second public read-model authority.

Public/search/ranking/activity/guild/presence effects continue through `OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md` and its privacy-revocation fence. A successful command may advance Platform saga state from its authoritative result, but stale public projections cannot reverse a newer mutation or privacy deny.

Current Canary numeric identities/direct SQL remain `Legacy Canary Compatibility` only.

## Exact-head validation

Final PR #920 head: `a42def79245ea1aebabe1c935a133aa6ea3ff9b3`.

All selected workflows passed on that unchanged final head:

- Agent Governance — `31271276091`;
- Native protocol contract — `31271276097`;
- Native protocol contract audits — `31271276098`;
- Game Auth Ticket Concurrency — `31271276095`;
- Edge Security Emulation — `31271276089`;
- Platform DB Outage Validation — `31271276114`;
- CI — `31271276107`;
- Phase 7 Production-Like Validation — `31271276088`.

Final merge gate:

- branch `behind_by=0` against protected main;
- mergeable: true;
- changed paths: exactly four intended architecture/task/report paths;
- unresolved material findings: 0;
- unresolved review threads: 0;
- runtime/browser E2E: `NOT_APPLICABLE` because no executable producer, consumer, schema, endpoint, UI, product activation, deployment or external-repository behavior changed.

## Product issue handoff

- #317 deletion/restore can consume this shared operation/idempotency/result/reconciliation baseline but still owns its product lifecycle and runtime implementation.
- #319 rename can consume this shared baseline but still owns name/cooldown/history/public behavior and runtime implementation.
- #320 receives the reusable command profile only; world-transfer product adoption and authoritative placement semantics remain unresolved.
- #277 remains the parent character-management completeness issue.

## Deferred implementation authority

Still intentionally deferred:

- exact Oteryn-v2 service/module name;
- transport/endpoint/IDL/serialization;
- exact operation-ID encoding;
- durable result storage/retention and reconciliation API/event;
- game-internal transaction/locking/serialization implementation;
- exact rename/deletion/world-transfer product policies;
- Platform runtime/database worker implementation;
- staging/production rollout.

No Oteryn-v2, Canary, runtime, database, workflow, deployment, payment or production mutation occurred.

## Final checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-08T20:15:00+02:00
status: completed
phase: closeout
architecture_pr: 920
architecture_merge_sha: 1b6d0f2649b95a972d39b714c735e78c46076f9f
final_validated_head: a42def79245ea1aebabe1c935a133aa6ea3ff9b3
review_findings_repaired:
  - P1 Bazaar authoritative transfer before wallet settlement
  - P2 retryable failure terminality and operation identity
validation:
  - command: Agent Governance 31271276091
    result: PASS
  - command: Native protocol contract 31271276097
    result: PASS
  - command: Native protocol contract audits 31271276098
    result: PASS
  - command: Game Auth Ticket Concurrency 31271276095
    result: PASS
  - command: Edge Security Emulation 31271276089
    result: PASS
  - command: Platform DB Outage Validation 31271276114
    result: PASS
  - command: CI 31271276107
    result: PASS
  - command: Phase 7 Production-Like Validation 31271276088
    result: PASS
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: architecture/documentation-only task
blockers: []
next_action: none
```
