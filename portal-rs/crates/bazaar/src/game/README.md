# Bazaar Game boundary

[PLAT3.md](PLAT3.md) documents the consumer of Game CANDIDATE rev 1 at
`3b49a20feec3ba29700bb4b2fa28fa9b539b79a2` and owner acceptance on 2026-11-06.
The portal and worker select real adapters with `OTERYN_BAZAAR_ENABLED=true`;
default-off composition keeps the existing unavailable response.

The private mTLS listener implements the six Game-initiated exchanges, extends
the single Character receipt facility and validates shared ANL envelopes.
Status, watermark, HTTP success, timeout and absence never prove eligibility,
custody or delivery. Parameterized SQL runs in caller-owned transactions;
adapters perform no remote I/O inside them.

## Owner defaults

Accepted proposals cover v2 receipts, obligations, snapshot survival, Inbox/item
classes, Gold's 250 bps ceil fee/headroom and common restore scope. Conservative
configurable choices are 30-day Character cooldown and 3650-day proof retention.
Escrow identity, history coverage, nonrollback fence and public signing keys must
be explicitly provisioned. See [the complete Owner defaults](PLAT3.md#owner-defaults)
and [runtime configuration](../../../../e2e/ENVIRONMENT.md).
