# Profile selections

`OTERYN_ACCOUNT_BADGES_ENABLED` is false unless its value is exactly `true`.
The browser route is `/pl/account/badges` or `/en/account/badges`, behind the
existing browser session, account status and synchronizer CSRF boundaries.
The account identifier always comes from the session.

T45 had no remote branch at implementation time. T47 was merged; its public
premium read API does not expose consumed loyalty days without issuing a
snapshot revision. Viewing these preferences must not issue such revisions.
`Evidence` is the narrow integration seam for T45 privacy/current ownership,
G-4 earned facts and complete level facts, and read-only consumed loyalty.
`LocalEvidence` exposes only the account's whole age in years. All other
sources remain unavailable, with no fabricated zero or inferred ownership.
No Game ingest, catalogue grant or cross-repository command is introduced.

An adapter must read and lock authoritative current ownership, visibility and
its durable ownership generation through the supplied transaction, in the
same lock order as privacy/ownership mutations. It must bind every character
to the requested account. LCFA names and a fresh family watermark alone do
not prove the G-4 ownership-transition revision floor. Earned facts remain
`None` until that floor, catalogue compatibility, source completeness and
accepted attribution are established. An authoritative empty earned set is
`Some(empty)`. Never populate it from public projections or client input.

Badge values represent whole years, consumed Premium days, the complete sum
of owned character levels, and the minimum level across every profession.
Missing profession coverage is unavailable. The plan defines no award tiers;
this implementation introduces none. The four stable badge keys are
`account-age`, `loyalty`, `total-levels`, and `all-professions`.

Profile and ranking consumers must use `Service::public` and
`Service::loyalty_character` at serving time, using a trusted owner resolver.
Raw preference rows are not permission to publish. The service rechecks
eligibility and the saved ownership generation. A hidden, transferred,
unknown or unavailable character suppresses the selection. Public output
contains only explicitly selected eligible badge values and up to five
currently earned keys, never earned times, achievement totals or account IDs.
The complete earned set stays in the authenticated owner view.

PostgreSQL tests create an isolated schema and read
`PORTAL_TEST_DATABASE_URL`; an unset variable prints a skip notice.
