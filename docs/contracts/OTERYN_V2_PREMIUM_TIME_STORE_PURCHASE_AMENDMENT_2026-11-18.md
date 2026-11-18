# Premium time store purchases

Status: owner decision 2026-11-18. Amends the [premium snapshot contract](OTERYN_V2_PREMIUM_TIME_SNAPSHOT_CONTRACT.md) §1/§2.2 and the [2026-11-05 store owner decisions amendment](OTERYN_V2_STORE_OWNER_DECISIONS_AMENDMENT_2026-11-05.md).

Issuance sources in v1: OPERATOR_GRANT, REFERRAL and STORE_PURCHASE. A STORE_PURCHASE grant is applied by Platform in the purchase transaction for each PREMIUM_TIME fulfilment unit: duration_days = the unit's quantity (price-list days per unit x purchased quantity), 1-366, merged per §2.2 (additive while ACTIVE). Its request identity is the fulfilment unit's entitlement_id; a replay changes nothing. A grant that would exceed 366 days or the 3660-day cap refuses the whole purchase with no debit. Store refund of PREMIUM_TIME is not offered in v1; reversal is the §2.3 operator revocation. Game consumes the PREMIUM_TIME DELIVER as registration evidence only and derives the benefit from the premium snapshot.
