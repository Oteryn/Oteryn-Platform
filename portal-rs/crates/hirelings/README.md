# House helpers (T92)

Platform-only intents, with no game transport or local reward generation. Both
`Flags` fields default false. `PORTAL_HIRELINGS_ENABLED=true` exposes only the
read-only Polish/English account page at `/{lang}/panel/hirelings`. Runtime command
outbox remains disabled; there is deliberately no worker registration for it.

The store seam accepts only a committed entitlement verified by `Eligibility`,
including current active account, house ownership and profession. The default
adapter denies everything. T26 was not available as a dependency branch. Existing
T29 store delivery must own all charging and delivery; this crate never debits a
wallet or invents a purchase wire operation.

The destination supplied to `start` must come from a reviewed server catalogue,
not browser input. Every destination includes a complete reward table, duration
and account-wide weekly cap. Every destination must match the service account-wide
weekly cap; a missing or mismatched cap is refused. No destination or reward values are shipped: the pinned game reference
has no expedition contract. Requests snapshot the full table, serialize on the
account, reserve the weekly allowance and atomically insert one command/outbox
job. Retries must match the complete original request. Limits are conservative:
reservations are not automatically released, and an unconfirmed command remains
pending. There is no paid acceleration.

Only a trusted future G-14 receipt adapter can mark a return. Unknown receipts
are denied by default. Return notification and state transition share one
transaction; replay does not duplicate the notice. T36's optional News preference
controls this notice. Claim is another idempotent intent, never a local item grant.
Receipt transport, final claim acknowledgements, game-authoritative loot limits,
reviewed expedition catalogue and purchase/selection browser mutations remain
blocked on the missing contracts. The page explicitly communicates that boundary.

Read-only Game reference: Oteryn/Oteryn-Game at
857f15f376907c592bb28d854fccff18a15cdf87, notably
`apps/game-server/migrations/0720_store_hireling_delivery.sql`. No game fixtures
were copied. Platform contracts and Laravel routes contain no expedition API.

Validation: `PORTAL_TEST_DATABASE_URL` enables the PostgreSQL test; otherwise it
prints a skip notice. Its isolated schema uses the real outbox, notification and
hireling migrations and is removed after success. It exercises entitlement replay,
ownership, account limit, weekly limit, reward validation, default refusal,
command replay, return notification and claim replay.
