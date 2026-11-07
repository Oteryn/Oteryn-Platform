# Public character profiles

Routes: `/{pl,en}/characters` and `/{pl,en}/characters/{name}`. The
`PORTAL_PUBLIC_PROFILES_ENABLED=true` flag is off by default. The search form
redirects to an exact, case-insensitive current-name lookup. Invalid or oversized
input is 400, unknown/ambiguous/hidden is 404, and unavailable authority is 503.
Every rendered response is private and uncacheable. No internal subject or World
identifier is rendered.

The base reader uses T24's accepted snapshots, global epoch and 30-second
watermark policy (one-second uncertainty; no stale display). It resolves exactly
one World and refuses ambiguous name holders. The database test exercises real
HTTP handlers and PostgreSQL, including name handoff, epoch fencing and privacy
refusal. Set `PORTAL_TEST_DATABASE_URL` to run it; otherwise it prints a skip.

## Integration seams

T09c is merged. Its public `identity::privacy::Privacy::current` API is the account
upper bound, but does not prove ownership or character visibility. The configured `profiles::Privacy` implementation now combines current unique
LCFA ownership, saved T45 character/account-wide hiding and field switches with
current verified-account T09c preferences. The exact current World/name reference
must agree with LCFA. Missing/ambiguous/unreadable evidence denies; allows are
never cached. New characters remain hidden until the owner explicitly saves.
The public comment is limited to 500 Unicode characters and escaped by Askama.
Account association and account status have no representation in this reader;
there are no account identifiers, other-character links or account groupings.

`profiles::details::PublicDetails` is the read boundary for accepted G-4 guild,
house, skills and deaths. The existing T24 ingest accepts only its v1 families;
it does not yet supply the accepted G-4 extension. Production wiring deliberately
uses `MissingDetails`. An adapter must prove same-epoch, same-World, fresh,
current-subject evidence and the name-handoff barrier, and uphold the per-family
bounds before returning fields. T45 permissions independently narrow every
section. The injectable interface is exercised through the PostgreSQL/SSR test.
No separate ingestion protocol, shared schema changes or guessed ownership
mapping is introduced by this task.

Equipment, outfit and achievements are owner-only and have no public fields.
Historical killer names do not establish current identity and are omitted.
Online time remains unavailable until G-8 is accepted and separately activated;
no unaccepted activity family or fabricated zero is consumed. The 30-day deaths
view represents at most the latest 20 events, never a complete kill ledger.

References: the Platform public projection and profile preference contracts,
the G-4 extension with the owner's task notes, and read-only Game contract
`docs/contracts/OTERYN_GAME_PUBLIC_PROJECTIONS_V1.md` at
`857f15f376907c592bb28d854fccff18a15cdf87`. Test data is synthetic.
