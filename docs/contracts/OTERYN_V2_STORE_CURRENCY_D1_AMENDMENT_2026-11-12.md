# Oteryn-v2 Store currency D1 amendment (2026-11-12)

Status: `PROPOSED AMENDMENT — RECORDS COORDINATOR DECISION — DEFAULT-OFF`
Amends: [`OTERYN_V2_STORE_OWNER_DECISIONS_AMENDMENT_2026-11-05.md`](OTERYN_V2_STORE_OWNER_DECISIONS_AMENDMENT_2026-11-05.md), §10 *Currency*, third bullet
Follows: [`OTERYN_V2_STORE_WIRE_CONTRACT.md`](OTERYN_V2_STORE_WIRE_CONTRACT.md) owner decision D1 (2026-11-02), as implemented in #1824 (Store wire) and #1829 (price list S-5)

## Purpose

The 2026-11-05 amendment says in §10 that `TRANSFERABLE_COINS` "stays a reserved wire value and is never used. An offer quoted in it still answers `OFFER_UNAVAILABLE`." Store wire decision D1 makes `COINS` the only currency on both the Store wire and the S-5 price list. The coordinator decided on 2026-11-12 to retire that sentence explicitly. The merged amendment is not edited in place; this document supersedes the one bullet.

## Decision

- The §10 bullet about `TRANSFERABLE_COINS` in the 2026-11-05 amendment is **void**. An offer quoted in `TRANSFERABLE_COINS` no longer answers `OFFER_UNAVAILABLE`.
- Store wire D1 governs instead: `currency` is `COINS` only. `TRANSFERABLE_COINS` is not a value of the Store wire or of the S-5 price list (`docs/contracts/fixtures/store-price-list-v1/`). A price list that contains any offer in another currency is an invalid document and is refused as a whole at import, and the `store_offers` table accepts only `COINS` (migration `20261112100004`).
- Every other bullet of the 2026-11-05 amendment, including the rest of §10 (coins are transferable between players through a separate Platform ledger operation, not through the Store wire), stays in force.

This amendment changes no Game wire member and authorizes no deployment, real sale, payment or production mutation.
