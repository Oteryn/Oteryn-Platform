# Progress integration seam

T77 implements the five G-11 progress families, canonical closed record decoding,
durable replay/conflict checks, contiguous baseline watermarks, owner-only reads,
tombstones, goals, conservative forecasts, and escaped Polish/English SSR functions.
The migration uses the task-reserved version. No gameplay commands are produced.

T65 exists in the starting base. T72 and its remote task branch were absent at
implementation time. Consequently no HTTP route is mounted and there is no concrete
production Authority or PrivatePace implementation. Both feature flags default to
false and the enabled-family list defaults to empty at the integration boundary.

Before enabling, T72 must supply dedicated mTLS/partition/rate/encoding guards,
source/catalog acceptance, current ownership and transfer-receipt cut revalidation,
restore and erasure fences, encrypted payload storage, bounded payload retention,
replay digest retention and cleanup, and session/CSRF middleware. HTTP adapters
must set Cache-Control: no-store for every response. Production activation requires
the proposal's separate acceptance. The module is not evidence of Game publication
support. It never falls back to public statistics as private ownership evidence.

The Authority seam must reject stale/unknown ownership, account deletion, catalog
revision mismatch and both parties during a transfer. It must suppress pre-transfer
character history for both parties; account grants remain account-scoped. The
PrivatePace seam must return observations for the exact goal metric and current
ownership cut, not public projections. No concrete pace provider is assumed.

Fail-closed interpretations: an unknown boss deadline stays unknown; elapsed time
is only an observation, not game eligibility. Quest tracks are named signed counts,
not stages. Character achievement counts never imply account grants. Generations
never change automatically; rebootstrap is an explicit dependency operation.
Incomplete baselines, gaps and stale evidence never mean empty. Skill counters are
tries or mana spent, never percentages. Forecasts with insufficient, flat, falling,
future or stale evidence have no completion date. G-11 revision 2 privacy and sale
rules take precedence over broader retention/history language in the design plan.

The SSR functions are available to the future authenticated router. Browser route,
CSRF and real actor-to-page E2E verification remain dependent on T72; no public
route or header-based identity substitute is provided. Goal editing is a service
seam, not a browser form. Database tests create and drop isolated schemas and skip
with a printed notice if PORTAL_TEST_DATABASE_URL is unset.

Workspace validation found an existing dual Rustls provider conflict: tough via
downloads enables aws-lc-rs, while gateway/SQLx enable ring. The database-backed
admission-listener test and gateway issuer tests then fail in Rustls automatic
provider selection. Those dependency declarations and gateway provider selection
are outside the T77 path allowance; this task does not modify them. This blocks
the requested all-green workspace acceptance even when T77 tests pass.
