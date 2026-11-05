# Account character creation

Routes: `/{pl|en}/account/characters/new` (GET, POST),
`/{pl|en}/account/characters/suggest` (GET), and
`/{pl|en}/account/characters/requests/{operation}` (GET).
The web router mounts these inside its existing browser-session and CSRF middleware.
The session adapter carries authentication, CSRF and one-use form feedback without a
reverse dependency on the web crate. Input errors redirect with HTTP 302 and preserve
input and the submission key in the session. Successful submissions also redirect
with HTTP 302. Malformed form bodies receive a localized validation response.

All three flags must explicitly equal `true`:

- `OTERYN_CHARACTER_CREATE_WEB_ENABLED`
- `OTERYN_CHARACTER_COMMANDS_ENABLED`
- `OTERYN_CHARACTER_COMMAND_CREATE_ENABLED`

`OTERYN_CHARACTER_COMMAND_SOURCE_AUTHORITY` must match the private command consumer.
No private listener, production configuration or feature activation is added here.

T22 already supplies the public durable command API. T44 is absent in the base and
has no dependency branch. The local eligibility seam uses the identity crate's public
account reader and holds the identity row lock across durable command submission.
Only active and pending-verification accounts may attempt creation; missing,
suspended, closed or unreadable account state denies. LCFA does not establish
eligibility or slot capacity. Game checks its own account, world, naming and quota
rules. World choices use the existing registry and display PvP/location metadata. With more
than one world, the form offers server-rendered PvP and location filters.

Naming matches the existing Laravel `CharacterNamePolicy`: ASCII-only input,
normalized case/spaces, 1–3 words, 2–15 letters per word, 3–29 characters total,
reserved staff tokens/phrases and protected product prefixes. The source policy's
retired product prefix is represented by bytes. `reserved_names` adds expiring,
case-insensitive Platform reservations; a deletion producer can set expiry to six
months. Suggestions avoid active Platform reservations but promise no availability.
Game remains the final name registry. Shared PostgreSQL buckets enforce the existing
5 creates/minute and 60 reads/minute budgets per authenticated account.

Status is owner-only and command-specific. A stored Game receipt decides created or
rejected; missing receipts stay pending, including after the intent TTL. No local
placeholder character or projection is written.

Governing references:

- `docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md`
- Laravel `CreateCharacter`, `CharacterNamePolicy`, `CharacterCreationController`,
  `NativeAccountOverviewReadModel` and `CharacterCreationOutcome`
- [Game wire contract, sections 4.2, 4.3, 6.1 and 7](https://github.com/Oteryn/Oteryn-Game/blob/857f15f376907c592bb28d854fccff18a15cdf87/docs/contracts/OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md)

The pinned wire explicitly excludes sex, outfit and profession inputs. The form
explains the standard starting appearance and profession instead of collecting choices
that would be discarded.
The module reuses existing T22 golden fixtures without copying them.

Focused tests: `cargo test --locked -p oteryn-characters create_web -- --nocapture`.
Five PostgreSQL tests use isolated schemas and read `PORTAL_TEST_DATABASE_URL`;
unset configuration prints a skip notice. They cover duplicate/concurrent submission,
changed-input conflict, unknown/ineligible accounts, bad worlds, expiring reservations,
shared limits, browser feedback, owner isolation and actual receipt publication.

Local qualification used the mounted public router with real browser sessions and
PostgreSQL 18.6. At 360 px, PL/EN pages had no horizontal overflow; labels, keyboard
focus, world filtering, reserved-name feedback, suggestions, creation, exact replay,
CSRF refusal and independent Origin refusal passed. The input contrast measured
16.24:1 in the light theme; the existing design-system tests validate both themes.
A repeated browser submission left exactly one command row.

The standard binary has a pre-existing startup collision: its redirect router also
mounts `GET /register`, already registered by the web router. It panics during router
composition before listening. Browser qualification therefore used a temporary local
harness of the same public router and registration configuration. Resolving that
unrelated collision is outside this task's allowed paths.
