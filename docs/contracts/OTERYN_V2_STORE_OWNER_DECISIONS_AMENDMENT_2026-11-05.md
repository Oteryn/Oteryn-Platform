# Oteryn-v2 Store owner decisions amendment (2026-11-05)

Status: `PROPOSED AMENDMENT — RECORDS OWNER DECISIONS — DEFAULT-OFF`
Amends: [`OTERYN_V2_STORE_WIRE_CONTRACT.md`](OTERYN_V2_STORE_WIRE_CONTRACT.md) §10, §11 and §15 (PR #1659), and [`OTERYN_V2_STORE_PURCHASE_DELIVERY_CONTRACT.md`](OTERYN_V2_STORE_PURCHASE_DELIVERY_CONTRACT.md) (PR #1625)
Governing issues: #1658 (STORE-WIRE-1), #1674 (STORE-IMPL-1)

## Purpose

This document records the owner's answers to the open questions in Store wire contract §15. The decisions were made on 2026-10-23 and 2026-11-05; the 2026-11-05 decisions replace the earlier answers to Q1, Q3 and Q4. The merged contracts are not edited in place. Where this amendment and a merged contract disagree, this amendment wins once it is accepted.

Everything else in both contracts stays in force. This amendment authorizes no deployment, real sale, payment, production mutation or Oteryn-Game change. The environment limit of wire contract §11 (`testing` and `preproduction` only) still applies.

## Decisions

| Question | Decision | Date |
|---|---|---|
| Q1 transferable coins | One coin type. It is transferable between players. | 2026-11-05 |
| Q2 `catalog_revision` source | SHA-256 of the canonical bytes of the Game-authored price-list file that Platform imports. One active import at a time. An explicit operator command activates it. Platform never edits offers. | 2026-10-23 |
| Q3 Premium time | `PREMIUM_TIME` is enabled. | 2026-11-05 |
| Q4 cosmetic authority read | `COSMETIC` stays disabled for now, so the cosmetic read is not needed yet. | 2026-11-05 |
| Q5 Profile-B intents | Confirmed: every fulfilment unit gets a `DELIVER` intent (wire §6.1). | 2026-10-23 |
| Q6 Platform PITR | A PITR/restore fence is a precondition for enabling purchases on any stack that can be restored. The environment limit stays. | 2026-10-23 |
| Q7 identity split | One Store mTLS identity in v1 (wire §4.1). | 2026-10-23 |
| Q8 account to Wallet mapping | No owner decision needed. The implementation confirms the mapping in code and records it in its task packet. | 2026-10-23 |

## Effect on the wire contract

### §10 Currency (replaces the v1 behaviour list)

- `COINS` is the only Store currency. It is the existing Wallet balance (available/reserved per identity).
- Coins are transferable between players. A transfer is a Platform ledger operation on the portal. It has its own ledger entries, idempotency key, row locking and limits, under the Wallet's existing transaction and locking rules. It needs its own implementation task and is not part of the Store wire.
- `TRANSFERABLE_COINS` stays a reserved wire value and is never used. An offer quoted in it still answers `OFFER_UNAVAILABLE`. No member changes.
- Game only spends coins through the Store purchase route. Starting a transfer from the game client would need a new Game command and a new contract. That is not in scope now.

### §11 Rollout step 2 (replaces the step text)

2. `purchase_enabled` with `enabled_unit_kinds = ["PREMIUM_TIME"]`, funded only by operator-granted coins. `COSMETIC` stays out of `enabled_unit_kinds` until the owner enables it and a cosmetic authority read (Q4) exists.

Game consumes the `DELIVER` intent for Premium time (Game task STORE-1).

### §15 Open questions

Q1, Q2, Q5, Q6, Q7 and Q8 are closed by the table above. Q4 is deferred together with `COSMETIC`.

Q3 is closed only as far as enabling `PREMIUM_TIME` goes. The fit with `oteryn.premium_time` v1 is still open, because v1 allows one entitlement per account, issues only through `OPERATOR_GRANT`, and removes all remaining time on revocation. Before rollout step 2, the Premium time implementation must either fit purchased time into v1 or propose a Premium time amendment, for example a purchase issuance source and a per-unit refund. This amendment does not decide which.

## Cross-repository impact

No change to the Store wire members, routes or fixtures. Oteryn-Game needs no wire change for these decisions.

## References

- `docs/contracts/OTERYN_V2_STORE_WIRE_CONTRACT.md`
- `docs/contracts/OTERYN_V2_STORE_PURCHASE_DELIVERY_CONTRACT.md`
- `docs/contracts/OTERYN_V2_PREMIUM_TIME_SNAPSHOT_CONTRACT.md`
