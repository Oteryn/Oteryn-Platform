# Bazaar Game boundary

The latest implementation and qualification record is [PLAT3.md](PLAT3.md).
It consumes the Game CANDIDATE revision 1 at exact commit
`3b49a20feec3ba29700bb4b2fa28fa9b539b79a2`, behind explicit default-off gates.

`Settings`/`Outbox` retain the older preparation-only behavior. Those flags do not
activate dispatch or make a staged command authoritative. `AuthorityConfig` governs
the separate receipt/audit/evidence and bounded private exchange adapters. Existing
portal wiring stays unavailable until qualified composition supplies real trust,
nonrollback recovery and owner acceptance evidence.

There is one Character receipt facility and no Bazaar receipt endpoint. Status,
watermark, intent issuance, HTTP success, timeout and absent records never prove
custody, delivery or eligibility. Every database operation uses parameterized SQL
in the caller's transaction; no adapter performs remote I/O inside it.

See PLAT3.md for delivered paths, exact fixture provenance, tests, and remaining
Character/gold/compensation/browser integration seams. This candidate is not a
production enablement or full PLAT3-BAZAAR completion claim.
