---
task_id: OTERYN-20261023-store-wire-1
governing_issue: 1658
required_reads:
  - docs/contracts/OTERYN_V2_STORE_PURCHASE_DELIVERY_CONTRACT.md
  - docs/contracts/OTERYN_V2_STORE_WIRE_CONTRACT.md
search_first:
  - docs/contracts/fixtures/store-wire-v1/manifest.json
  - docs/contracts/OTERYN_V2_GAME_RESTORE_RECONCILIATION_CONTRACT.md
  - app/ProductsEntitlements/Http/GuardPremiumSnapshotPeer.php
optional_reads:
  - docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md
  - app/CharacterAuthority/CharacterCommandWire.php
---

# OTERYN-20261023 Store wire contract and fixtures (STORE-WIRE-1)

## Goal

Governing GitHub Issue: #1658, the canonical lifecycle authority for this task. It is stacked on the semantic contract STORE-DELIVERY-CONTRACT-1 (PR #1625).

Propose `oteryn-v2-store-wire-v1`, the wire layer for the Store purchase and delivery path between Platform and Oteryn-Game, as documentation and fixtures only. It must fix:

- the closed, versioned Game→Platform routes;
- the dedicated mTLS Store identity and its guard;
- member sets, types and bounds;
- closed result codes;
- canonical encoding and digests;
- idempotency keys, paging and limits.

It covers eight messages: balance, history page, purchase, pending list, exact intent read, receipt, restore notice reuse and reconcile listing. Flags default off, and the rollout follows S-9. No PHP runtime, schema, configuration or Oteryn-Game change is in scope.

## Acceptance criteria

- [x] `docs/contracts/OTERYN_V2_STORE_WIRE_CONTRACT.md` is PROPOSED and fixes routes, identity and guard, members, codes, encoding, digests, idempotency, paging, limits, default-off flags and the S-9 rollout.
- [x] `docs/contracts/fixtures/store-wire-v1/` holds closed JSON Schemas, valid examples for all eight messages, schema-level and rule-level invalid cases, and a manifest with sizes and SHA-256 values.
- [x] An independent fixture check passes (schemas, canonical bytes, digests, member order, binding digests, cursors, worst cases within bounds).
- [x] Repository documentation validators pass.
- [x] Questions the owner has not decided are listed in contract §15 and not decided.
- [ ] Exact-head independent review accepts the contract (owner and reviewer; never the author).

## Ownership

```yaml
owned_paths:
  - docs/contracts/OTERYN_V2_STORE_WIRE_CONTRACT.md
  - docs/contracts/fixtures/store-wire-v1/**
  - docs/agents/tasks/active/OTERYN-20261023-store-wire-1.md
  - docs/agents/tasks/archive/OTERYN-20261023-store-wire-1.md
modules:
  - products-entitlements
dependencies:
  - STORE-DELIVERY-CONTRACT-1 (PR #1625, semantic contract, Proposed)
blockers:
  - Acceptance of the semantic contract (PR #1625) and an exact-head independent review of this contract
cross_repository_tasks:
  - Oteryn-Game Store consumer wire (not started; owned by the Game side)
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-23T12:30:00Z
head: df6cf9b669f42122b3a86511f025ae895d64cd61
branch: claude/happy-davinci-ijz3wy-store-wire-1
pr: 1659
status: validating
context_routes:
  - products-entitlements
owned_paths:
  - docs/contracts/OTERYN_V2_STORE_WIRE_CONTRACT.md
  - docs/contracts/fixtures/store-wire-v1/**
  - docs/agents/tasks/active/OTERYN-20261023-store-wire-1.md
  - docs/agents/tasks/archive/OTERYN-20261023-store-wire-1.md
proven:
  - The command exchange (CharacterCommandWire) fixes canonical JSON, uint64 strings, empty-body failures and the accepted receipt answer; the Store wire reuses them.
  - GuardPremiumSnapshotPeer fixes the 503/401/429 order, TLSv1.3, the 128-byte subject check with hash_equals and the hashed-identity limiter key.
  - The accepted restore contract covers deliveries in its A4 section, but its implementation only reopens character command receipts.
  - The Wallet holds one available/reserved balance per identity, with idempotency keys of at most 120 bytes.
derived:
  - Store intents reuse the restore notice unchanged; reopened and outstanding count both exchanges together, which needs a restore contract amendment before Store delivery is enabled.
  - Every fulfilment unit, Profile B included, gets a DELIVER intent so that semantic contract §6.1 holds literally.
  - The ITEM claim is a separate Platform-issued CLAIM intent after the committed DELIVER, which keeps one receipt per key.
unknown:
  - Transferable coins semantics (Q1), catalog_revision source (Q2), Premium time v1 fit (Q3), cosmetic authority read (Q4), Profile B intents (Q5), Platform PITR fence (Q6), identity split (Q7).
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - A Store-specific restore notice; the existing notice already foresees deliveries and a second notice would split the generation order.
  - Platform calling Game; S-2 decides Game pulls.
  - A TTL on Store intents; S-6 decides that a paid delivery never expires.
changed_paths:
  - docs/contracts/OTERYN_V2_STORE_WIRE_CONTRACT.md
  - docs/contracts/fixtures/store-wire-v1/**
  - docs/agents/tasks/active/OTERYN-20261023-store-wire-1.md
validation:
  - command: python3 check_store_wire.py docs/contracts/fixtures/store-wire-v1 (independent session checker)
    result: PASS
    evidence: 16 schemas, 30 valid, 23 schema-invalid, 9 rule-invalid
  - command: python3 tools/agents/documentation_ia.py; tools/validation/legacy_token_ratchet.py; tools/agents/checkpoint.py --tasks docs/agents/tasks/active --require-checkpoint
    result: PASS
    evidence: see PR validation table
blockers:
  - Acceptance of PR #1625 and exact-head independent review
next_action: Owner and independent reviewer review the draft PR; answer contract §15 questions.
terminal_pr_policy: archive_pending
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```
