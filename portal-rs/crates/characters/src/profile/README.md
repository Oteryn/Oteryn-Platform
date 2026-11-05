# Character profile preferences

Implements `docs/contracts/CHARACTER_PROFILE_PREFERENCES_CONTRACT.md` for the
Rust identity schema: one identity per canonical `accounts.id`. Only main-character
selection and guild visibility are collected. Hiding, comments and the remaining
visibility toggles are outside this contract.

`OTERYN_CHARACTER_PROFILES_ENABLED=true` enables browser routes and consumers;
it is off when absent. LCFA must also be explicitly enabled and configured in
`testing` or `preprod`; production projection restrictions remain intact.

GET, POST and PUT `/account/characters/{name}/profile` are mounted with server-side
sessions and CSRF protection. `/pl` and `/en` variants select the language.
The account comes exclusively from the session. Missing/foreign characters return
404, stale or unreadable evidence returns sanitized 503, invalid form fields 422,
and successful writes redirect to the localized edit page.

`Service::edit` and `Service::update` repeat current ownership checks. Exact names
win; a case-insensitive match must be unique. A shared feed lock blocks every
publisher's exclusive feed lock through the preference transaction. Locked reads
apply LCFA freshness, epoch, source and account-state fences in the same transaction;
opening another projection transaction under that lock could deadlock behind a
queued publisher. Writes additionally lock the account row, clear previous main
selections (including unbound rows), and atomically record bounded audit events.
A partial unique index also enforces at most one main per account. Deadlock and
serialization failures during writes receive at most three attempts.

`Service::guild_visible(character, game_visible, now)` is the public privacy API:
Game's permission is always an upper bound. `public_preferences` returns `None`
for unknown/ambiguous/unavailable ownership; callers must deny on `None` and must
not turn a previous cached allow into a fallback. `main_character` returns only
a currently owned main, for private overview consumers. No game facts are written.
Existing overview templates are outside this task's allowed paths; consumers can
use these APIs to render settings links and main markers.

Unbound imported rows have a null `character_id`. An unbound false `show_guild`
remains an account-wide default denial until each character has its own saved row.
Saving one native character never deletes or rewrites other characters' opt-outs.
Deleting an account cascades preferences and their bounded audit records.

The PostgreSQL test uses an isolated schema, reads `PORTAL_TEST_DATABASE_URL`,
and prints a skip notice if unset. It covers success, foreign-account refusal,
invalid input, concurrent main selection, audit rollback, publisher exclusion,
legacy opt-outs, Game denial, disabled flags, stale/invalid/ambiguous evidence,
revoked ownership and account deletion.
