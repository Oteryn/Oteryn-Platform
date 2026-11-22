# Monster collection

T90 implements private, nontransferable collection cards, up to five decks per
account with one to three distinct cards, and deterministic turn-based PvE.
No coins, gold, items, purchases, ratings, PvP or gameplay commands are involved.

All three task prerequisites were fetched explicitly. Package 8 and T77 required
merges; T42 was already an ancestor of main. Package integration kept both sides
of registration conflicts, removed duplicate dependency declarations and retained
main's newer journal-sessions contract. The lockfile was regenerated with cargo.

The existing contracts contain no collection HTTP wire, and Laravel has no
collection implementation. The plan §4, §5.10 and §6.1 govern this mechanic.
G-11 revision 4 defines the accepted bestiary wire. The catalogue definition
mapping is a candidate. The collection consumes Platform-held evidence.

## Source and activation boundary

Game's native creature shape was inspected read-only at
857f15f376907c592bb28d854fccff18a15cdf87 in
`content/creatures/definitions/creatures-00000-00499.json`. The native record uses
`definition.identity`, `definition.client_projection` and
`authoring.profile.details` (display name and bestiary metadata). No Game fixture
or asset is copied into this directory. All tests use synthetic records.

`Card::from_world` reads an allow-listed WorldStore entity. A trusted publication
caller must supply the accepted creature-to-G-11-race/revision/threshold and
creature-to-presentation-appearance mappings, plus a Polish label. It must not
infer these from names. T42's exact creature thumbnail is hash-verified, bounded,
matched to the world release commit and served from private database storage.
Publication is immutable and idempotent. A missing mapping is unavailable.

`OTERYN_COLLECTION_ENABLED` and `OTERYN_WORLD_ENABLED` accept only exact `true`;
both default off. Disabled collection mounts no routes. The separate journal
read flag controls the character selector and private source availability view.
Card unlocks still require a verified catalogue mapping and a configured progress
store.

Every private card/deck/battle action requires an active verified account.
Card unlocks are derived anew from complete fresh owner-only T77 progress with
exact race revision and threshold matching. Completion requires the final kill
threshold, not charm points or a public statistic. Unknown or stale state denies.
World retirement or visibility revocation prevents further battle reads/turns.
Forfeit only clears the authenticated account's own active slot, so a transfer or
source retirement cannot permanently consume its quota. Deck deletion detaches
battle history without changing its immutable combat snapshot.

## HTTP and battle rules

Browser routes use the shared session and CSRF middleware, a 4096-byte body limit,
private no-store responses, CSP and Polish/English Fluent text. Paths:

- `GET /{lang}/collection`: private character selector and source availability landing page.
- `GET /{lang}/collection/{character}`: private inventory and bestiary availability when journal reads are enabled.
- `POST /{lang}/collection/{character}/decks`: create/update a private deck.
- `POST /{lang}/collection/decks/{id}/delete`: delete the owner's deck.
- `POST /{lang}/collection/decks/{id}/battle`: start a mirrored PvE encounter.
- `GET|POST /{lang}/collection/battles/{id}`: view/submit a turn.
- `POST /{lang}/collection/battles/{id}/forfeit`: clear the owner's active battle.
- `GET /{lang}/collection/{character}/cards/{id}/image`: verified WebP.

Successful forms redirect with 303. Service refusals use 403, bad domain input
400, conflicts 409 and unavailable sources 503. Axum rejects malformed forms with
422 and malformed path identifiers with 400. Unsupported languages use 404;
anonymous users redirect to the localized login. Every error body is localized.

Ruleset 1 gives all cards 30 health, independent of real-game combat statistics.
Each turn the server resolves simultaneous actions against the first surviving
card. Strike deals seeded damage 6–10; guard deals 3 and reduces retaliation by 6.
Reserves enter on the next round. After 100 rounds, remaining ties are draws.
The opponent mirrors the deck and always strikes. The server generates and stores
the seed; it never sends it to the browser. Ruleset and roster are persisted.
Same-turn same-action retries return the stored state; different/stale submissions
conflict. Row locks serialize concurrent turns. One active battle per account and
30 starts per hour bound creation. There are no rewards or tradable results.

PostgreSQL tests use PORTAL_TEST_DATABASE_URL, create isolated schemas and print
a skip notice when unset. Tests exercise success, ownership/visibility/freshness
refusals, duplicate/locked cards, stale turns, concurrent retries, malformed forms,
CSRF, shared authenticated sessions, and both localized rendered pages.

Mutation adapters must implement `Authority::lock_mutation` and retain their
ownership, catalog, transfer, restore and erasure fences in the supplied database
transaction until commit. Its default refuses mutation. After the account lock,
deck saves and battle starts/turns reread unlock evidence under the progress
stream lock and retain world publication/visibility locks. No second pool
connection is acquired for these mutation checks. Publication uses the same
world fence. Browser operations have a three-second deadline, 250 ms transaction
pool admission, 500 ms local lock waits and two-second statement limits; dropping
a cancelled operation rolls back its transaction.

### Private journal evidence

`OTERYN_COLLECTION_JOURNAL_READ_ENABLED=true` enables the owner-only evidence
view when `OTERYN_COLLECTION_ENABLED=true`. It uses the configured T72 G-10 and
G-11 source authorities and displays the time of the completed inventory
snapshot at the current custody cut and the account-wide bestiary watermark.
Both sources display unavailable while the catalogue mapping remains a
candidate. Missing evidence has an unknown update time. The read flag defaults
to off and the view performs only local PostgreSQL reads.
