# Shared-template portal qualification

Qualification is in progress. No complete four-project result is claimed yet.
The previous 200-case record belongs to the earlier shell and is superseded by
this task. Base: 26c767eec36c04b5227a9676ec827ec76c4e82e9, fetched from the requested
source branch. The suite is extended, with all original flows retained.

The source inventory contains 318 route rows and 48 registry sections. One
compatibility Bazaar item-action route is not mounted when the qualified Bazaar
runtime is selected; its absence must be asserted as 404. Current listing
mutations use the account Bazaar bid/control endpoints and the existing complete
Bazaar flows. All other rows require an executed passing case in every project.

## Defects found and repaired

| Root cause | Fix and owning regression |
| --- | --- |
| Phone CSS hides dropdowns even on focus; anchor triggers cannot reliably toggle on touch | Native details/summary disclosure, preserved focus-within/hover, bounded menu scrolling; UI disclosure regression plus real keyboard/touch activation |
| Desktop last-child alignment clips the Account dropdown in the phone grid | Reset odd phone columns to left alignment; UI CSS regression and keyboard/touch bounds checks for every group |
| Parent and child sections both claim the current page | Select only the most specific current section; UI current-section regression |
| Unprefixed aliases never match localized registry URLs | Compare canonical route suffixes; UI alias/current-section regression |
| Standalone pages and the injected shell each render a skip link | Adapter retains one shared skip link; Web composition regression |
| Standalone main landmarks lack a focus target or use a custom ID | Add focusability while preserving custom IDs and provide a shared panel target; Web landmark regressions plus actual keyboard activation |
| Language links invent localized aliases or unsupported language query parameters | Preserve explicit localized routes; use the existing language endpoint for unprefixed routes; UI route-shape regressions |
| Standalone CSPs omit local image/font sources needed by the shared shell | Adapter adds only absent same-origin asset directives and preserves explicit restrictions; Web policy regression |
| Account shortcuts and sudo completion target an unmounted security landing route | Restore the localized landing alias to the existing credentials page; owning binary language/redirect regression |
| Public party board exposes a staff-only queue link | Restrict the link to the moderation surface; Community template regression |
| Wiki revision source blocks scroll without keyboard access | Focusable pre elements; Wiki rendered-template regression and positive publication/history browser flow |
| OAuth consent no-referrer policy turns the native form Origin opaque | HTML uses strict-origin, keeping queries out of referrers and preserving the real POST origin; protocol JSON retains no-referrer; Gameauth policy regression and browser consent/PKCE flow |
| Journal mutations parse forms before checking CSRF and bypass the shared origin guard | Standard session/CSRF middleware precedes extraction, rendered fields use _csrf; PostgreSQL owner, missing-token and foreign-origin regression |
| Staff control refusal documents omit a title | Complete localized document title and h1; Admin response regression |

The launcher also omitted analytics switches whose names do not end in ENABLED;
the explicit local configuration now enables them and a local OAuth client. All
87 discovered environment switches ending ENABLED are on. See ENVIRONMENT.md.
The internal journal weekly_email gate remains false until an authoritative
JournalSource is wired; valid-token preference updates persist locally and
missing-token requests assert 403; delivery remains disabled. No production authority is fabricated.

The combined run also exposed a launcher authority mismatch: the continuing
closure producer published ownership watermarks under a different authority from
the privacy reader. The producer and all local ownership readers now share one
configured authority. The privacy flow asserts agreement with the live feed before
seeding its isolated read models. No production authority guard was changed.

The retained privacy flow also contained a stale landing-page assumption: the
world statistics selector is a healthy 200 page. It now asserts that page has no
character disclosure and submits a registered world/date query, retaining the
exact 503/no-store Game-data refusal. Privacy and world-read passed all 12 cases
across the four projects after these fixture corrections.

With the shared feed healthy, missing-character profile routes correctly refuse
with 404 rather than the old misconfigured-authority 503. Both localized and
unprefixed inventory expectations now require exactly 404. All eight alias cases
passed across the four projects. The completed Polish desktop diagnostic passed
435/437 cases; these two obsolete fixture expectations were its only failures.

## Evidence so far

- Focused UI, Web, Community, Wiki, Gameauth and Admin regressions pass.
- The journal PostgreSQL regression ran with the disposable database URL and
  passed, including valid owner writes and missing-token/foreign-origin refusal.
- Strict locked Clippy passes after the source fixes validated so far.
- A complete phone diagnostic ran 374 cases: 355 passed, 19 failed, no retries.
  Its failures identify the defects and fixture corrections above. It is not
  current-candidate qualification; those fixes require a fresh server and rerun.
- Final Rust acceptance, full browser matrix, coverage reconciliation and normal
  push remain pending. No pull request has been opened.

Raw traces, cookies, captured mail, keys and database URLs remain private in
scratch and ignored test artifacts. Only sanitized qualification is published.

## Live executed progress

Snapshot 2026-11-08T16:10:24Z; tested source `7bcfe31a6b53ac55115ef642ae37fa23aa0cee0a`.
1421 passed, 0 failed, 327 remaining, 0 skipped, 0 retries.

| Project | Passed | Remaining |
| --- | --- | --- |
| pl-1440 | 437 | 0 |
| pl-390 | 437 | 0 |
| en-1440 | 437 | 0 |
| en-390 | 110 | 327 |

All 318 route rows and all 48 registry sections have passed in at least one
project. Final four-project qualification remains pending. Source is frozen;
only the three evidence documents are updated during the active sweep. Formatting,
strict locked Clippy and three explicitly invoked Rust browser cases passed. Full
PostgreSQL workspace acceptance follows the sweep after private ports are freed.

Navigation milestone: all 48 registry destinations and the keyboard/touch
interaction case passed in each of the four projects. No serious/critical axe
violations, overflow failures, unexpected browser errors or retries occurred.
