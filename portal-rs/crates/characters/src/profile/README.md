# Platform character privacy (T45)

Platform owns hidden, one main character, a plain-text public comment (500 Unicode
characters), seven switches (account, status, guild, house, skills, deaths, kills),
and the D11 online-time switch. There is also an account-wide hide-all control.
New settings default hidden with every switch off; existing guild choices remain
stored, but require an explicitly saved visible character. Existing unbound legacy
guild opt-outs remain untouched. Missing saved native settings never grant visibility.
The additive migration is 20261107130001; its down migration restores the old schema.

`OTERYN_CHARACTER_PROFILES_ENABLED=true` enables owner browser routes and the
serve-time authority. LCFA's existing testing/preprod, source, epoch, freshness and
unique current ownership restrictions remain in effect. No production source,
fixture, ownership history or Game facts are invented.

GET/POST/PUT `/[pl|en/]account/characters/{name}/profile` use server-side sessions
and CSRF. Service methods repeat ownership checks and serialize account writes.
Foreign/missing characters return 404, invalid forms 422, unavailable evidence 503,
and a shared account budget of 30 successful writes/minute returns 429. The account
row lock, preferences, hide-all, privacy revision and bounded audit entry commit in
one transaction. Comment text never enters audit events. Askama escapes it in both
the editor and public profile. Duplicate or unknown form keys are refused.

`authority::Authority` returns current saved settings only for exactly one fresh
active owner with verified account privacy. Its `reference` API additionally binds
World/name to current LCFA evidence. Account-level association/status switches are
upper bounds; hiding applies to every section. Game eligibility independently
narrows these permissions. Errors and unknown/ambiguous/missing state deny; allows
are never cached. T09c fields are read in the same locked transaction, with its
verified-account predicate. Restrictive saves advance the account privacy revision.

The composed binary's configured community routers use this authority for public
profiles, online, deaths and guild members. Online resolves exact World/name only
through a fresh same-epoch Game profile and checks the LCFA reference. A profile
newer than the presence observation cannot authorize an older presence record.
Replacing/suppressing/fencing the presence snapshot during policy I/O refuses the
response. Counts include only currently visible characters with status permission.

Deaths/guilds/houses retain explicit production composition gates: coordinator
update 2026-11-07, Game main d34807f5bd234ee8318e57ba454ba39be0df113d,
OTERYN_GAME_PUBLIC_PROJECTIONS_V1.md rev 2, marks them CANDIDATE rev 1 without a
producer. Flags cannot remove these gates. Historical killer labels are withheld:
G-4 names alone prove neither stable identity nor killer privacy floors, and must
never resolve to a current name holder. Statistics retain their separate unaccepted
Game source gate. No public online-duration ranking is authorized.

Tests use `PORTAL_TEST_DATABASE_URL`, isolated schemas and synthetic identities.
They cover owner/foreign mutations, audit rollback, concurrent main selection,
rate limits, new switches, missing/ambiguous/stale state, account bounds, escaping,
and real authenticated Game push ingestion followed by the real online adapter.
